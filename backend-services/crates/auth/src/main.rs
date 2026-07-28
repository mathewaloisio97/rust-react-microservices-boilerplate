//! Authentication service binary executable entry point.
//!
//! Initializes the server infrastructure runtime environment, configures the underlying
//! relational database connection pool, applies outstanding schema migrations, and binds
//! the compiled gRPC transport layer to the network interface socket.

use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::{info, warn};
use your_app_auth::amqp::AmqpEventPublisher;
use your_app_auth::repository::PostgresTokenRepository;
use your_app_auth::YourAppAuth;
use your_app_contracts::auth::v1::auth_service_server::AuthServiceServer;

/// Application entry point configuring and executing the async gRPC service runtime.
///
/// # Errors
/// Returns an error if telemetry initialization, database connectivity,
/// embedded migration execution, or socket binding constraints fail to resolve.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry distributed tracing and logging.
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
    your_app_telemetry::init_telemetry("your_app_auth", &otlp_endpoint)?;

    // Pull database target URI from system environment variables or fallback to local baseline defaults.
    let db_url = env::var("AUTH_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost:5432/your_app_auth".to_string());

    // Pull AMQP target URI from system environment variables or fallback to local baseline defaults.
    let amqp_url =
        std::env::var("AUTH_AMQP_URL").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".to_string());

    // Allocate an asynchronous connection pool managed by the SQLx runtime engine.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Execute schema migrations embedded within the binary distribution context.
    info!("Applying Auth Database Migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("Auth DB Migrations Applied Successfully");

    // AMQP event broker loop with transient failure recovery.
    let mut retries = 5;
    let event_publisher = loop {
        match AmqpEventPublisher::new(&amqp_url).await {
            Ok(b) => break Arc::new(b),
            Err(e) => {
                retries -= 1;
                if retries == 0 {
                    panic!("Failed to connect to RabbitMQ AMQP: {}", e);
                }
                warn!("Waiting for RabbitMQ to become available...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    };

    // Instantiate thread-safe abstract database access object layers.
    let repo = Arc::new(PostgresTokenRepository::new(pool));
    let service = YourAppAuth::new(repo, event_publisher);

    // Bind and resolve the listener interface address.
    let port = env::var("PORT").unwrap_or_else(|_| "50052".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    info!("Auth gRPC Service actively listening on {}", addr);

    // Bootstrap and block the thread on the asynchronous Tonic gRPC server orchestrator.
    Server::builder()
        .layer(your_app_telemetry::OtelGrpcLayer)
        .add_service(AuthServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
