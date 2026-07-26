//! Identity Domain Storage Models.
//!
//! This module defines the internal domain models and entity structures that map
//! directly to persistent storage tables in the identity database layer. These
//! structures remain strictly decoupled from both external gateway HTTP contracts
//! and gRPC wire schemas.

use time::OffsetDateTime;
use uuid::Uuid;

/// Represents a persistently stored core user entity.
#[derive(Debug, Clone)]
pub struct User {
    /// The globally unique identifier for the user (UUIDv7).
    pub id: Uuid,
    /// The timestamp when the user account was originally created.
    pub created_at: OffsetDateTime,
}

/// Represents a user's traditional email and password credentials.
#[derive(Debug, Clone)]
pub struct LocalCredential {
    /// The unique identifier of the user these credentials belong to.
    pub user_id: Uuid,
    /// The unique email address used for login and account communication.
    pub email: String,
    /// The Argon2id representation of the user's password.
    pub password_hash: String,
    /// The timestamp when these credentials were created.
    pub created_at: OffsetDateTime,
}

/// Represents a linked third-party identity.
#[derive(Debug, Clone)]
pub struct OAuthLink {
    /// The unique identifier of the user linked to this external identity.
    pub user_id: Uuid,
    /// The name of the third-party identity provider (e.g., "google", "github").
    pub provider: String,
    /// The unique subject identifier provided by the third-party authentication service.
    pub provider_subject_id: String,
    /// The timestamp when the third-party account was linked.
    pub created_at: OffsetDateTime,
}
