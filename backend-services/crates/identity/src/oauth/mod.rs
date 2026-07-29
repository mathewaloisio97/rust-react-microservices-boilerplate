//! External OAuth identity routing and verification engine.
//!
//! Exposes a centralized registry to manage enabled identity providers,
//! routing verification requests to the appropriate implementation strategy.

pub mod oidc;

use self::oidc::OidcProvider;
use std::env;
use std::sync::Arc;
use tracing::{info, warn};

/// Central registry managing all active OAuth identity providers.
pub struct OAuthRegistry {
    google: Option<Arc<OidcProvider>>,
    apple: Option<Arc<OidcProvider>>,
}

impl OAuthRegistry {
    /// Constructs the registry by reading configuration from environment variables.
    /// Providers missing required configuration will be explicitly disabled.
    pub fn from_env() -> Self {
        let google = env::var("GOOGLE_CLIENT_ID").ok().map(|id| {
            info!("OAuth: Google provider enabled.");
            Arc::new(OidcProvider::new(
                id,
                "https://www.googleapis.com/oauth2/v3/certs",
                "https://accounts.google.com",
            ))
        });

        if google.is_none() {
            warn!("OAuth: Google provider disabled (GOOGLE_CLIENT_ID not set).");
        }

        let apple = env::var("APPLE_CLIENT_ID").ok().map(|id| {
            info!("OAuth: Apple provider enabled.");
            Arc::new(OidcProvider::new(
                id,
                "https://appleid.apple.com/auth/keys",
                "https://appleid.apple.com",
            ))
        });

        if apple.is_none() {
            warn!("OAuth: Apple provider disabled (APPLE_CLIENT_ID not set).");
        }

        Self { google, apple }
    }

    /// Routes the provided token to the appropriate identity provider verification logic.
    /// Returns a tuple of `(subject_id, email, email_verified)`.
    pub async fn verify_token(
        &self,
        provider: &str,
        token: &str,
    ) -> Result<(String, String, bool), anyhow::Error> {
        match provider.to_lowercase().as_str() {
            "google" => {
                let p = self
                    .google
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Google OAuth is not configured"))?;
                p.verify(token).await
            }
            "apple" => {
                let p = self
                    .apple
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Apple OAuth is not configured"))?;
                p.verify(token).await
            }
            _ => Err(anyhow::anyhow!("Unsupported OAuth provider: {provider}")),
        }
    }
}
