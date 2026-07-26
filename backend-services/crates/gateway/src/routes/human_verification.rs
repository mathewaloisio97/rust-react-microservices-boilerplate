//! Route tree compilation for the anti-bot engine gateway.
//!
//! Directs target path endpoints to the corresponding captcha generation
//! and client verification transaction handlers.

use crate::{handlers::human_verification, state::AppState};
use axum::{
    routing::{get, post},
    Router,
};

/// Generates the public routing layer for human verification check sequences.
///
/// Registers the challenge generation and payload verification execution hooks.
/// These routes bypass the standard gateway session validation layers, ensuring
/// anonymous clients can acquire verification vouchers prior to registry actions.
pub fn build_router(state: AppState) -> Router {
    // These routes are deliberately public (No Auth Middleware applied).
    // This allows unregistered users to solve captchas before hitting POST /register.
    Router::new()
        .route(
            "/api/v1/captcha/request",
            get(human_verification::get_challenge),
        )
        .route("/api/v1/captcha/verify", post(human_verification::verify))
        .with_state(state)
}
