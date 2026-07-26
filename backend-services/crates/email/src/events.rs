//! Event contract definitions for outbound email messaging payload structures.

use serde::{Deserialize, Serialize};

/// Payload structure sent to the broker to request an outbound verification email.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmailDispatchEvent {
    /// Recipient email address.
    pub target_email: String,

    /// Random confirmation code sequence generated for the user challenge.
    pub verification_code: String,

    /// Purpose context of the code lifecycle transition.
    pub verification_type: String,
}
