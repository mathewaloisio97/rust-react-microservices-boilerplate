//! Binary entry point for the Email identity microservice.
//!
//! Initializes environment configuration, runs database schema migrations, connects
//! to the infrastructure dependencies, and boots the gRPC API server alongside the
//! asynchronous background email worker.

use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::{info, warn};
use your_app_contracts::email::v1::email_service_server::EmailServiceServer;
use your_app_email::amqp::AmqpBroker;
use your_app_email::repository::PostgresEmailRepository;
use your_app_email::worker::start_email_worker;
use your_app_email::YourAppEmail;

/// Application entry point configuring and executing the async gRPC service runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry distributed tracing and logging.
    let otlp_endpoint =
        std::env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
    your_app_telemetry::init_telemetry("your_app_email", &otlp_endpoint)?;

    // Parse runtime database and message queue parameters from environment fallbacks.
    let db_url = env::var("EMAIL_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost:5432/your_app_email".to_string());
    let amqp_url =
        std::env::var("EMAIL_AMQP_URL").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".to_string());

    // Parse SMTP transport configuration with container-friendly fallbacks.
    let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "mailpit".to_string());
    let smtp_port: u16 = env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1025);

    // Read optional SMTP credentials for production provider authentication.
    let smtp_username = env::var("SMTP_USERNAME").ok();
    let smtp_password = env::var("SMTP_PASSWORD").ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    info!("Applying Email Database Migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Connect to AMQP with a retry boundary to accommodate transient container startup ordering.
    let mut retries = 5;
    let broker = loop {
        match AmqpBroker::new(&amqp_url).await {
            Ok(b) => break Arc::new(b),
            Err(e) => {
                retries -= 1;
                if retries == 0 {
                    panic!("RabbitMQ Connection Failed: {}", e);
                }
                warn!("Waiting for RabbitMQ instance to become healthy...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    };

    info!(
        "Starting background SMTP worker targeting {}:{}...",
        smtp_host, smtp_port
    );
    start_email_worker(amqp_url, smtp_host, smtp_port, smtp_username, smtp_password).await;

    let repo = Arc::new(PostgresEmailRepository::new(pool));
    let service = YourAppEmail::new(repo, broker);

    let port = env::var("PORT").unwrap_or_else(|_| "50053".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    info!("Email gRPC Service actively listening on {}", addr);

    Server::builder()
        .layer(your_app_telemetry::OtelGrpcLayer)
        .add_service(EmailServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
