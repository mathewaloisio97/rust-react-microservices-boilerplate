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
use your_app_contracts::identity::v1::UpdateLocalEmailRequest;

/// Retrieves the current email resource state for an authenticated user.
///
/// Queries the email microservice using the authenticated user's ID extracted from the bearer token,
/// returning configuration status, verification flags, and any pending update details.
///
/// # Arguments
///
/// * `state` - Shared application state container holding the email service gRPC client.
/// * `user_id` - Authenticated user identifier extracted via middleware extension.
///
/// # Returns
///
/// An Axum response containing `200 OK` with the `EmailStateResponse` data or `401 Unauthorized`.
#[utoipa::path(
    get,
    path = "/api/v1/email",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current email state returned", body = EmailStateResponse),
        (status = 401, description = "Unauthorized - Missing or invalid token")
    )
)]
#[tracing::instrument(skip(state))]
pub async fn get_email(
    State(mut state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> impl IntoResponse {
    let req = tonic::Request::new(GetEmailRequest { user_id: user_id.0 });

    match state.email_client.get_email(req).await {
        Ok(res) => {
            let inner = res.into_inner();
            (
                StatusCode::OK,
                Json(EmailStateResponse {
                    current_email: inner.current_email,
                    is_verified: inner.is_verified,
                    pending_new_email: inner.pending_new_email,
                    verification_type: inner.verification_type,
                }),
            )
                .into_response()
        }
        Err(err) => handle_grpc_error(err),
    }
}

/// Initiates an email resource transition or registration update for an authenticated user.
///
/// Validates the inbound email payload format, submits the change request down-funnel to the
/// email service gRPC client, and triggers a challenge dispatch workflow. Requires a valid proof-of-humanity
/// header (`x-captcha-voucher`) evaluated by security middleware.
///
/// # Arguments
///
/// * `state` - Shared application state container holding the email service client.
/// * `user_id` - Authenticated user identifier extracted via middleware extension.
/// * `payload` - JSON body containing the target `email` address.
///
/// # Returns
///
/// An Axum response containing `200 OK` with `SetEmailResponse`, `400 Bad Request` on malformed inputs,
/// `401 Unauthorized`, or `403 Forbidden` if the captcha voucher is missing.
#[utoipa::path(
    post,
    path = "/api/v1/email",
    security(("bearer_auth" = [])),
    request_body = SetEmailPayload,
    params(
        ("x-captcha-voucher" = String, Header, description = "Cryptographic proof-of-humanity voucher")
    ),
    responses(
        (status = 200, description = "Email transition initiated", body = SetEmailResponse),
        (status = 400, description = "Invalid payload (e.g., malformed email)"),
        (status = 401, description = "Unauthorized - Missing or invalid token"),
        (status = 403, description = "Missing or invalid captcha voucher")
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

/// Submits a challenge confirmation code to verify a pending email transition.
///
/// Evaluates the verification code against the email subsystem. Upon successful confirmation,
/// synchronizes the finalized email update back to the core identity microservice.
///
/// # Arguments
///
/// * `state` - Shared application state container holding email and identity service clients.
/// * `user_id` - Authenticated user identifier extracted via middleware extension.
/// * `payload` - JSON body containing the submitted challenge `code`.
///
/// # Returns
///
/// An Axum response containing `200 OK` with `VerifyEmailResponse` on success, `400 Bad Request`
/// for invalid or expired challenge codes, or `401 Unauthorized`.
#[utoipa::path(
    post,
    path = "/api/v1/email/verify",
    security(("bearer_auth" = [])),
    request_body = VerifyEmailPayload,
    responses(
        (status = 200, description = "Verification attempt evaluated", body = VerifyEmailResponse),
        (status = 400, description = "Invalid code payload or expired code"),
        (status = 401, description = "Unauthorized - Missing or invalid token")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn verify_email(
    State(mut state): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(payload): Json<VerifyEmailPayload>,
) -> impl IntoResponse {
    if !payload.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Verification code cannot be empty"})),
        )
            .into_response();
    }

    let req = tonic::Request::new(VerifyEmailRequest {
        user_id: user_id.0.clone(),
        code: payload.code,
    });

    match state.email_client.verify_email(req).await {
        Ok(res) => {
            let inner = res.into_inner();
            if inner.success {
                // Synchronize finalized email changes back to the identity subsystem.
                if !inner.email_updated_to.is_empty() {
                    let update_req = tonic::Request::new(UpdateLocalEmailRequest {
                        user_id: user_id.0.clone(),
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
