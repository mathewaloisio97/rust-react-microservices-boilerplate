//! Identity Domain Storage Models.
//!
//! This module defines the internal domain models and entity structures that map
//! directly to persistent storage tables in the identity database layer. These
//! structures remain strictly decoupled from both external gateway HTTP contracts
//! and gRPC wire schemas.

use time::OffsetDateTime;
use uuid::Uuid;

/// Represents the authorization boundary and administrative privileges of a user.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "access_level", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccessLevel {
    Default,
    Staff,
    Admin,
    SuperAdmin,
    System,
}

/// Represents the operational state and accessibility of the user's account.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    Pending,
    Active,
    Suspended,
}

/// Represents a persistently stored core user entity.
#[derive(Debug, Clone)]
pub struct User {
    /// The globally unique identifier for the user (UUIDv7).
    pub id: Uuid,
    /// The hierarchical role or tier granted to the user.
    pub access_level: AccessLevel,
    /// The lifecycle state of the user account.
    pub status: UserStatus,
    /// The timestamp when the user account was originally created.
    pub created_at: OffsetDateTime,
}

/// Represents a user's traditional email and password credentials.
#[derive(Debug, Clone)]
pub struct LocalCredential {
    pub user_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
}

/// Represents a linked third-party identity.
#[derive(Debug, Clone)]
pub struct OAuthLink {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_subject_id: String,
    pub created_at: OffsetDateTime,
}
