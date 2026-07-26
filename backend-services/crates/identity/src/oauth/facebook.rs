//! Facebook Graph API Verification Engine.
//!
//! Handles token introspection via the Facebook Graph API, as Facebook does
//! not strictly adhere to standard OIDC JWT signatures for client-side tokens.

use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct FbDebugData {
    is_valid: bool,
    app_id: String,
    user_id: String,
}

#[derive(Deserialize)]
struct FbDebugResponse {
    data: FbDebugData,
}

/// Facebook OAuth Provider supporting remote token introspection.
pub struct FacebookProvider {
    app_id: String,
    app_secret: String,
    http_client: Client,
}

impl FacebookProvider {
    /// Instantiates a new Facebook Graph API verification provider.
    pub fn new(app_id: String, app_secret: String) -> Self {
        Self {
            app_id,
            app_secret,
            http_client: Client::new(),
        }
    }

    /// Calls the Graph API debug_token endpoint to definitively validate the client token.
    pub async fn verify(&self, token: &str) -> Result<String, anyhow::Error> {
        let url = format!(
            "https://graph.facebook.com/debug_token?input_token={}&access_token={}|{}",
            token, self.app_id, self.app_secret
        );

        let response: FbDebugResponse = self.http_client.get(&url).send().await?.json().await?;

        if !response.data.is_valid {
            return Err(anyhow::anyhow!(
                "Facebook token is flagged invalid by Graph API"
            ));
        }

        if response.data.app_id != self.app_id {
            return Err(anyhow::anyhow!(
                "Facebook token was not issued for this application"
            ));
        }

        Ok(response.data.user_id)
    }
}
