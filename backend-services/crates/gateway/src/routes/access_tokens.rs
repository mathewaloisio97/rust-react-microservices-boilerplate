//! Axum router configuration for the access tokens subsystem.
//!
//! Assembles public and authenticated route endpoints, attaching the necessary
//! session validation middleware for token issuance requests.

use crate::{handlers::access_tokens, middleware::auth_middleware, state::AppState};
use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};

/// Builds and configures the API router for access token routes.
pub fn build_router(state: AppState) -> Router {
    // Routes requiring a valid root session token.
    let protected = Router::new()
        .route("/api/v1/access-tokens", post(access_tokens::issue_token))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Publicly accessible endpoints.
    let public = Router::new().route(
        "/api/v1/access-tokens/key",
        get(access_tokens::get_public_key),
    );

    Router::new()
        .merge(protected)
        .merge(public)
        .with_state(state)
}
