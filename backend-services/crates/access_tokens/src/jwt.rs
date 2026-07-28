//! JWT key management and token issuance engine using RS256 asymmetric signing.
//!
//! Manages cryptographic RSA key pairs, handles public key exports for downstream
//! verification, and signs structured token payloads.

#[cfg(feature = "local-dev")]
use tracing::warn;

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::{
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Structured JWT claims payload representing authenticated identity and granted roles.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject identifier (user UUID).
    pub sub: String,

    /// Session identifier backing the token.
    pub sid: String,

    /// Token issuer authority domain or identifier.
    pub iss: String,

    /// Expiration timestamp in Unix epoch seconds.
    pub exp: usize,

    /// Issuance timestamp in Unix epoch seconds.
    pub iat: usize,

    /// List of assigned permission roles or scopes.
    pub roles: Vec<String>,

    /// The administrative access level granted to the user.
    pub access_level: String,
}

/// Cryptographic manager responsible for loading RSA keys and issuing signed JWTs.
pub struct JwtManager {
    encoding_key: EncodingKey,
    public_key_pem: String,
}

impl JwtManager {
    /// Instantiates a new `JwtManager`, loading a PKCS#8 PEM string or generating an ephemeral 2048-bit keypair.
    ///
    /// In `local-dev` environments, if no key is provided, it generates and caches
    /// a persistent RSA keypair locally to ensure JWTs survive backend restarts.
    pub fn new(private_pem_opt: Option<String>) -> Self {
        #[allow(unused_mut)]
        let mut final_pem = private_pem_opt;

        #[cfg(feature = "local-dev")]
        if final_pem.is_none() {
            let key_path = ".dev_jwt_private_key.pem";
            if let Ok(cached_pem) = std::fs::read_to_string(key_path) {
                info!(
                    "JWT: Loaded persistent development RSA key from {}.",
                    key_path
                );
                final_pem = Some(cached_pem);
            } else {
                warn!("JWT: Generating new persistent RSA keypair for local development...");
                let mut rng = rand::rng();
                let priv_key =
                    RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate RSA key");
                let pem = priv_key.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
                std::fs::write(key_path, &pem).expect("Failed to write dev RSA key");
                final_pem = Some(pem);
            }
        }

        if let Some(pem) = final_pem {
            info!("JWT: Initializing with configured RSA Private Key.");

            // Parse the PKCS#8 PEM to mathematically derive the Public Key for exportation.
            let priv_key = RsaPrivateKey::from_pkcs8_pem(&pem)
                .expect("FATAL: Failed to parse the provided RSA Private Key PEM");
            let pub_key = RsaPublicKey::from(&priv_key);

            let public_key_pem = pub_key.to_public_key_pem(LineEnding::LF).unwrap();
            let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();

            Self {
                encoding_key,
                public_key_pem,
            }
        } else {
            // Failsafe for production if the environment variable is missing.
            tracing::error!(
                "FATAL: No Private Key configured! Generating ephemeral 2048-bit RSA keypair."
            );
            let mut rng = rand::rng();
            let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate RSA key");
            let pub_key = RsaPublicKey::from(&priv_key);

            let priv_pem = priv_key.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
            let pub_pem = pub_key.to_public_key_pem(LineEnding::LF).unwrap();

            let encoding_key = EncodingKey::from_rsa_pem(priv_pem.as_bytes()).unwrap();

            Self {
                encoding_key,
                public_key_pem: pub_pem,
            }
        }
    }

    /// Encodes and signs the provided claims using RS256.
    pub fn issue(&self, claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
        let header = Header::new(Algorithm::RS256);
        encode(&header, claims, &self.encoding_key)
    }

    /// Returns the active public key formatted in PEM syntax.
    pub fn get_public_key_pem(&self) -> String {
        self.public_key_pem.clone()
    }
}
