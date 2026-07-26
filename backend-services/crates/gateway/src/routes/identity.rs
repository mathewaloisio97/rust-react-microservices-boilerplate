//! Gateway API route registration and middleware composition.
//!
//! Configures the top-level Axum router, wiring public HTTP endpoints to their
//! respective handlers and applying security layers such as human verification.

use crate::{handlers::identity, middleware::captcha::captcha_middleware, state::AppState};
use axum::{middleware, routing::post, Router};

/// Builds and configures the main API routing tree, attaching application state
/// and security middleware layers.
///
/// # Arguments
///
/// * `state` - The shared application state container wrapped in [`AppState`].
///
/// # Returns
///
/// A fully composed Axum [`Router`] ready to be bound to a TCP listener.
pub fn build_router(state: AppState) -> Router {
    // Apply captcha protection globally to all identity entry points to mitigate
    // automated credential stuffing and rapid account provisioning.
    Router::new()
        .route("/api/v1/register", post(identity::local_register))
        .route("/api/v1/login", post(identity::local_login))
        .route("/api/v1/oauth", post(identity::oauth_login))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            captcha_middleware,
        ))
        .with_state(state)
}
