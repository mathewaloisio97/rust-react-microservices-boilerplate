//! gRPC server execution entrypoint for the Human Verification service.
//!
//! Handles challenges and solution validations (e.g., reCAPTCHA, Turnstile)
//! and issues signed cryptographic vouchers upon successful human verification.

use cleard_constants::security::DEFAULT_HV_SECRET;
use cleard_contracts::human_verification::v1::human_verification_service_server::HumanVerificationServiceServer;
use cleard_human_verification::config::VerificationConfig;
use cleard_human_verification::grpc::VerificationGrpcServer;
use cleard_human_verification::providers::{
    recaptcha::RecaptchaProvider, turnstile::TurnstileProvider,
};
use cleard_human_verification_crypto::CryptoEngine;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = VerificationConfig::from_env();
    let hv_secret =
        env::var("HUMAN_VERIFICATION_SECRET").unwrap_or_else(|_| DEFAULT_HV_SECRET.to_string());

    if hv_secret == DEFAULT_HV_SECRET {
        if cfg!(feature = "local-dev") {
            warn!("===========================================================");
            warn!("SECURITY ALERT: Running with the default development secret");
            warn!("===========================================================");
        } else {
            tracing::error!(
                "FATAL: Insecure fallback secret detected without local-dev authorization!"
            );
            panic!("Insecure configuration payload blocked.");
        }
    }

    let crypto = Arc::new(CryptoEngine::new(hv_secret.as_bytes()));

    let recaptcha = config.recaptcha_secret_key.clone().map(|key| {
        info!("Human Verification: Google reCAPTCHA provider enabled.");
        Arc::new(RecaptchaProvider::new(key))
    });
    if recaptcha.is_none() {
        warn!("Human Verification: Google reCAPTCHA provider disabled (missing RECAPTCHA_SECRET_KEY).");
    }

    let turnstile = config.turnstile_secret_key.clone().map(|key| {
        info!("Human Verification: Cloudflare Turnstile provider enabled.");
        Arc::new(TurnstileProvider::new(key))
    });
    if turnstile.is_none() {
        warn!("Human Verification: Cloudflare Turnstile provider disabled (missing TURNSTILE_SECRET_KEY).");
    }

    let grpc_service = VerificationGrpcServer {
        crypto,
        token_timeout_secs: config.token_timeout_secs,
        recaptcha,
        turnstile,
    };

    let addr: SocketAddr = "0.0.0.0:50055".parse().unwrap();
    info!(
        "Cleard Human Verification gRPC Service actively listening on {}",
        addr
    );

    Server::builder()
        .add_service(HumanVerificationServiceServer::new(grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}
