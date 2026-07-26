//! Domain and infrastructure errors encountered during email workflows.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmailError {
    /// Failure during database operations.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
