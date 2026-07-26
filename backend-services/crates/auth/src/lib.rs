//! Authentication gRPC service transport layer.
//!
//! Implements the boundary-facing gRPC contract endpoints for session orchestration.
//! Handles incoming request translation, cryptographic token generation, and coercion
//! of internal repository results into deterministic transport-layer network statuses.

pub mod amqp;
pub mod errors;
pub mod events;
pub mod models;
pub mod repository;

use crate::events::{EventPublisher, SessionRevokedEvent};
use crate::repository::TokenRepository;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use cleard_contracts::auth::v1::auth_service_server::AuthService;
use cleard_contracts::auth::v1::{
    AuthenticateRequest, AuthenticateResponse, CreateTokenRequest, CreateTokenResponse,
    RevokeTokenRequest, RevokeTokenResponse,
};
use rand::RngExt;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Core service coordinate orchestrating the identity and session token lifecycle over gRPC.
pub struct CleardAuth {
    /// Dispatched abstract data storage repository handling persistence operations.
    repo: Arc<dyn TokenRepository>,

    /// Abstract message broker publisher used to dispatch domain events across the system fabric.
    event_publisher: Arc<dyn EventPublisher>,
}

impl CleardAuth {
    /// Constructs a new gRPC service coordinator backed by a thread-safe repository variant.
    pub fn new(repo: Arc<dyn TokenRepository>, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self {
            repo,
            event_publisher,
        }
    }

    /// Generates a cryptographically secure 32-byte opaque token encoded as Base64Url-NoPad.
    fn generate_token() -> String {
        let bytes: [u8; 32] = rand::rng().random();
        URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Hashes an opaque token string using SHA-256 and encodes the digest as Base64Url-NoPad.
    ///
    /// This deterministic transformation allows safe storage, searching, and public broadcasting
    /// of token identifiers without exposing the raw secret material over the wire or in message logs.
    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }
}

#[tonic::async_trait]
impl AuthService for CleardAuth {
    #[instrument(skip(self, req))]
    async fn authenticate(
        &self,
        req: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let token = &req.get_ref().token;

        let record = self.repo.get_token(token).await.map_err(|e| {
            error!("Database error during token introspection: {:?}", e);
            Status::internal("Internal database fault")
        })?;

        match record {
            Some(tok) if !tok.revoked => Ok(Response::new(AuthenticateResponse {
                valid: true,
                user_id: tok.user_id.to_string(),
            })),
            _ => {
                info!("Security: Intercepted invalid, expired, or revoked token attempt");
                Err(Status::unauthenticated("Invalid or expired session token"))
            }
        }
    }

    #[instrument(skip(self, req))]
    async fn create_token(
        &self,
        req: Request<CreateTokenRequest>,
    ) -> Result<Response<CreateTokenResponse>, Status> {
        let user_id = Uuid::parse_str(&req.get_ref().user_id)
            .map_err(|_| Status::invalid_argument("Malformed UUID format for user_id payload"))?;

        let token = Self::generate_token();
        self.repo.create_token(&token, user_id).await.map_err(|e| {
            error!("Failed to commit token: {:?}", e);
            Status::internal("Internal database fault")
        })?;

        info!(
            "Auth: Provisioned new secure session token for user: {}",
            user_id
        );
        Ok(Response::new(CreateTokenResponse { token }))
    }

    #[instrument(skip(self, req))]
    async fn revoke_token(
        &self,
        req: Request<RevokeTokenRequest>,
    ) -> Result<Response<RevokeTokenResponse>, Status> {
        let token = &req.get_ref().token;
        let success = self.repo.revoke_token(token).await.map_err(|e| {
            error!("Failed to commit revocation: {:?}", e);
            Status::internal("Internal database fault")
        })?;

        if success {
            let sid = Self::hash_token(token);
            info!(
                "Auth: Token revoked in Database. Broadcasting 'SessionRevoked' event (SID: {}).",
                sid
            );

            // Broadcast the invalidation to all downstream microservices (e.g. Chat/WebSockets).
            if let Err(e) = self
                .event_publisher
                .publish_session_revoked(&SessionRevokedEvent { sid })
                .await
            {
                error!(
                    "Message Broker Fault: Failed to broadcast session revocation: {:?}",
                    e
                );
            }
        }

        Ok(Response::new(RevokeTokenResponse { success }))
    }
}
