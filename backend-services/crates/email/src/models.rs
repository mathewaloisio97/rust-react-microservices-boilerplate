//! Domain models and verification states for user email addresses.
//!
//! Provides the core data structures used to track active email configurations,
//! manage identity verification states, and handle safe multi-step email update flows.

use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserEmail {
    /// Unique identifier of the user.
    pub user_id: Uuid,

    /// Active, confirmed email address.
    pub current_email: String,

    /// Indicates if the current email has been verified.
    pub is_verified: bool,

    /// Unverified email address pending confirmation during an update flow.
    pub pending_new_email: Option<String>,

    /// Token sent to the user to confirm an email address.
    pub verification_code: Option<String>,

    /// Purpose of the active verification code.
    pub verification_type: Option<String>,

    /// Expiration timestamp for the active verification code.
    pub code_expires_at: Option<OffsetDateTime>,
}
