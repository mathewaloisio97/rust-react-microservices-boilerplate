//! Binary entry point for the YourApp email identity microservice.
//!
//! Initializes environment configuration, runs database schema migrations, connects
//! to the infrastructure dependencies, and boots the gRPC API server alongside the
//! asynchronous background email worker.

use your_app_contracts::email::v1::email_service_server::EmailServiceServer;
use your_app_email::amqp::AmqpBroker;
use your_app_email::repository::PostgresEmailRepository;
use your_app_email::worker::start_email_worker;
use your_app_email::YourAppEmail;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Parse runtime parameters from the environment fallback defaults.
    let db_url = std::env::var("EMAIL_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost:5432/your_app_email".to_string());
    let amqp_url =
        std::env::var("EMAIL_AMQP_URL").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".to_string());
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "1025".to_string())
        .parse()
        .unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    info!("Applying Email Migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Connect to AMQP with a retry boundary to accommodate transient container startup ordering.
    let mut retries = 5;
    let broker = loop {
        match AmqpBroker::new(&amqp_url).await {
            Ok(b) => break Arc::new(b),
            Err(e) => {
                retries -= 1;
                if retries == 0 {
                    panic!("RabbitMQ Failed: {}", e);
                }
                warn!("Waiting for RabbitMQ...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    };

    info!("Starting background SMTP worker...");
    start_email_worker(amqp_url, smtp_host, smtp_port).await;

    let repo = Arc::new(PostgresEmailRepository::new(pool));
    let service = YourAppEmail::new(repo, broker);

    let addr: SocketAddr = "0.0.0.0:50053".parse().unwrap();
    info!("Email gRPC Service listening on {}", addr);
    Server::builder()
        .add_service(EmailServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
