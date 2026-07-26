//! Data Transfer Objects (DTOs) for identity management workflows.
//!
//! Defines the public-facing JSON schemas and validation contracts for user
//! registration, authentication, and OAuth verification endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request payload containing credentials for traditional email and password registration.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LocalRegisterPayload {
    /// The corporate or personal email address used to create the account.
    #[schema(example = "employee@company.com")]
    pub email: String,

    /// The plain text password secret to be securely hashed down-funnel.
    #[schema(example = "secure_password")]
    pub password: String,
}

impl LocalRegisterPayload {
    /// Ensures that neither the email nor the password strings are blank.
    pub fn is_valid(&self) -> bool {
        !self.email.trim().is_empty() && !self.password.trim().is_empty()
    }
}

/// Request payload containing credentials for traditional email and password authentication.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LocalLoginPayload {
    /// The email address associated with the user account.
    #[schema(example = "employee@company.com")]
    pub email: String,

    /// The plain text password challenge to match against stored credentials.
    #[schema(example = "secure_password")]
    pub password: String,
}

impl LocalLoginPayload {
    /// Ensures that neither the email nor the password credentials are blank.
    pub fn is_valid(&self) -> bool {
        !self.email.trim().is_empty() && !self.password.trim().is_empty()
    }
}

/// Request payload containing tokens and providers for external OAuth authentication.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct OAuthLoginPayload {
    /// The external identity provider name (e.g., 'google', 'microsoft').
    #[schema(example = "google")]
    pub provider: String,

    /// The OIDC identity token or access token provided by the client's login flow.
    #[schema(example = "eyJhbGciOiJSUzI1NiIs...")]
    pub id_token: String,
}

impl OAuthLoginPayload {
    /// Ensures that neither the provider identifier nor the ID token strings are blank.
    pub fn is_valid(&self) -> bool {
        !self.provider.trim().is_empty() && !self.id_token.trim().is_empty()
    }
}

/// Response payload containing the generated session credentials.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthResponseDto {
    /// The generated session or authentication token.
    #[schema(example = "opaque_session_token_xyz")]
    pub token: String,
}

/// Response payload containing the newly registered user's unique identity mapping.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegisterResponseDto {
    /// The canonical Identity UUIDv7 identifier assigned to the user.
    #[schema(example = "0191701c-da51-749a-a4fc-e96579ebc043")]
    pub user_id: String,
}
