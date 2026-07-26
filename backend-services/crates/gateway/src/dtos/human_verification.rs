//! Data Transfer Objects (DTOs) for the human verification subsystem.
//!
//! Defines the validation contracts and public-facing JSON schemas used to serve
//! bot protection challenges and process verification tokens from external cloud providers.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Query parameters used to fetch a customized captcha challenge layout configuration.
#[derive(Deserialize, IntoParams, Debug)]
pub struct ChallengeQuery {
    /// The unique identifier of the target bot protection provider engine.
    pub provider_id: String,

    /// An optional iteration or layout variant index to alter the challenge state.
    pub edition_id: Option<String>,
}

/// The verification token data submitted by the client interface for validation.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ClientVerifyPayload {
    /// The unique identifier of the active bot protection provider handling the evaluation.
    #[schema(example = "turnstile")]
    pub provider_id: String,

    /// The raw token string or structured payload returned by the provider's client-side widget.
    #[schema(example = "0.XXXXX...")]
    pub client_payload: String,
}
