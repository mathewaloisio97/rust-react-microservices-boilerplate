//! HTTP error handling middleware for gRPC upstream services.
//!
//! Provides translation functions to catch Tonic gRPC status codes returned by
//! downstream microservices and map them into consistent, semantic JSON responses
//! for the Axum REST gateway.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::error;

/// Maps upstream gRPC error statuses to standard HTTP REST responses.
///
/// Expected errors (like bad inputs or conflicts) return their original message
/// to the user, while unexpected internal faults are logged securely and masked.
pub fn handle_grpc_error(err: tonic::Status) -> axum::response::Response {
    let (status, message) = match err.code() {
        tonic::Code::AlreadyExists => (StatusCode::CONFLICT, err.message().to_string()),
        tonic::Code::InvalidArgument => (StatusCode::BAD_REQUEST, err.message().to_string()),
        tonic::Code::Unauthenticated => (StatusCode::UNAUTHORIZED, err.message().to_string()),
        _ => {
            // Log the raw gRPC details internally but do not leak them to the client.
            error!("Upstream gRPC fault: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        }
    };

    (status, Json(json!({ "error": message }))).into_response()
}
