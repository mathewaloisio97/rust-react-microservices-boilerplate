//! Gateway authentication middleware using upstream gRPC validation.
//!
//! Intercepts incoming HTTP requests to verify access tokens against the
//! core authentication microservice before allowing requests to reach
//! protected route handlers.

use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    Json,
};
use serde_json::json;
use tracing::{error, info};
use your_app_contracts::auth::v1::AuthenticateRequest;

/// Extracted user identifier injected into request extensions upon successful auth.
#[derive(Clone, Debug)]
pub struct UserId(pub String);

/// Verified raw session token injected into request extensions for downstream use.
#[derive(Clone)]
pub struct SessionToken(pub String);

/// Protects routes by validating Bearer tokens against the Auth microservice.
///
/// Extracts the token from the `Authorization` header, validates it via an
/// upstream gRPC call, and attaches the resulting `UserId` and `SessionToken`
/// to the request extensions. Missing, malformed, or rejected tokens short-circuit
/// the request with an HTTP 401 Unauthorized status.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = req.headers().get(AUTHORIZATION);

    let token = match auth_header {
        Some(header) => {
            let s = header.to_str().unwrap_or_default();
            if let Some(stripped) = s.strip_prefix("Bearer ") {
                stripped.to_string()
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Malformed Authorization header"})),
                ));
            }
        }
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing Authorization header"})),
            ));
        }
    };

    let grpc_req = tonic::Request::new(AuthenticateRequest {
        token: token.clone(),
    });

    let mut client = state.auth_client.clone();
    let res = match client.authenticate(grpc_req).await {
        Ok(r) => r.into_inner(),
        Err(e) => {
            // Log internal network/gRPC faults securely but return a generic 401 to the client.
            error!("Gateway Auth: Upstream gRPC communication fault: {:?}", e);
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired session token"})),
            ));
        }
    };

    if !res.valid {
        info!("Gateway Auth: Token validation rejected for request");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired session token"})),
        ));
    }

    // Inject verified contextual data for downstream route handlers.
    req.extensions_mut().insert(UserId(res.user_id));
    req.extensions_mut().insert(SessionToken(token));

    Ok(next.run(req).await)
}
