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
use your_app_contracts::identity::v1::{GetUserRequest, UserStatus};

/// Extracted user identifier injected into request extensions upon successful auth.
#[derive(Clone, Debug)]
pub struct UserId(pub String);

/// Verified raw session token injected into request extensions for downstream use.
#[derive(Clone)]
pub struct SessionToken(pub String);

/// Protects routes by validating Bearer tokens against the Auth microservice.
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

    let mut auth_client = state.auth_client.clone();
    let res = match auth_client.authenticate(grpc_req).await {
        Ok(r) => r.into_inner(),
        Err(e) => {
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

    // Fetch User Identity to retrieve authoritative lifecycle Status
    let user_req = tonic::Request::new(GetUserRequest {
        user_id: res.user_id.clone(),
    });

    let mut id_client = state.identity_client.clone();
    let user_res = match id_client.get_user(user_req).await {
        Ok(r) => r.into_inner(),
        Err(e) => {
            error!(
                "Gateway Auth: Failed to fetch user profile from Identity: {:?}",
                e
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to resolve identity profile"})),
            ));
        }
    };

    let path = req.uri().path();

    // Block all protected routes if the account is suspended, EXCEPT the logout route.
    if user_res.status == UserStatus::Suspended as i32 && path != "/api/v1/logout" {
        info!(
            "Gateway Auth: Request rejected for suspended user {}",
            res.user_id
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "ACCOUNT_SUSPENDED",
                "message": "Your account has been suspended."
            })),
        ));
    }

    // Restrict unverified squatter accounts to only email modifications or session revocation.
    if user_res.status == UserStatus::Pending as i32
        && !path.starts_with("/api/v1/email")
        && path != "/api/v1/logout"
    {
        info!(
            "Gateway Auth: Request rejected for unverified user {}",
            res.user_id
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "ACCOUNT_UNVERIFIED",
                "message": "Please verify your email address to access this feature."
            })),
        ));
    }

    req.extensions_mut().insert(UserId(res.user_id));
    req.extensions_mut().insert(SessionToken(token));

    Ok(next.run(req).await)
}
