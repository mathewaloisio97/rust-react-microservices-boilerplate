//! HTTP routing layer for the human verification subsystem.
//!
//! Provides the public Axum handler entry points to request new bot protection
//! challenges and verify corresponding client telemetry payloads.

use crate::{
    dtos::{ChallengeQuery, ClientVerifyPayload},
    error::handle_grpc_error,
    state::AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use your_app_contracts::human_verification::v1::{GetChallengeRequest, VerifyRequest};

/// Requests a new bot-protection verification challenge payload from the backend gRPC engine.
///
/// Accepts query parameters specifying the provider and optional edition ID, forwards the
/// request down-funnel, and returns the serialized challenge data required to render the frontend widget.
///
/// # Arguments
///
/// * `state` - Shared application state container holding the gRPC service client.
/// * `query` - URL query parameters containing `provider_id` and optional `edition_id`.
///
/// # Returns
///
/// An Axum response containing `200 OK` with the JSON challenge payload, or a mapped gRPC error status.
#[utoipa::path(
    get,
    path = "/api/v1/captcha/request",
    params(ChallengeQuery),
    responses(
        (status = 200, description = "Challenge payload generated")
    )
)]
#[tracing::instrument(skip(state))]
pub async fn get_challenge(
    State(mut state): State<AppState>,
    Query(query): Query<ChallengeQuery>,
) -> impl IntoResponse {
    let req = tonic::Request::new(GetChallengeRequest {
        provider_id: query.provider_id,
        edition_id: query.edition_id.unwrap_or_default(),
    });

    match state.human_verification_client.get_challenge(req).await {
        Ok(res) => {
            let inner = res.into_inner();
            let challenge_json: serde_json::Value =
                serde_json::from_str(&inner.challenge_payload).unwrap_or_else(|_| json!({}));
            (StatusCode::OK, Json(challenge_json)).into_response()
        }
        Err(err) => handle_grpc_error(err),
    }
}

/// Verifies a completed client-side challenge response token or telemetry payload.
///
/// Submits the client payload to the verification gRPC service. If successful, a cryptographically
/// signed time-limited voucher token is returned; otherwise, a `401 Unauthorized` status code is issued.
///
/// # Arguments
///
/// * `state` - Shared application state container holding the gRPC service client.
/// * `payload` - JSON body containing the provider ID and client-side challenge token/telemetry.
///
/// # Returns
///
/// An Axum response containing `200 OK` with the `captcha_voucher`, `401 Unauthorized` on bot detection,
/// or a mapped gRPC system error status.
#[utoipa::path(
    post,
    path = "/api/v1/captcha/verify",
    request_body = ClientVerifyPayload,
    responses(
        (status = 200, description = "Verification passed, voucher issued"),
        (status = 401, description = "Verification failed (Bot detected)")
    )
)]
#[tracing::instrument(skip(state, payload))]
pub async fn verify(
    State(mut state): State<AppState>,
    Json(payload): Json<ClientVerifyPayload>,
) -> impl IntoResponse {
    let req = tonic::Request::new(VerifyRequest {
        provider_id: payload.provider_id,
        client_payload: payload.client_payload,
    });

    match state.human_verification_client.verify(req).await {
        Ok(res) => {
            let inner = res.into_inner();
            if inner.success {
                (
                    StatusCode::OK,
                    Json(json!({ "captcha_voucher": inner.voucher })),
                )
                    .into_response()
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "Human verification failed. Automated behavior detected." })),
                ).into_response()
            }
        }
        Err(err) => handle_grpc_error(err),
    }
}
