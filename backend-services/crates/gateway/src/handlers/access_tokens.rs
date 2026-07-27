//! HTTP gateway routing handlers for stateless access token management.
//!
//! Exposes the REST API surface to convert stateful session credentials into
//! cryptographically signed JWTs and retrieve the public key for validation.

use crate::{
    dtos::{GetPublicKeyResponse, IssueTokenPayload, IssueTokenResponse},
    error::handle_grpc_error,
    middleware::SessionToken,
    state::AppState,
};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use your_app_contracts::access_tokens::v1::{GetPublicKeyRequest, IssueTokenRequest};

#[utoipa::path(
    post,
    path = "/api/v1/access-tokens",
    security(("bearer_auth" = [])),
    request_body = IssueTokenPayload,
    responses((status = 200, description = "JWT minted", body = IssueTokenResponse))
)]
#[tracing::instrument(skip(state, token))]
pub async fn issue_token(
    State(mut state): State<AppState>,
    Extension(token): Extension<SessionToken>,
    Json(payload): Json<IssueTokenPayload>,
) -> impl IntoResponse {
    let req = tonic::Request::new(IssueTokenRequest {
        session_token: token.0,
        roles: payload.roles,
        ttl_seconds: payload.ttl_seconds,
    });

    // Forward the token issuance request to the underlying gRPC microservice.
    match state.access_tokens_client.issue_token(req).await {
        Ok(res) => {
            let inner = res.into_inner();
            (
                StatusCode::OK,
                Json(IssueTokenResponse {
                    access_token: inner.access_token,
                    expires_at: inner.expires_at,
                }),
            )
                .into_response()
        }
        Err(err) => handle_grpc_error(err),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/access-tokens/key",
    responses((status = 200, description = "Public Key retrieved", body = GetPublicKeyResponse))
)]
#[tracing::instrument(skip(state))]
pub async fn get_public_key(State(mut state): State<AppState>) -> impl IntoResponse {
    let req = tonic::Request::new(GetPublicKeyRequest {});

    // Retrieve the public PEM key for offline signature verification.
    match state.access_tokens_client.get_public_key(req).await {
        Ok(res) => (
            StatusCode::OK,
            Json(GetPublicKeyResponse {
                public_key_pem: res.into_inner().public_key_pem,
            }),
        )
            .into_response(),
        Err(err) => handle_grpc_error(err),
    }
}
