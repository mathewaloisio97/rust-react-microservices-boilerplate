//! Authentication session token infrastructure repository.
//!
//! Implements stateful persistence boundaries for security session data using PostgreSQL.
//! Handles transaction tracking, error coercion, and transient failure recovery via
//! exponential backoff mechanics.

use crate::errors::AuthError;
use crate::models::TokenRecord;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

/// Data access object abstraction layer enforcing storage invariants for stateful tokens.
///
/// Provides thread-safe, decoupled asynchronous operational definitions for creating,
/// retrieving, and invalidating security session contexts.
#[async_trait]
pub trait TokenRepository: Send + Sync {
    /// Commits a newly generated session context bound to a target user profile.
    ///
    /// # Errors
    /// Returns an [`AuthError::Database`] wrapper if the operation violates unique constraints
    /// or connection parameters fail.
    async fn create_token(&self, token: &str, user_id: Uuid) -> Result<(), AuthError>;

    /// Resolves an active or inactive token string against the current tracking store.
    ///
    /// # Errors
    /// Returns an [`AuthError::Database`] wrapper upon execution or parsing errors.
    async fn get_token(&self, token: &str) -> Result<Option<TokenRecord>, AuthError>;

    /// Executes a soft-delete operation to invalidate an active session token.
    ///
    /// Returns `true` if a record matched the signature and mutated successfully.
    ///
    /// # Errors
    /// Returns an [`AuthError::Database`] wrapper upon underlying database engine failure.
    async fn revoke_token(&self, token: &str) -> Result<bool, AuthError>;
}

/// A relational PostgreSQL infrastructure implementation of the [`TokenRepository`] contract.
pub struct PostgresTokenRepository {
    /// Active, thread-safe underlying connection pool instance.
    pool: PgPool,
}

impl PostgresTokenRepository {
    /// Generates a new instance of the repository around an initialized connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Helper macro to intercept and retry transient connection failures using an exponential backoff loop.
///
/// Mitigates system downtime spikes by evaluating pool timeouts, closed sockets, and low-level I/O disruptions.
macro_rules! with_retry {
    ($op:expr) => {{
        let mut retries = 3;
        let mut backoff = std::time::Duration::from_millis(100);
        loop {
            match $op.await {
                Ok(res) => break Ok(res),
                Err(e) => {
                    // Distinguish network blips or pool exhaustion scenarios from fatal syntax or constraint failures.
                    let is_transient = match &e {
                        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => true,
                        sqlx::Error::Io(_) => true,
                        _ => false,
                    };

                    if is_transient && retries > 0 {
                        tracing::warn!("Database transient failure, retrying in {:?}...", backoff);
                        retries -= 1;
                        tokio::time::sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }
                    break Err(e);
                }
            }
        }
    }};
}

#[async_trait]
impl TokenRepository for PostgresTokenRepository {
    #[tracing::instrument(skip(self, token))]
    async fn create_token(&self, token: &str, user_id: Uuid) -> Result<(), AuthError> {
        with_retry!(sqlx::query!(
            "INSERT INTO tokens (token, user_id) VALUES ($1, $2)",
            token,
            user_id
        )
        .execute(&self.pool))
        .map_err(AuthError::Database)?;

        Ok(())
    }

    #[tracing::instrument(skip(self, token))]
    async fn get_token(&self, token: &str) -> Result<Option<TokenRecord>, AuthError> {
        let record = with_retry!(sqlx::query_as!(
            TokenRecord,
            r#"SELECT token, user_id, revoked, created_at as "created_at: _" FROM tokens WHERE token = $1"#,
            token
        )
        .fetch_optional(&self.pool))
        .map_err(AuthError::Database)?;

        Ok(record)
    }

    #[tracing::instrument(skip(self, token))]
    async fn revoke_token(&self, token: &str) -> Result<bool, AuthError> {
        let result = with_retry!(sqlx::query!(
            "UPDATE tokens SET revoked = TRUE WHERE token = $1",
            token
        )
        .execute(&self.pool))
        .map_err(AuthError::Database)?;

        Ok(result.rows_affected() > 0)
    }
}
