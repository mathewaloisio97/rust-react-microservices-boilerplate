//! Identity Domain Storage Models.
//!
//! This module defines the internal domain models and entity structures that map
//! directly to persistent storage tables in the identity database layer. These
//! structures remain strictly decoupled from both external gateway HTTP contracts
//! and gRPC wire schemas.

use time::OffsetDateTime;
use uuid::Uuid;

/// Represents the operational state and lifecycle accessibility of a user's account.
///
/// Maps directly to the PostgreSQL `user_status` ENUM type in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    /// Account created but pending initial verification (e.g., email confirmation).
    Pending,

    /// Normal operating status with standard platform access granted.
    Active,

    /// Explicitly blocked from issuing tokens or accessing gateway-protected endpoints.
    Suspended,
}

/// Represents a persistently stored core user identity record.
///
/// Corresponds directly to the `users` table in PostgreSQL, evaluated alongside primary provider source.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct User {
    /// Primary key: globally unique identifier for the user (UUIDv7, time-ordered).
    pub id: Uuid,

    /// Lifecycle state governing account activation and endpoint accessibility.
    pub status: UserStatus,

    /// UTC timestamp indicating when the user record was inserted into persistent storage.
    pub created_at: OffsetDateTime,

    /// Evaluated string identifying the primary credential mechanism ("local", "google", "apple").
    pub credential_provider: String,
}

/// Represents a user's local email and password credentials.
///
/// Corresponds directly to the `local_credentials` table in PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct LocalCredential {
    /// Foreign key referencing `users.id` (1:1 primary key relationship).
    pub user_id: Uuid,

    /// Primary email address used for local password-based authentication.
    pub email: String,

    /// Cryptographic password hash generated via Argon2id (never stores plaintext).
    pub password_hash: String,

    /// UTC timestamp indicating when these credentials were provisioned or last reset.
    pub created_at: OffsetDateTime,
}

/// Represents an identity link established with an external third-party OAuth 2.0 / OIDC provider.
///
/// Corresponds directly to the `oauth_links` table in PostgreSQL.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct OAuthLink {
    /// Foreign key referencing `users.id` to map the third-party account to an internal identity.
    pub user_id: Uuid,

    /// Unique string identifier of the OAuth provider (e.g., `"google"`, `"github"`).
    pub provider: String,

    /// The unique, immutable subject ID (`sub` claim) assigned to the user by the external provider.
    pub provider_subject_id: String,

    /// UTC timestamp indicating when this third-party provider link was bound to the account.
    pub created_at: OffsetDateTime,
}
