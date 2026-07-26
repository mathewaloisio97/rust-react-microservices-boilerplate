//! Environment-driven configuration for the human verification subsystem.

use std::env;

/// Configuration parameters for captcha validation and voucher issuance.
#[derive(Clone, Debug)]
pub struct VerificationConfig {
    /// Maximum lifespan of an issued verification voucher in seconds.
    pub token_timeout_secs: u64,

    /// Google reCAPTCHA secret key for server-side validation.
    pub recaptcha_secret_key: Option<String>,

    /// Cloudflare Turnstile secret key for server-side validation.
    pub turnstile_secret_key: Option<String>,
}

impl VerificationConfig {
    /// Loads parameters from environment variables, falling back to defaults if unset.
    pub fn from_env() -> Self {
        Self {
            token_timeout_secs: env::var("HUMAN_VERIFICATION_TOKEN_TIMEOUT")
                .unwrap_or_else(|_| "900".to_string())
                .parse()
                .expect("Invalid token timeout"),
            recaptcha_secret_key: env::var("RECAPTCHA_SECRET_KEY").ok(),
            turnstile_secret_key: env::var("TURNSTILE_SECRET_KEY").ok(),
        }
    }
}
