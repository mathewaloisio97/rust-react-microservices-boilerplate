//! Shared application state for the API Gateway.
//!
//! Stores downstream clients and network connections in a thread-safe,
//! immutable struct shared across all request handlers.

use cleard_contracts::access_tokens::v1::access_tokens_service_client::AccessTokensServiceClient;
use cleard_contracts::auth::v1::auth_service_client::AuthServiceClient;
use cleard_contracts::email::v1::email_service_client::EmailServiceClient;
use cleard_contracts::human_verification::v1::human_verification_service_client::HumanVerificationServiceClient;
use cleard_contracts::identity::v1::identity_service_client::IdentityServiceClient;
use cleard_human_verification_crypto::CryptoEngine;
use tonic::transport::Channel;

/// Shared application state containing cloned gRPC service clients.
///
/// Tonic channels are internally multiplexed and connection-pooled,
/// making this struct cheap to clone across concurrent gateway request handlers.
#[derive(Clone)]
pub struct AppState {
    /// Client for the downstream Identity microservice.
    pub identity_client: IdentityServiceClient<Channel>,

    /// Client for the downstream Authentication microservice.
    pub auth_client: AuthServiceClient<Channel>,

    /// Client for the downstream Human Verification microservice.
    pub human_verification_client: HumanVerificationServiceClient<Channel>,

    /// Client for the downstream Email microservice.
    pub email_client: EmailServiceClient<Channel>,

    /// Client for the downstream Access Tokens microservice.
    pub access_tokens_client: AccessTokensServiceClient<Channel>,

    /// Cryptographic signing and token verification engine.
    pub crypto_engine: CryptoEngine,
}
