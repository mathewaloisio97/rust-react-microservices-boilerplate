//! Authentication domain error definitions.
//!
//! Groups all the errors that can happen during session tracking and
//! token processing, such as database query or connection failures.

use thiserror::Error;

/// Groups all errors that can occur during authentication operations.
#[derive(Error, Debug)]
pub enum AuthError {
    /// Sent back when the underlying database experiences a query or connection failure.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
