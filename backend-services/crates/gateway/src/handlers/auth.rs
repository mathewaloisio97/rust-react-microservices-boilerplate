//! HTTP route handlers for session management.
//!
//! Provides the external API gateway endpoints for modifying active user
//! sessions, including authentication termination (logout).

use crate::{error::handle_grpc_error, middleware::SessionToken, state::AppState};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use cleard_contracts::auth::v1::RevokeTokenRequest;
use serde_json::json;

/// Terminate the active user session.
///
/// Extracts the verified token from request extensions and calls the downstream
/// Auth microservice to invalidate the session globally. Successful revocations
/// short-circuit any subsequent authorization checks using this token.
#[utoipa::path(
    post,
    path = "/api/v1/logout",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Successfully logged out"),
        (status = 401, description = "Unauthorized - Missing or invalid token")
    )
)]
#[tracing::instrument(skip(state, token))]
pub async fn logout(
    State(mut state): State<AppState>,
    Extension(token): Extension<SessionToken>,
) -> impl IntoResponse {
    let req = tonic::Request::new(RevokeTokenRequest { token: token.0 });

    match state.auth_client.revoke_token(req).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({"message": "Session successfully terminated"})),
        )
            .into_response(),
        Err(err) => {
            // Internal gRPC network errors or upstream database faults are handled
            // and mapped consistently by the global gateway error mapper.
            handle_grpc_error(err)
        }
    }
}
