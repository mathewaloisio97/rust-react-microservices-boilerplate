//! HTTP gateway routing handlers for user email resource management.
//!
//! Exposes the REST API surface to manage active email configurations, initiate
//! lifecycle state changes protected by captcha telemetry validation, and submit
//! challenge code verification attempts.

use crate::{
    dtos::{
        EmailStateResponse, SetEmailPayload, SetEmailResponse, VerifyEmailPayload,
        VerifyEmailResponse,
    },
    error::handle_grpc_error,
    middleware::UserId,
    state::AppState,
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use your_app_contracts::email::v1::{GetEmailRequest, SetEmailRequest, VerifyEmailRequest};
use your_app_contracts::identity::v1::{
    ActivateUserRequest, GetUserRequest, UpdateLocalEmailRequest, UserStatus,
};

/// Retrieves the active and pending email state for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/email",
    tag = "Email",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current email state returned", body = EmailStateResponse),
        (status = 401, description = "Unauthorized - Missing or invalid bearer token"),
        (status = 500, description = "Internal server error or upstream service error")
    )
)]
#[tracing::instrument(skip(state))]
pub async fn get_email(
    State(mut state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    // 1. Retrieve the core identity status and credential provider type from the Identity service.
    let user_req = tonic::Request::new(GetUserRequest {
        user_id: user_id.0.clone(),
    });

    let mut id_client = state.identity_client.clone();
    let (is_active, can_change_email, provider) = match id_client.get_user(user_req).await {
        Ok(r) => {
            let inner = r.into_inner();
            let is_local = inner.credential_provider == "local";
            (
                inner.status == UserStatus::Active as i32,
                is_local,
                inner.credential_provider,
            )
        }
        Err(_) => (false, false, "sso".to_string()),
    };

    let req = tonic::Request::new(GetEmailRequest {
        user_id: user_id.0.clone(),
    });

    match state.email_client.get_email(req).await {
        Ok(res) => {
            let inner = res.into_inner();

            // 2. If the user identity is ACTIVE, they either verified a local email or logged in via OAuth.
            // If they are OAuth, the email service has no record initially (so current_email is empty and is_verified = false).
            // We implicitly treat these ACTIVE users as verified so the frontend doesn't prompt them unnecessarily.
            let implicitly_verified =
                inner.is_verified || (is_active && inner.current_email.is_empty());
            (
                StatusCode::OK,
                Json(EmailStateResponse {
                    user_id: user_id.0,
                    current_email: inner.current_email,
                    is_verified: implicitly_verified,
                    pending_new_email: inner.pending_new_email,
                    verification_type: inner.verification_type,
                    can_change_email,
                    provider,
                }),
            )
                .into_response()
        }
        Err(err) => handle_grpc_error(err),
    }
}

/// Initiates an email change request or updates a pending email destination.
#[utoipa::path(
    post,
    path = "/api/v1/email",
    tag = "Email",
    security(("bearer_auth" = [])),
    request_body = SetEmailPayload,
    params(
        ("x-captcha-voucher" = String, Header, description = "Cryptographic proof-of-humanity voucher")
    ),
    responses(
        (status = 200, description = "Email transition initiated", body = SetEmailResponse),
        (status = 400, description = "Invalid payload (e.g., malformed email address)"),
        (status = 401, description = "Unauthorized - Missing or invalid bearer token"),
        (status = 403, description = "Missing or invalid captcha voucher"),
        (status = 500, description = "Internal server error or upstream service error")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn set_email(
    State(mut state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(payload): Json<SetEmailPayload>,
) -> impl IntoResponse {
    if !payload.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid email format"})),
        )
            .into_response();
    }

    // STRICT BLOCK: Prevent SSO users from manually altering their email context.
    let user_req = tonic::Request::new(GetUserRequest {
        user_id: user_id.0.clone(),
    });
    let mut id_client = state.identity_client.clone();

    if let Ok(res) = id_client.get_user(user_req).await {
        if res.into_inner().credential_provider != "local" {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "SSO accounts are strictly bound to their identity provider and cannot manually alter their email address."})),
            ).into_response();
        }
    } else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to verify identity constraints"})),
        )
            .into_response();
    }

    let req = tonic::Request::new(SetEmailRequest {
        user_id: user_id.0,
        new_email: payload.email,
    });

    match state.email_client.set_email(req).await {
        Ok(res) => (
            StatusCode::OK,
            Json(SetEmailResponse {
                status: res.into_inner().status,
            }),
        )
            .into_response(),
        Err(err) => handle_grpc_error(err),
    }
}

/// Evaluates a public verification token against the email subsystem.
///
/// Because this endpoint does not require prior authentication, it expects the user ID
/// and target email address directly within the JSON payload. Upon successful confirmation,
/// synchronizes the finalized email update and active state back to the core identity microservice.
#[utoipa::path(
    post,
    path = "/api/v1/email/verify",
    tag = "Email",
    request_body = VerifyEmailPayload,
    responses(
        (status = 200, description = "Verification attempt evaluated successfully", body = VerifyEmailResponse),
        (status = 400, description = "Invalid payload, empty fields, or expired verification code"),
        (status = 500, description = "Internal server error or upstream service sync error")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn verify_email(
    State(mut state): State<AppState>,
    Json(payload): Json<VerifyEmailPayload>,
) -> impl IntoResponse {
    if !payload.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Email, User ID, and verification code cannot be empty"})),
        )
            .into_response();
    }

    let req = tonic::Request::new(VerifyEmailRequest {
        user_id: payload.user_id.clone(),
        email: payload.email,
        code: payload.code,
    });

    match state.email_client.verify_email(req).await {
        Ok(res) => {
            let inner = res.into_inner();
            if inner.success {
                // Instantly activate the user identity (Pending -> Active State Machine)
                let activate_req = tonic::Request::new(ActivateUserRequest {
                    user_id: payload.user_id.clone(),
                });

                if let Err(e) = state.identity_client.activate_user(activate_req).await {
                    tracing::error!("Failed to activate user post-verification: {:?}", e);
                }

                // Synchronize finalized email changes back to the local credential identity subsystem.
                if !inner.email_updated_to.is_empty() {
                    let update_req = tonic::Request::new(UpdateLocalEmailRequest {
                        user_id: payload.user_id.clone(),
                        new_email: inner.email_updated_to,
                    });

                    if let Err(e) = state.identity_client.update_local_email(update_req).await {
                        tracing::error!(
                            "Failed to sync updated email to identity subsystem: {:?}",
                            e
                        );
                    }
                }

                (StatusCode::OK, Json(VerifyEmailResponse { success: true })).into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    Json(
                        json!({"error": "Invalid or expired verification code", "success": false}),
                    ),
                )
                    .into_response()
            }
        }
        Err(err) => handle_grpc_error(err),
    }
}
