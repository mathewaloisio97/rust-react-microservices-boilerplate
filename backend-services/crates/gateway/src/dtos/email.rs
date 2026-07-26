//! Data Transfer Objects (DTOs) for user email and verification lifecycle management.
//!
//! Defines the public-facing JSON schemas and validation contracts for managing
//! active configurations, updating email settings, and verifying authorization challenges.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response payload containing the active and pending verification state of a user's email configuration.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EmailStateResponse {
    /// The current confirmed and active email address.
    pub current_email: String,

    /// Indicates if the active email address has been successfully verified.
    pub is_verified: bool,

    /// A newly requested email address awaiting verification tokens.
    pub pending_new_email: String,

    /// The current stage within the active verification state machine.
    pub verification_type: String,
}

/// Request payload to update or register a user's email destination address.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SetEmailPayload {
    /// The new target email address.
    #[schema(example = "user@example.com")]
    pub email: String,
}

impl SetEmailPayload {
    /// Evaluates whether the basic structure of the email string meets format baselines.
    pub fn is_valid(&self) -> bool {
        !self.email.trim().is_empty() && self.email.contains('@')
    }
}

/// Response payload indicating the execution status of an email change registration request.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SetEmailResponse {
    /// The structural outcome applied by the state machine (e.g., 'UNVERIFIED', 'ALREADY_VERIFIED').
    #[schema(example = "UNVERIFIED")]
    pub status: String,
}

/// Request payload containing a challenge validation token received via an email dispatch event.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct VerifyEmailPayload {
    /// The 6-digit numeric verification code string.
    #[schema(example = "123456")]
    pub code: String,
}

impl VerifyEmailPayload {
    /// Ensures the submitted token sequence string is non-empty.
    pub fn is_valid(&self) -> bool {
        !self.code.trim().is_empty()
    }
}

/// Response payload returning the programmatic outcome of a challenge code evaluation.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct VerifyEmailResponse {
    /// Indicates whether the submitted challenge successfully updated the state machine context.
    pub success: bool,
}
