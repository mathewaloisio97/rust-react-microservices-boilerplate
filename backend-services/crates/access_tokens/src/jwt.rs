//! JWT key management and token issuance engine using RS256 asymmetric signing.
//!
//! Manages cryptographic RSA key pairs, handles public key exports for downstream
//! verification, and signs structured token payloads.

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

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
}

/// Cryptographic manager responsible for loading RSA keys and issuing signed JWTs.
pub struct JwtManager {
    encoding_key: EncodingKey,
    public_key_pem: String,
}

impl JwtManager {
    /// Instantiates a new `JwtManager`, loading a PKCS#8 PEM string or generating an ephemeral 2048-bit keypair.
    pub fn new(private_pem_opt: Option<String>) -> Self {
        if let Some(pem) = private_pem_opt {
            info!("Loading provided RSA Private Key from configuration.");
            let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes())
                .expect("Failed to parse provided RSA PEM");

            Self {
                encoding_key,
                public_key_pem: "PUBLIC_KEY_EXTRACTION_OMITTED".to_string(),
            }
        } else {
            warn!("No Private Key configured. Generating ephemeral 2048-bit RSA keypair.");
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
