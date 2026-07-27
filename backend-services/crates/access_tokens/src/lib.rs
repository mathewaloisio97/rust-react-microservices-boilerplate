//! gRPC service implementation for minting stateless, short-lived JWT access tokens.
//!
//! Validates stateful session tokens upstream against the Auth microservice before
//! minting cryptographically signed JSON Web Tokens for downstream authorization.

pub mod jwt;

use crate::jwt::{Claims, JwtManager};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use time::OffsetDateTime;
use tonic::{transport::Channel, Request, Response, Status};
use tracing::{error, info, instrument};
use your_app_contracts::access_tokens::v1::access_tokens_service_server::AccessTokensService;
use your_app_contracts::access_tokens::v1::{
    GetPublicKeyRequest, GetPublicKeyResponse, IssueTokenRequest, IssueTokenResponse,
};
use your_app_contracts::auth::v1::auth_service_client::AuthServiceClient;
use your_app_contracts::auth::v1::AuthenticateRequest;

/// Implements the gRPC service responsible for validating sessions and issuing JWTs.
pub struct YourAppAccessTokens {
    jwt_manager: Arc<JwtManager>,
    auth_client: AuthServiceClient<Channel>,
}

impl YourAppAccessTokens {
    /// Instantiates the service with its JWT signing manager and upstream Auth gRPC client.
    pub fn new(jwt_manager: Arc<JwtManager>, auth_client: AuthServiceClient<Channel>) -> Self {
        Self {
            jwt_manager,
            auth_client,
        }
    }

    /// Computes a URL-safe Base64 SHA-256 hash of a session token for claim embedding.
    fn hash_session_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }
}

#[tonic::async_trait]
impl AccessTokensService for YourAppAccessTokens {
    #[instrument(skip(self, req))]
    async fn issue_token(
        &self,
        req: Request<IssueTokenRequest>,
    ) -> Result<Response<IssueTokenResponse>, Status> {
        let inner = req.into_inner();
        let session_token = inner.session_token;

        if session_token.is_empty() {
            return Err(Status::invalid_argument("Session token cannot be empty"));
        }

        info!("Validating session root-of-trust upstream.");

        let auth_req = tonic::Request::new(AuthenticateRequest {
            token: session_token.clone(),
        });

        let mut client = self.auth_client.clone();
        let auth_res = match client.authenticate(auth_req).await {
            Ok(res) => res.into_inner(),
            Err(e) => {
                error!("Auth Subsystem communication fault: {:?}", e);
                return Err(Status::unauthenticated("Invalid or revoked session token"));
            }
        };

        if !auth_res.valid {
            info!("Denied access token issuance for revoked or invalid session.");
            return Err(Status::unauthenticated("Invalid or revoked session token"));
        }

        // Apply a server-enforced lifetime ceiling of 1 hour (3600s).
        let requested_ttl = if inner.ttl_seconds == 0 {
            900
        } else {
            inner.ttl_seconds
        };
        let final_ttl = std::cmp::min(requested_ttl, 3600);

        let now = OffsetDateTime::now_utc();
        let exp = now + time::Duration::seconds(final_ttl as i64);
        let sid = Self::hash_session_token(&session_token);

        let claims = Claims {
            sub: auth_res.user_id.clone(),
            sid: sid.clone(),
            iss: "your_app_access_tokens".to_string(),
            iat: now.unix_timestamp() as usize,
            exp: exp.unix_timestamp() as usize,
            roles: inner.roles,
        };

        let access_token = self.jwt_manager.issue(&claims).map_err(|e| {
            error!("Cryptography Fault: Failed to sign JWT: {:?}", e);
            Status::internal("Failed to issue access token")
        })?;

        info!(user_id = %auth_res.user_id, sid = %sid, "Successfully minted stateless JWT.");

        Ok(Response::new(IssueTokenResponse {
            access_token,
            expires_at: exp.unix_timestamp() as u64,
        }))
    }

    #[instrument(skip(self))]
    async fn get_public_key(
        &self,
        _req: Request<GetPublicKeyRequest>,
    ) -> Result<Response<GetPublicKeyResponse>, Status> {
        info!("Exporting PKCS#8 Public Key for downstream verification.");
        Ok(Response::new(GetPublicKeyResponse {
            public_key_pem: self.jwt_manager.get_public_key_pem(),
        }))
    }
}
