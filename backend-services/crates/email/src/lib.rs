//! Service layer handling email identity mappings and identity verification state machines.
//!
//! Exposes a gRPC interface to retrieve configuration states, stage multi-step email
//! address changes, and securely process verification code lifecycle transitions.

pub mod amqp;
pub mod errors;
pub mod events;
pub mod models;
pub mod repository;
pub mod worker;

use crate::{amqp::AmqpBroker, events::EmailDispatchEvent, repository::EmailRepository};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};
use uuid::Uuid;
use your_app_contracts::email::v1::email_service_server::EmailService;
use your_app_contracts::email::v1::{
    GetEmailRequest, GetEmailResponse, SetEmailRequest, SetEmailResponse, SetVerifiedEmailRequest,
    SetVerifiedEmailResponse, VerifyEmailRequest, VerifyEmailResponse,
};

/// Implements the gRPC service for managing user email state and confirmation lifecycles.
pub struct YourAppEmail {
    repo: Arc<dyn EmailRepository>,
    broker: Arc<AmqpBroker>,
}

impl YourAppEmail {
    pub fn new(repo: Arc<dyn EmailRepository>, broker: Arc<AmqpBroker>) -> Self {
        Self { repo, broker }
    }

    fn generate_code() -> String {
        format!("{:06}", rand::random_range(100000..=999999))
    }
}

#[tonic::async_trait]
impl EmailService for YourAppEmail {
    #[instrument(skip(self))]
    async fn get_email(
        &self,
        req: Request<GetEmailRequest>,
    ) -> Result<Response<GetEmailResponse>, Status> {
        let user_id = Uuid::parse_str(&req.get_ref().user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID"))?;

        let record = self.repo.get_email(user_id).await.map_err(|e| {
            error!("Database fault: {:?}", e);
            Status::internal("DB Error")
        })?;

        match record {
            Some(e) => Ok(Response::new(GetEmailResponse {
                current_email: e.current_email,
                is_verified: e.is_verified,
                pending_new_email: e.pending_new_email.unwrap_or_default(),
                verification_type: e.verification_type.unwrap_or_default(),
            })),
            None => Ok(Response::new(GetEmailResponse::default())),
        }
    }

    #[instrument(skip(self))]
    async fn set_email(
        &self,
        req: Request<SetEmailRequest>,
    ) -> Result<Response<SetEmailResponse>, Status> {
        let inner = req.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID"))?;

        let record = self.repo.get_email(user_id).await.map_err(|e| {
            error!("Database fault: {:?}", e);
            Status::internal("DB Error")
        })?;

        let code = Self::generate_code();
        let expires = OffsetDateTime::now_utc() + Duration::minutes(15);

        if let Some(r) = record {
            if r.is_verified {
                if r.current_email == inner.new_email {
                    return Ok(Response::new(SetEmailResponse {
                        status: "ALREADY_VERIFIED".into(),
                    }));
                }

                info!("Verified email modification requested. Dispatching confirmation challenge to old address.");
                self.repo
                    .set_pending_change(user_id, &inner.new_email, "CONFIRM_OLD", &code, expires)
                    .await
                    .map_err(|e| {
                        error!("Database fault: {:?}", e);
                        Status::internal("DB Error")
                    })?;

                let _ = self
                    .broker
                    .publish_dispatch(&EmailDispatchEvent {
                        target_email: r.current_email,
                        user_id: user_id.to_string(),
                        verification_code: code,
                        verification_type: "CONFIRM_OLD".into(),
                    })
                    .await;
                return Ok(Response::new(SetEmailResponse {
                    status: "PENDING_OLD_CONFIRMATION".into(),
                }));
            }
        }

        info!("Overwriting unverified email mapping.");
        self.repo
            .upsert_unverified(user_id, &inner.new_email, &code, expires)
            .await
            .map_err(|e| {
                error!("Database fault: {:?}", e);
                Status::internal("DB Error")
            })?;

        let _ = self
            .broker
            .publish_dispatch(&EmailDispatchEvent {
                target_email: inner.new_email,
                user_id: user_id.to_string(),
                verification_code: code,
                verification_type: "VERIFY_CURRENT".into(),
            })
            .await;

        Ok(Response::new(SetEmailResponse {
            status: "UNVERIFIED".into(),
        }))
    }

    #[instrument(skip(self))]
    async fn set_verified_email(
        &self,
        req: Request<SetVerifiedEmailRequest>,
    ) -> Result<Response<SetVerifiedEmailResponse>, Status> {
        let inner = req.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID"))?;

        self.repo
            .set_verified_email(user_id, &inner.email)
            .await
            .map_err(|e| {
                error!("Database fault: {:?}", e);
                Status::internal("DB Error")
            })?;

        Ok(Response::new(SetVerifiedEmailResponse { success: true }))
    }

    #[instrument(skip(self))]
    async fn verify_email(
        &self,
        req: Request<VerifyEmailRequest>,
    ) -> Result<Response<VerifyEmailResponse>, Status> {
        let inner = req.into_inner();
        let user_id = Uuid::parse_str(&inner.user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID"))?;

        let record = self
            .repo
            .get_email(user_id)
            .await
            .map_err(|e| {
                error!("Database fault: {:?}", e);
                Status::internal("DB Error")
            })?
            .ok_or_else(|| Status::not_found("Email not found"))?;

        let expired = record
            .code_expires_at
            .map(|e| e < OffsetDateTime::now_utc())
            .unwrap_or(true);

        // Anti-Squatter evaluation: strictly ensure the requested email matches the pending or current target
        // to prevent token spraying cross-contamination.
        let is_valid_email = record.current_email == inner.email
            || record.pending_new_email.as_deref() == Some(inner.email.as_str());

        if expired
            || record.verification_code.as_deref() != Some(inner.code.as_str())
            || !is_valid_email
        {
            return Ok(Response::new(VerifyEmailResponse {
                success: false,
                email_updated_to: "".to_string(),
            }));
        }

        match record.verification_type.as_deref() {
            Some("VERIFY_CURRENT") => {
                self.repo.mark_verified(user_id).await.map_err(|e| {
                    error!("Database fault: {:?}", e);
                    Status::internal("DB Error")
                })?;
                Ok(Response::new(VerifyEmailResponse {
                    success: true,
                    email_updated_to: "".to_string(),
                }))
            }
            Some("CONFIRM_OLD") => {
                let code = Self::generate_code();
                let expires = OffsetDateTime::now_utc() + Duration::minutes(15);
                self.repo
                    .transition_to_verify_new(user_id, &code, expires)
                    .await
                    .map_err(|e| {
                        error!("Database fault: {:?}", e);
                        Status::internal("DB Error")
                    })?;

                let new_target = record.pending_new_email.clone().unwrap_or_default();
                let _ = self
                    .broker
                    .publish_dispatch(&EmailDispatchEvent {
                        target_email: new_target,
                        user_id: user_id.to_string(),
                        verification_code: code,
                        verification_type: "VERIFY_NEW".into(),
                    })
                    .await;

                Ok(Response::new(VerifyEmailResponse {
                    success: true,
                    email_updated_to: "".to_string(),
                }))
            }
            Some("VERIFY_NEW") => {
                let new_email = record.pending_new_email.clone().unwrap_or_default();
                self.repo.apply_new_email(user_id).await.map_err(|e| {
                    error!("Database fault: {:?}", e);
                    Status::internal("DB Error")
                })?;
                Ok(Response::new(VerifyEmailResponse {
                    success: true,
                    email_updated_to: new_email,
                }))
            }
            _ => Err(Status::internal("Corrupted state machine")),
        }
    }
}
