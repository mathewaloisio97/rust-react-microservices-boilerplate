//! Data Transfer Objects (DTOs) for user email and verification lifecycle management.
//!
//! Defines the public-facing JSON schemas and validation contracts for managing
//! active configurations, updating email settings, and verifying authorization challenges.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response payload containing the active and pending verification state of a user's email configuration.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EmailStateResponse {
    /// The unique identifier (UUID v7) of the user account.
    #[schema(example = "0191701c-da51-749a-a4fc-e96579ebc043")]
    pub user_id: String,

    /// The current confirmed and active email address associated with the account.
    #[schema(example = "user@example.com")]
    pub current_email: String,

    /// Indicates if the active primary email address has been successfully verified.
    #[schema(example = true)]
    pub is_verified: bool,

    /// A newly requested target email address awaiting verification token completion; empty string if no change is staged.
    #[schema(example = "newuser@example.com")]
    pub pending_new_email: String,

    /// The current operational stage or mode within the active verification state machine.
    #[schema(example = "INITIAL_REGISTRATION")]
    pub verification_type: String,

    /// Explicitly flags if the user is authorized to change their email through the portal (e.g. true for local accounts, false for SSO).
    #[schema(example = true)]
    pub can_change_email: bool,

    /// The primary authentication provider associated with the user (e.g., "local", "google", "apple").
    #[schema(example = "google")]
    pub provider: String,
}

/// Request payload to register or stage a new user email destination address.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SetEmailPayload {
    /// The new target email address to register or set.
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
    /// The structural outcome applied by the state machine (e.g., 'UNVERIFIED', 'ALREADY_VERIFIED', 'VERIFICATION_SENT').
    #[schema(example = "UNVERIFIED")]
    pub status: String,
}

/// Request payload containing challenge credentials necessary to publicly verify an email update.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct VerifyEmailPayload {
    /// The unverified target email address undergoing authorization check.
    #[schema(example = "user@example.com")]
    pub email: String,

    /// The specific user identifier owning the email mapping.
    #[schema(example = "0191701c-da51-749a-a4fc-e96579ebc043")]
    pub user_id: String,

    /// The short-lived numeric or alphanumeric verification code string dispatched to the email.
    #[schema(example = "123456")]
    pub code: String,
}

impl VerifyEmailPayload {
    /// Ensures none of the validation token strings are blank prior to processing.
    pub fn is_valid(&self) -> bool {
        !self.email.trim().is_empty()
            && !self.user_id.trim().is_empty()
            && !self.code.trim().is_empty()
    }
}

/// Response payload returning the programmatic outcome of a challenge code evaluation.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct VerifyEmailResponse {
    /// Indicates whether the submitted challenge successfully updated the state machine context.
    #[schema(example = true)]
    pub success: bool,
}
