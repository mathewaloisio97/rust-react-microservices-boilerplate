//! Identity Domain Route Handlers.
//!
//! This module implements the public HTTP endpoints for user onboarding and
//! authentication. It handles the mapping between inbound REST JSON payloads
//! and the internal gRPC contracts required by the Identity, Auth, and Email microservices.

use crate::dtos::{
    AuthResponseDto, LocalLoginPayload, LocalRegisterPayload, OAuthLoginPayload,
    RegisterResponseDto,
};
use crate::{error::handle_grpc_error, state::AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use your_app_contracts::auth::v1::CreateTokenRequest;
use your_app_contracts::email::v1::SetEmailRequest;
use your_app_contracts::identity::v1::{
    LoginLocalRequest, OAuthLoginRequest, RegisterLocalRequest,
};

/// Registers a new user account locally via email and password.
///
/// Validates the inbound payload, submits a registration request to the Identity gRPC service,
/// triggers an asynchronous dispatch to bind and verify the initial user email address, and
/// returns the newly assigned canonical user identifier. This endpoint requires an active
/// proof-of-humanity header (`x-captcha-voucher`) evaluated by upstream middleware.
///
/// # Arguments
///
/// * `state` - Shared application state container holding identity and email service clients.
/// * `payload` - JSON body containing the `email` and plaintext `password`.
///
/// # Returns
///
/// An Axum response containing `201 Created` with the `RegisterResponseDto`, `400 Bad Request` on
/// invalid formats, `403 Forbidden` if the captcha voucher is missing, or `409 Conflict` if the email exists.
#[utoipa::path(
    post,
    path = "/api/v1/register",
    request_body = LocalRegisterPayload,
    params(
        ("x-captcha-voucher" = String, Header, description = "Cryptographic proof-of-humanity voucher")
    ),
    responses(
        (status = 201, description = "User registered successfully", body = RegisterResponseDto),
        (status = 400, description = "Invalid payload (e.g., missing fields)"),
        (status = 403, description = "Missing or invalid captcha voucher"),
        (status = 409, description = "Email address already in use")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn local_register(
    State(mut state): State<AppState>,
    Json(payload): Json<LocalRegisterPayload>,
) -> impl IntoResponse {
    if !payload.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Email and password cannot be empty" })),
        )
            .into_response();
    }

    let req = tonic::Request::new(RegisterLocalRequest {
        email: payload.email.clone(),
        password: payload.password,
    });

    match state.identity_client.register_local(req).await {
        Ok(res) => {
            let user_id = res.into_inner().user_id;

            // Orchestrate immediate email verification dispatch to begin the lifecycle.
            let email_req = tonic::Request::new(SetEmailRequest {
                user_id: user_id.clone(),
                new_email: payload.email,
            });

            if let Err(e) = state.email_client.set_email(email_req).await {
                tracing::error!("Failed to dispatch initial verification email: {:?}", e);
            }

            (StatusCode::CREATED, Json(RegisterResponseDto { user_id })).into_response()
        }
        Err(err) => handle_grpc_error(err),
    }
}

/// Authenticates a user using traditional local email and password credentials.
///
/// Verifies credentials against the Identity microservice. Upon successful validation,
/// requests an active session token from the Auth microservice. This endpoint requires
/// an active proof-of-humanity header (`x-captcha-voucher`) evaluated by upstream middleware.
///
/// # Arguments
///
/// * `state` - Shared application state container holding identity and auth service clients.
/// * `payload` - JSON body containing the `email` and `password`.
///
/// # Returns
///
/// An Axum response containing `200 OK` with an `AuthResponseDto` session token, `400 Bad Request` on
/// blank inputs, `401 Unauthorized` if authentication fails, or `403 Forbidden` if the captcha voucher is missing.
#[utoipa::path(
    post,
    path = "/api/v1/login",
    request_body = LocalLoginPayload,
    params(
        ("x-captcha-voucher" = String, Header, description = "Cryptographic proof-of-humanity voucher")
    ),
    responses(
        (status = 200, description = "Successfully authenticated", body = AuthResponseDto),
        (status = 400, description = "Invalid payload (e.g., missing fields)"),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Missing or invalid captcha voucher")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn local_login(
    State(mut state): State<AppState>,
    Json(payload): Json<LocalLoginPayload>,
) -> impl IntoResponse {
    if !payload.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Email and password cannot be empty" })),
        )
            .into_response();
    }

    let req = tonic::Request::new(LoginLocalRequest {
        email: payload.email,
        password: payload.password,
    });

    let res = match state.identity_client.login_local(req).await {
        Ok(r) => r.into_inner(),
        Err(err) => return handle_grpc_error(err),
    };

    if !res.valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid email or password" })),
        )
            .into_response();
    }

    let auth_req = tonic::Request::new(CreateTokenRequest {
        user_id: res.user_id,
    });

    match state.auth_client.create_token(auth_req).await {
        Ok(auth_res) => (
            StatusCode::OK,
            Json(AuthResponseDto {
                token: auth_res.into_inner().token,
            }),
        )
            .into_response(),
        Err(err) => handle_grpc_error(err),
    }
}

/// Authenticates or provisions a user via an external OAuth provider (e.g., Google, Microsoft).
///
/// Validates the external identity token via the Identity service, and upon success,
/// issues an active session token via the Auth microservice. This endpoint requires
/// an active proof-of-humanity header (`x-captcha-voucher`) evaluated by upstream middleware.
///
/// # Arguments
///
/// * `state` - Shared application state container holding identity and auth service clients.
/// * `payload` - JSON body containing the external `provider` name and client `id_token`.
///
/// # Returns
///
/// An Axum response containing `200 OK` with an `AuthResponseDto` session token, `400 Bad Request` on
/// blank fields, `401 Unauthorized` if token validation fails, or `403 Forbidden` if the captcha voucher is missing.
#[utoipa::path(
    post,
    path = "/api/v1/oauth",
    request_body = OAuthLoginPayload,
    params(
        ("x-captcha-voucher" = String, Header, description = "Cryptographic proof-of-humanity voucher")
    ),
    responses(
        (status = 200, description = "Successfully authenticated via OAuth", body = AuthResponseDto),
        (status = 400, description = "Invalid payload (e.g., missing fields)"),
        (status = 401, description = "Invalid OAuth identity"),
        (status = 403, description = "Missing or invalid captcha voucher")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn oauth_login(
    State(mut state): State<AppState>,
    Json(payload): Json<OAuthLoginPayload>,
) -> impl IntoResponse {
    if !payload.is_valid() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Provider and id_token cannot be empty" })),
        )
            .into_response();
    }

    let req = tonic::Request::new(OAuthLoginRequest {
        provider: payload.provider,
        id_token: payload.id_token,
    });

    let res = match state.identity_client.o_auth_login(req).await {
        Ok(r) => r.into_inner(),
        Err(err) => return handle_grpc_error(err),
    };

    if !res.valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid OAuth identity" })),
        )
            .into_response();
    }

    let auth_req = tonic::Request::new(CreateTokenRequest {
        user_id: res.user_id,
    });

    match state.auth_client.create_token(auth_req).await {
        Ok(auth_res) => (
            StatusCode::OK,
            Json(AuthResponseDto {
                token: auth_res.into_inner().token,
            }),
        )
            .into_response(),
        Err(err) => handle_grpc_error(err),
    }
}
