//! Gateway API main server entry point.
//!
//! Sets up our central web server by connecting to backend services,
//! creating our shared application state, configuring interactive API
//! documentation (Swagger UI), and starting the network listener.

#[cfg(feature = "local-dev")]
use tower_http::cors::Any;

#[cfg(not(feature = "local-dev"))]
use axum::http::HeaderValue;

use axum::http::{header, Method};
use axum::Router;
use std::env;
use std::net::SocketAddr;
use tonic::transport::Channel;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;
use your_app_constants::security::DEFAULT_HV_SECRET;
use your_app_contracts::access_tokens::v1::access_tokens_service_client::AccessTokensServiceClient;
use your_app_contracts::auth::v1::auth_service_client::AuthServiceClient;
use your_app_contracts::email::v1::email_service_client::EmailServiceClient;
use your_app_contracts::human_verification::v1::human_verification_service_client::HumanVerificationServiceClient;
use your_app_contracts::identity::v1::identity_service_client::IdentityServiceClient;
use your_app_gateway::{dtos, handlers, routes, state::AppState};

/// Tells Swagger UI how to display and accept our Bearer token login security format.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("Opaque")
                    .build(),
            ),
        );
    }
}

/// Builds our interactive API specification docs from existing code paths and payload structures.
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::identity::local_register,
        handlers::identity::local_login,
        handlers::identity::oauth_login,
        handlers::auth::logout,
        handlers::human_verification::get_challenge,
        handlers::human_verification::verify,
        handlers::email::get_email,
        handlers::email::set_email,
        handlers::email::verify_email,
        handlers::access_tokens::issue_token,
        handlers::access_tokens::get_public_key
    ),
    components(schemas(
        dtos::LocalRegisterPayload,
        dtos::RegisterResponseDto,
        dtos::LocalLoginPayload,
        dtos::AuthResponseDto,
        dtos::OAuthLoginPayload,
        dtos::ClientVerifyPayload,
        dtos::EmailStateResponse,
        dtos::SetEmailPayload,
        dtos::SetEmailResponse,
        dtos::VerifyEmailPayload,
        dtos::VerifyEmailResponse,
        dtos::IssueTokenPayload,
        dtos::IssueTokenResponse,
        dtos::GetPublicKeyResponse
    )),
    modifiers(&SecurityAddon),
    tags((name = "YourApp Edge Gateway", description = "YourApp REST API"))
)]
struct ApiDoc;

/// Starts the web server loop.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry distributed tracing and logging.
    let otlp_endpoint =
        env::var("OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
    your_app_telemetry::init_telemetry("your_app_gateway", &otlp_endpoint)?;

    // Ingest configuration from the environment, falling back to local dev defaults.
    let identity_url =
        env::var("IDENTITY_URL").unwrap_or_else(|_| "http://localhost:50051".to_string());
    let auth_url = env::var("AUTH_URL").unwrap_or_else(|_| "http://localhost:50052".to_string());
    let hv_url =
        env::var("HUMAN_VERIFICATION_URL").unwrap_or_else(|_| "http://localhost:50055".to_string());
    let hv_secret =
        env::var("HUMAN_VERIFICATION_SECRET").unwrap_or_else(|_| DEFAULT_HV_SECRET.to_string());
    let email_url = env::var("EMAIL_URL").unwrap_or_else(|_| "http://localhost:50053".to_string());
    let access_tokens_url =
        env::var("ACCESS_TOKENS_URL").unwrap_or_else(|_| "http://localhost:50054".to_string());
    let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    // Verify the integrity of the cryptographic validation keys.
    if hv_secret == DEFAULT_HV_SECRET {
        #[cfg(feature = "local-dev")]
        {
            tracing::warn!("===========================================================");
            tracing::warn!("SECURITY ALERT: Running with the default development secret");
            tracing::warn!("===========================================================");
        }

        #[cfg(not(feature = "local-dev"))]
        {
            tracing::error!(
                "FATAL: Insecure fallback secret detected without local-dev authorization!"
            );
            panic!("Insecure configuration payload blocked.");
        }
    }

    // Establish instrumented channels to the backend microservices.
    // This injects OpenTelemetry headers automatically on outgoing requests.
    info!("Connecting to Identity Subsystem at {}...", identity_url);
    let identity_channel =
        your_app_telemetry::instrument_channel(Channel::from_shared(identity_url)?.connect_lazy());

    info!("Connecting to Auth Subsystem at {}...", auth_url);
    let auth_channel =
        your_app_telemetry::instrument_channel(Channel::from_shared(auth_url)?.connect_lazy());

    info!(
        "Connecting to Human Verification Subsystem at {}...",
        hv_url
    );
    let hv_channel =
        your_app_telemetry::instrument_channel(Channel::from_shared(hv_url)?.connect_lazy());

    info!("Connecting to Email Subsystem at {}...", email_url);
    let email_channel =
        your_app_telemetry::instrument_channel(Channel::from_shared(email_url)?.connect_lazy());

    info!(
        "Connecting to Access Tokens Subsystem at {}...",
        access_tokens_url
    );
    let access_tokens_channel = your_app_telemetry::instrument_channel(
        Channel::from_shared(access_tokens_url)?.connect_lazy(),
    );

    let crypto_engine = your_app_human_verification_crypto::CryptoEngine::new(hv_secret.as_bytes());

    let state = AppState {
        identity_client: IdentityServiceClient::new(identity_channel),
        auth_client: AuthServiceClient::new(auth_channel),
        human_verification_client: HumanVerificationServiceClient::new(hv_channel),
        email_client: EmailServiceClient::new(email_channel),
        access_tokens_client: AccessTokensServiceClient::new(access_tokens_channel),
        crypto_engine,
    };

    let cors_base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            "x-captcha-voucher".parse().unwrap(),
        ]);

    #[cfg(feature = "local-dev")]
    let cors = {
        tracing::warn!("CORS: Running in local-dev mode. Allowing all origins.");
        cors_base.allow_origin(Any)
    };

    #[cfg(not(feature = "local-dev"))]
    let cors = {
        let origins_str = env::var("ALLOWED_CORS_ORIGINS")
            .expect("FATAL: ALLOWED_CORS_ORIGINS environment variable must be set in production for CORS constraints.");

        let origins: Vec<HeaderValue> = origins_str
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let clean_str = s.trim();
                clean_str.parse::<HeaderValue>().unwrap_or_else(|_| {
                    panic!(
                        "FATAL: Invalid origin format in ALLOWED_CORS_ORIGINS: {}",
                        clean_str
                    )
                })
            })
            .collect();

        tracing::info!("CORS: Bound to exact origins: [{}]", origins_str);
        cors_base.allow_origin(origins)
    };

    let openapi = ApiDoc::openapi();

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi))
        .merge(routes::identity::build_router(state.clone()))
        .merge(routes::auth::build_router(state.clone()))
        .merge(routes::human_verification::build_router(state.clone()))
        .merge(routes::email::build_router(state.clone()))
        .merge(routes::access_tokens::build_router(state.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = server_addr.parse()?;
    info!("Gateway REST API online at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
