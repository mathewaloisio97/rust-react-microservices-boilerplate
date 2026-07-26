//! Verification Provider interfaces and routing.

pub mod recaptcha;
pub mod turnstile;

use async_trait::async_trait;

/// Defines the contract for external human verification mechanisms.
#[async_trait]
pub trait VerificationProvider: Send + Sync {
    /// Evaluates the client-provided CAPTCHA token against the provider's API.
    async fn verify(&self, token: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}
