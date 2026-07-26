//! Data access layer and PostgreSQL implementation for managing user email state.
//!
//! Provides abstract traits and concrete SQLx repositories to query, update, and
//! transition user verification codes and email records with built-in transient retry logic.

use crate::errors::EmailError;
use crate::models::UserEmail;
use async_trait::async_trait;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Storage abstraction for user email state management and verification flows.
#[async_trait]
pub trait EmailRepository: Send + Sync {
    /// Retrieves a user's email configuration if it exists.
    async fn get_email(&self, user_id: Uuid) -> Result<Option<UserEmail>, EmailError>;

    /// Creates or overwrites an unverified email record with an initial verification code.
    async fn upsert_unverified(
        &self,
        user_id: Uuid,
        email: &str,
        code: &str,
        expires: OffsetDateTime,
    ) -> Result<(), EmailError>;

    /// Sets a staged email address change and its corresponding lifecycle code.
    async fn set_pending_change(
        &self,
        user_id: Uuid,
        pending_email: &str,
        vtype: &str,
        code: &str,
        expires: OffsetDateTime,
    ) -> Result<(), EmailError>;

    /// Updates the confirmation flow state to target verification of the newly requested email.
    async fn transition_to_verify_new(
        &self,
        user_id: Uuid,
        code: &str,
        expires: OffsetDateTime,
    ) -> Result<(), EmailError>;

    /// Promotes the pending email address to active and clears verification metadata.
    async fn apply_new_email(&self, user_id: Uuid) -> Result<(), EmailError>;

    /// Marks the active email address as verified and clears active verification codes.
    async fn mark_verified(&self, user_id: Uuid) -> Result<(), EmailError>;
}

/// PostgreSQL implementation of the email data access layer.
pub struct PostgresEmailRepository {
    pool: PgPool,
}

impl PostgresEmailRepository {
    /// Instantiates a new repository using the provided connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Retries an asynchronous database operation up to 3 times on transient network or pool errors.
macro_rules! with_retry {
    ($op:expr) => {{
        let mut retries = 3;
        let mut backoff = std::time::Duration::from_millis(100);
        loop {
            match $op.await {
                Ok(res) => break Ok(res),
                Err(e) => {
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
impl EmailRepository for PostgresEmailRepository {
    async fn get_email(&self, user_id: Uuid) -> Result<Option<UserEmail>, EmailError> {
        let rec = with_retry!(sqlx::query_as!(
            UserEmail,
            r#"SELECT user_id, current_email, is_verified, pending_new_email, verification_code, verification_type, code_expires_at as "code_expires_at: _" FROM user_emails WHERE user_id = $1"#,
            user_id
        ).fetch_optional(&self.pool))?;
        Ok(rec)
    }

    async fn upsert_unverified(
        &self,
        user_id: Uuid,
        email: &str,
        code: &str,
        expires: OffsetDateTime,
    ) -> Result<(), EmailError> {
        with_retry!(sqlx::query!(
            r#"
            INSERT INTO user_emails (user_id, current_email, is_verified, verification_type, verification_code, code_expires_at)
            VALUES ($1, $2, FALSE, 'VERIFY_CURRENT', $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
                current_email = EXCLUDED.current_email,
                is_verified = FALSE,
                verification_type = 'VERIFY_CURRENT',
                verification_code = EXCLUDED.verification_code,
                code_expires_at = EXCLUDED.code_expires_at,
                pending_new_email = NULL,
                updated_at = NOW()
            "#,
            user_id, email, code, expires
        ).execute(&self.pool))?;
        Ok(())
    }

    async fn set_pending_change(
        &self,
        user_id: Uuid,
        pending_email: &str,
        vtype: &str,
        code: &str,
        expires: OffsetDateTime,
    ) -> Result<(), EmailError> {
        with_retry!(sqlx::query!(
            r#"UPDATE user_emails SET pending_new_email = $2, verification_type = $3, verification_code = $4, code_expires_at = $5, updated_at = NOW() WHERE user_id = $1"#,
            user_id, pending_email, vtype, code, expires
        ).execute(&self.pool))?;
        Ok(())
    }

    async fn transition_to_verify_new(
        &self,
        user_id: Uuid,
        code: &str,
        expires: OffsetDateTime,
    ) -> Result<(), EmailError> {
        with_retry!(sqlx::query!(
            r#"UPDATE user_emails SET verification_type = 'VERIFY_NEW', verification_code = $2, code_expires_at = $3, updated_at = NOW() WHERE user_id = $1"#,
            user_id, code, expires
        ).execute(&self.pool))?;
        Ok(())
    }

    async fn apply_new_email(&self, user_id: Uuid) -> Result<(), EmailError> {
        with_retry!(sqlx::query!(
            r#"UPDATE user_emails SET current_email = pending_new_email, pending_new_email = NULL, is_verified = TRUE, verification_type = NULL, verification_code = NULL, code_expires_at = NULL, updated_at = NOW() WHERE user_id = $1"#,
            user_id
        ).execute(&self.pool))?;
        Ok(())
    }

    async fn mark_verified(&self, user_id: Uuid) -> Result<(), EmailError> {
        with_retry!(sqlx::query!(
            r#"UPDATE user_emails SET is_verified = TRUE, verification_type = NULL, verification_code = NULL, code_expires_at = NULL, updated_at = NOW() WHERE user_id = $1"#,
            user_id
        ).execute(&self.pool))?;
        Ok(())
    }
}
