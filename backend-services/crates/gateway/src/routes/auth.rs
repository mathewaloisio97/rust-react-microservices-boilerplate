//! Gateway authentication routes.
//!
//! Registers secure web endpoints and sets up protection layers to make sure
//! only logged-in users can reach them.

use crate::{handlers::auth, middleware::auth_middleware, state::AppState};
use axum::{middleware as axum_middleware, routing::post, Router};

/// Creates the authentication API router and protects its routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Accepts a logout request to terminate a user's active session.
        .route("/api/v1/logout", post(auth::logout))
        // Wraps a protection layer around the routes above. This runs our
        // middleware script to verify session tokens before allowing access.
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        // Attaches our shared gRPC clients and state to the routes.
        .with_state(state)
}
