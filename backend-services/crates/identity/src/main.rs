//! Identity microservice runtime entry point.
//!
//! Initializes telemetry routines, provisions database connection pooling,
//! executes automated schema migrations, configures OAuth providers,
//! and binds the gRPC interface.

use cleard_contracts::identity::v1::identity_service_server::IdentityServiceServer;
use cleard_identity::oauth::OAuthRegistry;
use cleard_identity::repository::PostgresUserRepository;
use cleard_identity::CleardIdentity;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("IDENTITY_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost:5432/cleard_identity".to_string());

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
    let service = CleardIdentity::new(repo, oauth_registry);

    let addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
    info!(
        "Cleard Identity gRPC Service actively listening on {}",
        addr
    );

    Server::builder()
        .add_service(IdentityServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
