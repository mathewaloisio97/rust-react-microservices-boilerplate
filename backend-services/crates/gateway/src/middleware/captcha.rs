//! Middleware validation layers for target edge routes.
//!
//! Provides traffic filtering controls to shield sensitive application entry points
//! from automated bot abuse by verifying cryptographic success proofs.

use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    Json,
};
use serde_json::json;

/// Captcha / Human Verification Gateway Middleware.
///
/// Intercepts incoming HTTP requests on protected edge endpoints (like /register),
/// extracts the `x-captcha-voucher` header, and verifies its cryptographic signature
/// statelessly. If the voucher is missing, forged, or expired, the request is dropped.
pub async fn captcha_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let token = match req.headers().get("x-captcha-voucher") {
        Some(h) => h.to_str().unwrap_or_default(),
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(
                    json!({"error": "Missing x-captcha-voucher header. Please complete human verification."}),
                ),
            ));
        }
    };

    // Mathematically verify the voucher.
    if state.crypto_engine.verify_voucher(token).is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Invalid or expired captcha voucher."})),
        ));
    }

    Ok(next.run(req).await)
}
