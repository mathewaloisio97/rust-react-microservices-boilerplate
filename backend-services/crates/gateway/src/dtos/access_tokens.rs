//! Data Transfer Objects (DTOs) for stateless JWT access token endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request payload to mint an access token with specific permissions and lifetime limits.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct IssueTokenPayload {
    /// Granted permission roles or scopes.
    pub roles: Vec<String>,

    /// Requested token lifetime in seconds.
    pub ttl_seconds: u32,
}

/// Response payload containing the signed JWT and its expiration timestamp.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct IssueTokenResponse {
    /// Cryptographically signed JSON Web Token string.
    pub access_token: String,

    /// Unix epoch timestamp in seconds indicating expiration.
    pub expires_at: u64,
}

/// Response payload exposing the public RSA key for offline signature verification.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetPublicKeyResponse {
    /// Public key formatted in PEM syntax.
    pub public_key_pem: String,
}
