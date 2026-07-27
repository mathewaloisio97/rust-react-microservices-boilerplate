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
        #[cfg(feature = "local-dev")]
        let (recaptcha_secret_key, turnstile_secret_key) = {
            tracing::warn!(
                "CAPTCHA: local-dev mode active. Forcefully overriding environment with official dummy testing secrets."
            );
            (
                Some("6LeIxAcTAAAAAGG-vFI1TnRWxMZNFuojJ4WifJWe".to_string()),
                Some("1x0000000000000000000000000000000AA".to_string()),
            )
        };

        #[cfg(not(feature = "local-dev"))]
        let (recaptcha_secret_key, turnstile_secret_key) = (
            env::var("RECAPTCHA_SECRET_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            env::var("TURNSTILE_SECRET_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        );

        Self {
            token_timeout_secs: env::var("HUMAN_VERIFICATION_TOKEN_TIMEOUT")
                .unwrap_or_else(|_| "900".to_string())
                .parse()
                .expect("Invalid token timeout"),
            recaptcha_secret_key,
            turnstile_secret_key,
        }
    }
}
