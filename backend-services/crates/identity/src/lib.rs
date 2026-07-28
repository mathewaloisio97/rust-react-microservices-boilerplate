//! User identity and authentication subsystem.
//!
//! Encapsulates database access operations, provides cryptographic
//! password validation via Argon2id, handles OAuth mapping workflows,
//! and exposes gRPC service implementations.

pub mod errors;
pub mod models;
pub mod oauth;
pub mod repository;

use crate::oauth::OAuthRegistry;
use crate::repository::UserRepository;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};
use uuid::Uuid;
use your_app_contracts::identity::v1::identity_service_server::IdentityService;
use your_app_contracts::identity::v1::{
    ActivateUserRequest, ActivateUserResponse, AuthResponse, GetUserRequest, GetUserResponse,
    LoginLocalRequest, OAuthLoginRequest, RegisterLocalRequest, UpdateLocalEmailRequest,
    UpdateLocalEmailResponse,
};

/// Orchestrates user authentication and registration workflows.
pub struct YourAppIdentity {
    repo: Arc<dyn UserRepository>,
    oauth_registry: Arc<OAuthRegistry>,
}

impl YourAppIdentity {
    pub fn new(repo: Arc<dyn UserRepository>, oauth_registry: Arc<OAuthRegistry>) -> Self {
        Self {
            repo,
            oauth_registry,
        }
    }
}

#[tonic::async_trait]
impl IdentityService for YourAppIdentity {
    #[instrument(skip(self, req))]
    async fn get_user(
        &self,
        req: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let inner = req.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID format for user_id"))?;

        let user_opt = self.repo.get_user(user_id).await.map_err(|e| {
            error!("Database fault during user retrieval: {:?}", e);
            Status::internal("Internal database fault")
        })?;

        match user_opt {
            Some(u) => {
                let access_level = match u.access_level {
                    crate::models::AccessLevel::Default => {
                        your_app_contracts::identity::v1::AccessLevel::Default
                    }
                    crate::models::AccessLevel::Staff => {
                        your_app_contracts::identity::v1::AccessLevel::Staff
                    }
                    crate::models::AccessLevel::Admin => {
                        your_app_contracts::identity::v1::AccessLevel::Admin
                    }
                    crate::models::AccessLevel::SuperAdmin => {
                        your_app_contracts::identity::v1::AccessLevel::SuperAdmin
                    }
                    crate::models::AccessLevel::System => {
                        your_app_contracts::identity::v1::AccessLevel::System
                    }
                };

                let status = match u.status {
                    crate::models::UserStatus::Pending => {
                        your_app_contracts::identity::v1::UserStatus::Pending
                    }
                    crate::models::UserStatus::Active => {
                        your_app_contracts::identity::v1::UserStatus::Active
                    }
                    crate::models::UserStatus::Suspended => {
                        your_app_contracts::identity::v1::UserStatus::Suspended
                    }
                };

                Ok(Response::new(GetUserResponse {
                    user_id: u.id.to_string(),
                    access_level: access_level.into(),
                    status: status.into(),
                    created_at: u.created_at.to_string(),
                }))
            }
            None => Err(Status::not_found("User not found")),
        }
    }

    #[instrument(skip(self, req))]
    async fn activate_user(
        &self,
        req: Request<ActivateUserRequest>,
    ) -> Result<Response<ActivateUserResponse>, Status> {
        let inner = req.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID format for user_id"))?;

        self.repo.activate_user(user_id).await.map_err(|e| {
            error!("Database fault during user activation: {:?}", e);
            Status::internal("Internal database fault")
        })?;

        Ok(Response::new(ActivateUserResponse { success: true }))
    }

    #[instrument(skip(self, req))]
    async fn o_auth_login(
        &self,
        req: Request<OAuthLoginRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let inner = req.into_inner();
        let provider = inner.provider;
        let id_token = inner.id_token;

        let verified_subject_id = match self.oauth_registry.verify_token(&provider, &id_token).await
        {
            Ok(subject) => subject,
            Err(e) => {
                error!("OAuth Verification Failed [{}]: {}", provider, e);
                return Err(Status::unauthenticated("Invalid or rejected OAuth token"));
            }
        };

        info!(provider = %provider, "Processing OAuth authentication");

        match self
            .repo
            .get_oauth_link(&provider, &verified_subject_id)
            .await
        {
            Ok(Some(link)) => Ok(Response::new(AuthResponse {
                user_id: link.user_id.to_string(),
                valid: true,
            })),
            Ok(None) => {
                info!("Unrecognized OAuth identity. Provisioning new core user record.");
                let user_id = self
                    .repo
                    .create_oauth_user(&provider, &verified_subject_id)
                    .await
                    .map_err(|e| {
                        error!("Database fault during OAuth provisioning: {:?}", e);
                        Status::internal("Internal database fault")
                    })?;

                Ok(Response::new(AuthResponse {
                    user_id: user_id.to_string(),
                    valid: true,
                }))
            }
            Err(e) => {
                error!("Database fault during OAuth lookup: {:?}", e);
                Err(Status::internal("Internal database fault"))
            }
        }
    }

    #[instrument(skip(self, req))]
    async fn register_local(
        &self,
        req: Request<RegisterLocalRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let inner = req.into_inner();
        let email = inner.email.to_lowercase();
        let password = inner.password;

        if email.trim().is_empty() || password.trim().is_empty() {
            return Err(Status::invalid_argument(
                "Email and password constraints violated",
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| {
                error!("Cryptography fault during derivation: {}", e);
                Status::internal("Internal cryptography fault")
            })?
            .to_string();

        let user_id = self
            .repo
            .create_local_user(&email, &password_hash)
            .await
            .map_err(|e| match e {
                crate::errors::IdentityError::EmailAlreadyExists(_) => {
                    Status::already_exists("Email address already registered")
                }
                _ => {
                    error!("Database fault during local registration: {:?}", e);
                    Status::internal("Internal database fault")
                }
            })?;

        info!(user_id = %user_id, "Provisioned new local identity record");

        Ok(Response::new(AuthResponse {
            user_id: user_id.to_string(),
            valid: true,
        }))
    }

    #[instrument(skip(self, req))]
    async fn login_local(
        &self,
        req: Request<LoginLocalRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let inner = req.into_inner();
        let email = inner.email.to_lowercase();

        let record_opt = self.repo.get_local_credential(&email).await.map_err(|e| {
            error!("Database fault during credential retrieval: {}", e);
            Status::internal("Internal database fault")
        })?;

        if let Some(record) = record_opt {
            let parsed_hash = PasswordHash::new(&record.password_hash).map_err(|_| {
                error!("Data integrity fault: corrupted hash struct");
                Status::internal("Data integrity fault")
            })?;

            if Argon2::default()
                .verify_password(inner.password.as_bytes(), &parsed_hash)
                .is_ok()
            {
                return Ok(Response::new(AuthResponse {
                    user_id: record.user_id.to_string(),
                    valid: true,
                }));
            }
        }

        Ok(Response::new(AuthResponse {
            user_id: "".to_string(),
            valid: false,
        }))
    }

    #[instrument(skip(self, req))]
    async fn update_local_email(
        &self,
        req: Request<UpdateLocalEmailRequest>,
    ) -> Result<Response<UpdateLocalEmailResponse>, Status> {
        let inner = req.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID"))?;
        let new_email = inner.new_email.to_lowercase();

        self.repo
            .update_local_email(user_id, &new_email)
            .await
            .map_err(|e| match e {
                crate::errors::IdentityError::EmailAlreadyExists(_) => {
                    Status::already_exists("Email address already registered")
                }
                _ => {
                    error!("Database fault during local email sync: {:?}", e);
                    Status::internal("Internal database fault")
                }
            })?;

        info!(user_id = %user_id, "Synchronized local credential email mapping");

        Ok(Response::new(UpdateLocalEmailResponse { success: true }))
    }
}
