//! External OAuth identity routing and verification engine.
//!
//! Exposes a centralized registry to manage enabled identity providers,
//! routing verification requests to the appropriate implementation strategy.

pub mod facebook;
pub mod oidc;

use self::facebook::FacebookProvider;
use self::oidc::OidcProvider;
use std::sync::Arc;
use tracing::{info, warn};

/// Central registry managing all active OAuth identity providers.
pub struct OAuthRegistry {
    google: Option<Arc<OidcProvider>>,
    apple: Option<Arc<OidcProvider>>,
    facebook: Option<Arc<FacebookProvider>>,
}

impl OAuthRegistry {
    /// Constructs the registry by reading configuration from environment variables.
    /// Providers missing required configuration will be explicitly disabled.
    pub fn from_env() -> Self {
        let google = std::env::var("GOOGLE_CLIENT_ID").ok().map(|id| {
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

        let apple = std::env::var("APPLE_CLIENT_ID").ok().map(|id| {
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

        let facebook = match (
            std::env::var("FACEBOOK_APP_ID").ok(),
            std::env::var("FACEBOOK_APP_SECRET").ok(),
        ) {
            (Some(id), Some(secret)) => {
                info!("OAuth: Facebook provider enabled.");
                Some(Arc::new(FacebookProvider::new(id, secret)))
            }
            _ => {
                warn!("OAuth: Facebook provider disabled (FACEBOOK_APP_ID or FACEBOOK_APP_SECRET not set).");
                None
            }
        };

        Self {
            google,
            apple,
            facebook,
        }
    }

    /// Routes the provided token to the appropriate identity provider verification logic.
    pub async fn verify_token(&self, provider: &str, token: &str) -> Result<String, anyhow::Error> {
        match provider.to_lowercase().as_str() {
            "google" => {
                let p = self.google.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Google OAuth is not configured on this server")
                })?;
                p.verify(token).await
            }
            "apple" => {
                let p = self.apple.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Apple OAuth is not configured on this server")
                })?;
                p.verify(token).await
            }
            "facebook" => {
                let p = self.facebook.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Facebook OAuth is not configured on this server")
                })?;
                p.verify(token).await
            }
            _ => Err(anyhow::anyhow!("Unsupported OAuth provider: {}", provider)),
        }
    }
}
