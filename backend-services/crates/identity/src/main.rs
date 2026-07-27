//! Identity microservice runtime entry point.
//!
//! Initializes telemetry routines, provisions database connection pooling,
//! executes automated schema migrations, configures OAuth providers,
//! and binds the gRPC interface.

use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;
use your_app_contracts::identity::v1::identity_service_server::IdentityServiceServer;
use your_app_identity::oauth::OAuthRegistry;
use your_app_identity::repository::PostgresUserRepository;
use your_app_identity::YourAppIdentity;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("IDENTITY_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost:5432/your_app_identity".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    info!("Applying Identity database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("Schema verification complete.");

    // Initialize the OAuth provider registry based on active environment variables.
    let oauth_registry = Arc::new(OAuthRegistry::from_env());

    let repo = Arc::new(PostgresUserRepository::new(pool));
    let service = YourAppIdentity::new(repo, oauth_registry);

    let addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
    info!(
        "YourApp Identity gRPC Service actively listening on {}",
        addr
    );

    Server::builder()
        .add_service(IdentityServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
