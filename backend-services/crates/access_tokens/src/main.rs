//! Access Tokens service binary executable entry point.
//!
//! Initializes the gRPC server infrastructure, establishes connections to the
//! upstream authentication subsystem, and configures the RSA cryptographic engine.

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::{Channel, Server};
use tracing::info;
use your_app_access_tokens::jwt::JwtManager;
use your_app_access_tokens::YourAppAccessTokens;
use your_app_contracts::access_tokens::v1::access_tokens_service_server::AccessTokensServiceServer;
use your_app_contracts::auth::v1::auth_service_client::AuthServiceClient;

/// Application entry point configuring and executing the async gRPC service runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize distributed tracing.
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
    your_app_telemetry::init_telemetry("your_app_access_tokens", &otlp_endpoint)?;

    let auth_url = env::var("AUTH_SERVICE_URL").unwrap_or_else(|_| "http://localhost:50052".to_string());

    // Read the primary signing key from the environment.
    let private_key_pem = env::var("ACCESS_TOKENS_PRIVATE_KEY_PEM").ok();

    // Establish an instrumented channel to the upstream Auth subsystem.
    // This ensures outgoing gRPC requests automatically inject active trace contexts.
    let raw_auth_channel = Channel::from_shared(auth_url)?.connect_lazy();
    let instrumented_auth_channel = your_app_telemetry::instrument_channel(raw_auth_channel);
    let auth_client = AuthServiceClient::new(instrumented_auth_channel);

    // Pass the environment key to the manager. If None, it will fall back to local-dev persistent generation.
    let jwt_manager = Arc::new(JwtManager::new(private_key_pem));
    let service = YourAppAccessTokens::new(jwt_manager, auth_client);

    let port = env::var("PORT").unwrap_or_else(|_| "50054".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    info!("Access Tokens Issuance Service online at {}", addr);

    Server::builder()
        .layer(your_app_telemetry::OtelGrpcLayer)
        .add_service(AccessTokensServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
