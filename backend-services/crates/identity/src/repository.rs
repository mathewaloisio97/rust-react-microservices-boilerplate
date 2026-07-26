//! User database storage and lookup infrastructure.
//!
//! Provides transactional boundaries and retry mechanisms for managing
//! core identities, local credentials, and OAuth mappings.

use crate::errors::IdentityError;
use crate::models::{LocalCredential, OAuthLink};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

/// Defines operational contracts for managing user accounts in persistent storage.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Provisions a new core user and their associated local credentials.
    async fn create_local_user(
        &self,
        email: &str,
        password_hash: &str,
    ) -> Result<Uuid, IdentityError>;

    /// Retrieves local credential data by exact email match.
    async fn get_local_credential(
        &self,
        email: &str,
    ) -> Result<Option<LocalCredential>, IdentityError>;

    /// Updates the email address associated with an existing local credential record.
    async fn update_local_email(&self, user_id: Uuid, new_email: &str)
        -> Result<(), IdentityError>;

    /// Provisions a new core user and their associated OAuth mapping.
    async fn create_oauth_user(
        &self,
        provider: &str,
        subject_id: &str,
    ) -> Result<Uuid, IdentityError>;

    /// Retrieves an OAuth link mapping to identify an existing user.
    async fn get_oauth_link(
        &self,
        provider: &str,
        subject_id: &str,
    ) -> Result<Option<OAuthLink>, IdentityError>;
}

/// PostgreSQL implementation of the user repository using SQLx.
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    /// Instantiates a new repository utilizing the provided connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Automates exponential backoff for transient database connection faults.
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
                        tracing::warn!(
                            "Transient database fault detected. Retrying in {:?}...",
                            backoff
                        );
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
impl UserRepository for PostgresUserRepository {
    #[tracing::instrument(skip(self, password_hash))]
    async fn create_local_user(
        &self,
        email: &str,
        password_hash: &str,
    ) -> Result<Uuid, IdentityError> {
        let new_user_id = Uuid::now_v7();

        let mut tx = self.pool.begin().await.map_err(IdentityError::Database)?;

        sqlx::query!("INSERT INTO users (id) VALUES ($1)", new_user_id)
            .execute(&mut *tx)
            .await
            .map_err(IdentityError::Database)?;

        let result = sqlx::query!(
            "INSERT INTO local_credentials (user_id, email, password_hash) VALUES ($1, $2, $3)",
            new_user_id,
            email,
            password_hash
        )
        .execute(&mut *tx)
        .await;

        match result {
            Ok(_) => {
                tx.commit().await.map_err(IdentityError::Database)?;
                Ok(new_user_id)
            }
            Err(e) => {
                tx.rollback().await.ok();
                if let Some(db_err) = e.as_database_error() {
                    if db_err.code().as_deref() == Some("23505") {
                        return Err(IdentityError::EmailAlreadyExists(email.to_string()));
                    }
                }
                Err(IdentityError::Database(e))
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get_local_credential(
        &self,
        email: &str,
    ) -> Result<Option<LocalCredential>, IdentityError> {
        let record = with_retry!(sqlx::query_as!(
            LocalCredential,
            r#"SELECT user_id, email, password_hash, created_at as "created_at: _" FROM local_credentials WHERE email = $1"#,
            email
        )
        .fetch_optional(&self.pool))
        .map_err(IdentityError::Database)?;

        Ok(record)
    }

    #[tracing::instrument(skip(self))]
    async fn update_local_email(
        &self,
        user_id: Uuid,
        new_email: &str,
    ) -> Result<(), IdentityError> {
        with_retry!(sqlx::query!(
            "UPDATE local_credentials SET email = $1 WHERE user_id = $2",
            new_email,
            user_id
        )
        .execute(&self.pool))
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.code().as_deref() == Some("23505") {
                    return IdentityError::EmailAlreadyExists(new_email.to_string());
                }
            }
            IdentityError::Database(e)
        })?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn create_oauth_user(
        &self,
        provider: &str,
        subject_id: &str,
    ) -> Result<Uuid, IdentityError> {
        let new_user_id = Uuid::now_v7();

        let mut tx = self.pool.begin().await.map_err(IdentityError::Database)?;

        sqlx::query!("INSERT INTO users (id) VALUES ($1)", new_user_id)
            .execute(&mut *tx)
            .await
            .map_err(IdentityError::Database)?;

        sqlx::query!(
            "INSERT INTO oauth_links (user_id, provider, provider_subject_id) VALUES ($1, $2, $3)",
            new_user_id,
            provider,
            subject_id
        )
        .execute(&mut *tx)
        .await
        .map_err(IdentityError::Database)?;

        tx.commit().await.map_err(IdentityError::Database)?;

        Ok(new_user_id)
    }

    #[tracing::instrument(skip(self))]
    async fn get_oauth_link(
        &self,
        provider: &str,
        subject_id: &str,
    ) -> Result<Option<OAuthLink>, IdentityError> {
        let record = with_retry!(sqlx::query_as!(
            OAuthLink,
            r#"SELECT user_id, provider, provider_subject_id, created_at as "created_at: _" FROM oauth_links WHERE provider = $1 AND provider_subject_id = $2"#,
            provider, subject_id
        )
        .fetch_optional(&self.pool))
        .map_err(IdentityError::Database)?;

        Ok(record)
    }
}
