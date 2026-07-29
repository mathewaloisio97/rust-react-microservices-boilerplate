//! gRPC network interface implementation for processing human verification requests.

use crate::providers::{
    recaptcha::RecaptchaProvider, turnstile::TurnstileProvider, VerificationProvider,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use your_app_contracts::human_verification::v1::human_verification_service_server::HumanVerificationService;
use your_app_contracts::human_verification::v1::{
    GetChallengeRequest, GetChallengeResponse, VerifyRequest, VerifyResponse,
};
use your_app_human_verification_crypto::CryptoEngine;

pub struct VerificationGrpcServer {
    pub crypto: Arc<CryptoEngine>,
    pub token_timeout_secs: u64,
    pub recaptcha: Option<Arc<RecaptchaProvider>>,
    pub turnstile: Option<Arc<TurnstileProvider>>,
}

#[tonic::async_trait]
impl HumanVerificationService for VerificationGrpcServer {
    async fn get_challenge(
        &self,
        _req: Request<GetChallengeRequest>,
    ) -> Result<Response<GetChallengeResponse>, Status> {
        // Standard providers (Turnstile/reCAPTCHA) do not require backend generation.
        // Returning an empty JSON object. This is a placeholder for future custom captchas.
        Ok(Response::new(GetChallengeResponse {
            challenge_payload: "{}".to_string(),
        }))
    }

    async fn verify(
        &self,
        req: Request<VerifyRequest>,
    ) -> Result<Response<VerifyResponse>, Status> {
        let inner = req.into_inner();
        let provider_id = inner.provider_id.to_lowercase();
        let token = inner.client_payload;

        let provider: Arc<dyn VerificationProvider> = match provider_id.as_str() {
            "recaptcha" => {
                if let Some(p) = &self.recaptcha {
                    p.clone()
                } else {
                    return Err(Status::unimplemented(
                        "reCAPTCHA provider is not configured.",
                    ));
                }
            }
            "turnstile" => {
                if let Some(p) = &self.turnstile {
                    p.clone()
                } else {
                    return Err(Status::unimplemented(
                        "Turnstile provider is not configured.",
                    ));
                }
            }
            _ => {
                return Err(Status::invalid_argument(format!(
                    "Unknown provider: {provider_id}"
                )))
            }
        };

        match provider.verify(&token).await {
            Ok(true) => {
                info!("Human verification passed via [{}]", provider_id);
                let voucher = self
                    .crypto
                    .generate_signed_voucher(self.token_timeout_secs)
                    .map_err(|e| {
                        error!("Cryptography fault: {:?}", e);
                        Status::internal("Internal cryptography fault")
                    })?;

                Ok(Response::new(VerifyResponse {
                    success: true,
                    voucher,
                }))
            }
            Ok(false) => {
                info!("Human verification failed/rejected by [{}]", provider_id);
                Ok(Response::new(VerifyResponse {
                    success: false,
                    voucher: "".to_string(),
                }))
            }
            Err(e) => {
                error!("External API fault during captcha verification: {:?}", e);
                Err(Status::internal("Verification provider API fault"))
            }
        }
    }
}
