//! Email routing layer.
//!
//! Orchestrates the email fetch, update, and verification sequence, protecting
//! mutation endpoints with anti-bot captcha verification.

use crate::{
    handlers::email,
    middleware::{auth_middleware, captcha_middleware},
    state::AppState,
};
use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};

pub fn build_router(state: AppState) -> Router {
    // Isolate the SET email route and apply Captcha middleware to it.
    let captcha_protected = Router::new()
        .route("/api/v1/email", post(email::set_email))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            captcha_middleware,
        ));

    // Define standard routes that only need user authentication.
    let auth_protected = Router::new()
        .route("/api/v1/email", get(email::get_email))
        .route("/api/v1/email/verify", post(email::verify_email));

    // Combine them and apply the auth middleware globally over this router.
    Router::new()
        .merge(captcha_protected)
        .merge(auth_protected)
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}
