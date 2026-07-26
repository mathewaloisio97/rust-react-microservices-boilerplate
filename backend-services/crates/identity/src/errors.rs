//! Identity domain error definitions.
//!
//! Encapsulates failure states encountered during identity processing,
//! such as database constraints or connection faults.

use thiserror::Error;

/// Enumerates all errors originating from identity operations.
#[derive(Error, Debug)]
pub enum IdentityError {
    /// Indicates an attempt to register an email address that is already provisioned.
    #[error("account with email '{0}' already exists")]
    EmailAlreadyExists(String),

    /// Encapsulates underlying database engine or connection pool faults.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
