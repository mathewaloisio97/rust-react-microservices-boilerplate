//! Data Transfer Objects (DTOs) for the human verification subsystem.
//!
//! Defines the validation contracts and public-facing JSON schemas used to serve
//! bot protection challenges and process interactive telemetry submissions.

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

/// The telemetry and verification data submitted by the client interface for validation.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ClientVerifyPayload {
    /// The unique identifier of the active bot protection provider handling the evaluation.
    #[schema(example = "arrow_alignment")]
    pub provider_id: String,

    /// The raw, structured telemetry data and challenge contextual metadata.
    pub payload: serde_json::Value,
}
