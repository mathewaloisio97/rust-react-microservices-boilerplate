//! Authentication session token storage structures.
//!
//! Enforces strongly typed domain models mapping directly to stateful infrastructure
//! records. These representations decouple low-level relational table schemas from
//! outbound boundary protocols and transport-layer contracts.

use time::OffsetDateTime;
use uuid::Uuid;

/// Represents a stateful session token record mapped from the database tracking store.
///
/// This structure mirrors the `tokens` relation, maintaining a strict mapping of active
/// user sessions and stateful invalidation statuses to support security auditing.
#[derive(Debug, Clone)]
pub struct TokenRecord {
    /// The unique, Base64Url-encoded token string identifier.
    pub token: String,

    /// The unique UUIDv7 identifier of the user associated with this session token.
    pub user_id: Uuid,

    /// Invalidation flag indicating whether the session has been soft-deleted or revoked.
    pub revoked: bool,

    /// The timestamp when the session record was generated.
    pub created_at: OffsetDateTime,
}
