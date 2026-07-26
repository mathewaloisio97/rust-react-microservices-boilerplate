//! OpenID Connect (OIDC) Verification Engine.
//!
//! Handles cryptographic validation of JWTs via JSON Web Key Sets (JWKS).
//! Utilized by standard compliant identity providers (e.g., Google, Apple).

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Standard claims expected in a verified OIDC identity token.
#[derive(Debug, Deserialize)]
struct OidcClaims {
    sub: String,
}

#[derive(Deserialize, Clone)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

/// Generic OIDC Provider supporting JWKS key rotation and thread-safe caching.
pub struct OidcProvider {
    client_id: String,
    jwks_url: String,
    issuer: String,
    http_client: Client,
    /// Thread-safe cache of public RSA keys mapped by their Key ID (kid).
    key_cache: RwLock<HashMap<String, DecodingKey>>,
}

impl OidcProvider {
    /// Instantiates a new OIDC verification provider.
    pub fn new(client_id: String, jwks_url: &str, issuer: &str) -> Self {
        Self {
            client_id,
            jwks_url: jwks_url.to_string(),
            issuer: issuer.to_string(),
            http_client: Client::new(),
            key_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Fetches the latest JWKS from the provider and updates the internal cache.
    async fn refresh_keys(&self) -> Result<(), anyhow::Error> {
        let response: JwksResponse = self
            .http_client
            .get(&self.jwks_url)
            .send()
            .await?
            .json()
            .await?;

        let mut cache = self.key_cache.write().await;
        cache.clear();

        for jwk in response.keys {
            if let Ok(decoding_key) = DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                cache.insert(jwk.kid, decoding_key);
            }
        }
        Ok(())
    }

    /// Verifies the JWT signature, audience, and issuer, extracting the subject claim.
    pub async fn verify(&self, token: &str) -> Result<String, anyhow::Error> {
        let header = decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow::anyhow!("JWT missing 'kid' in header"))?;

        let mut cache_hit = false;
        {
            let cache = self.key_cache.read().await;
            if cache.contains_key(&kid) {
                cache_hit = true;
            }
        }

        if !cache_hit {
            self.refresh_keys().await?;
        }

        let cache = self.key_cache.read().await;
        let decoding_key = cache
            .get(&kid)
            .ok_or_else(|| anyhow::anyhow!("Unknown 'kid' after JWKS refresh"))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<OidcClaims>(token, decoding_key, &validation)?;
        Ok(token_data.claims.sub)
    }
}
