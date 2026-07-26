//! Cryptographic token utility for signing and verifying transient application state.
//!
//! This module provides the `CryptoEngine` which serializes payloads, wraps them with absolute
//! expiration timestamps, and secures them using HMAC-SHA256 signatures. Resulting tokens are
//! formatted as URL-safe, unpadded Base64 strings separated by a period (`payload.signature`).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Data payload for temporary access tokens or single-use vouchers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedVoucher {
    /// Internal lifecycle tracker. Note that outer token expiration is handled by SignedState.
    pub valid_until: u64,
}

/// Wrapper that attaches an expiration timestamp to any serializable payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedState<T> {
    pub valid_until: u64,
    pub data: T,
}

/// Handles cryptographic signing and verification of state payloads using HMAC-SHA256.
#[derive(Clone)]
pub struct CryptoEngine {
    secret_key: Vec<u8>,
}

impl CryptoEngine {
    /// Creates a new engine instance with the provided secret key.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret_key: secret.to_vec(),
        }
    }

    /// Wraps a payload with an expiration time, generates an HMAC signature,
    /// and returns a URL-safe token format: `payload_b64.signature_b64`.
    pub fn sign_state<T: Serialize>(
        &self,
        data: &T,
        ttl_seconds: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Calculate absolute expiration timestamp.
        let valid_until = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + ttl_seconds;
        let state = SignedState { valid_until, data };

        // Serialize payload and generate HMAC.
        let serialized = serde_json::to_string(&state)?;
        let mut mac = HmacSha256::new_from_slice(&self.secret_key)?;
        mac.update(serialized.as_bytes());
        let signature = mac.finalize().into_bytes();

        // Encode components into URL-safe base64 without padding.
        let payload_b64 = URL_SAFE_NO_PAD.encode(serialized.as_bytes());
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature);

        Ok(format!("{}.{}", payload_b64, signature_b64))
    }

    /// Decodes, verifies, and unpacks a token.
    /// Returns `None` if the token is malformed, the signature is invalid, or the payload has expired.
    pub fn verify_state<T: for<'de> Deserialize<'de>>(&self, token: &str) -> Option<T> {
        // Token must contain exactly two parts: [payload, signature].
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 2 {
            return None;
        }

        // Decode both sections from base64.
        let decoded_payload_bytes = URL_SAFE_NO_PAD.decode(parts[0]).ok()?;
        let serialized_payload = String::from_utf8(decoded_payload_bytes).ok()?;
        let provided_sig = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;

        // Recompute signature and verify it matches.
        let mut mac = HmacSha256::new_from_slice(&self.secret_key).ok()?;
        mac.update(serialized_payload.as_bytes());

        if mac.verify_slice(&provided_sig).is_err() {
            return None;
        }

        // Deserialize payload and validate expiration.
        let state: SignedState<T> = serde_json::from_str(&serialized_payload).ok()?;
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

        if current_time > state.valid_until {
            return None;
        }

        Some(state.data)
    }

    /// Convenience wrapper to generate a signed voucher token with a specific TTL.
    pub fn generate_signed_voucher(
        &self,
        ttl_seconds: u64,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let voucher = VerifiedVoucher { valid_until: 0 };
        self.sign_state(&voucher, ttl_seconds)
    }

    /// Convenience wrapper to verify a signed voucher token.
    pub fn verify_voucher(&self, token: &str) -> Option<VerifiedVoucher> {
        self.verify_state::<VerifiedVoucher>(token)
    }
}
