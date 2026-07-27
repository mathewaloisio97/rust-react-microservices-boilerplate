//! OpenTelemetry and Distributed Tracing configuration.
//!
//! Provides a standardized bootstrapper to initialize the OTLP pipeline
//! and strongly typed middleware layers to transparently propagate trace
//! contexts across gRPC network boundaries.

use http::{HeaderMap, Request};
use opentelemetry::{global, propagation::Extractor, propagation::Injector, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Initializes the global tracing subscriber and OpenTelemetry OTLP export pipeline.
///
/// Sets up a global W3C [`TraceContextPropagator`], connects an OTLP trace exporter
/// via gRPC using Tonic, registers service resource attributes, and wires up both
/// a human-readable stdout logger and a tracing-opentelemetry layer.
///
/// # Arguments
///
/// * `service_name` - The logical identifier for this service (e.g., `"access_tokens"`).
/// * `otlp_endpoint` - The target gRPC URI for the OTLP collector (e.g., `"http://localhost:4317"`).
///
/// # Errors
///
/// Returns an error if the OTLP pipeline fails to install or if the global tracing subscriber
/// has already been initialized.
///
/// # Example
///
/// ```ignore
/// telemetry::init_telemetry("my_service", "http://localhost:4317")?;
/// ```
pub fn init_telemetry(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Build the OTLP span exporter targeting the specified endpoint over gRPC (Tonic).
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()?;

    // Define the resource attributes (e.g., service name) identifying this instance.
    let resource = Resource::builder()
        .with_attributes([KeyValue::new("service.name", service_name.to_string())])
        .build();

    // Construct the SDK Tracer Provider with a batch exporter for asynchronous processing.
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());

    // Instantiate an OpenTelemetry tracing subscriber layer bound to our provider.
    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "telemetry");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true);

    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .with(telemetry_layer)
        .try_init()?;

    Ok(())
}

// ============================================================================
// gRPC Outbound Client Middleware (Gateway / Access Tokens)
// ============================================================================

/// A strongly-typed alias for a Tonic transport channel wrapped in OpenTelemetry
/// context injection middleware.
///
/// Use this type when defining client structs or traits that wrap an instrumented channel.
pub type InstrumentedChannel = OtelGrpcClientService<tonic::transport::Channel>;

/// Wraps a standard Tonic transport channel in our OpenTelemetry context injection middleware.
///
/// Every request passing through the returned [`InstrumentedChannel`] will automatically
/// extract W3C trace context headers (`traceparent`) from the active `tracing` span and inject
/// them into the outgoing gRPC metadata.
///
/// # Arguments
///
/// * `channel` - An initialized [`tonic::transport::Channel`].
///
/// # Example
///
/// ```ignore
/// let channel = tonic::transport::Endpoint::from_static("[http://[::1]:50051](http://[::1]:50051)").connect().await?;
/// let instrumented = telemetry::instrument_channel(channel);
/// let mut client = MyServiceClient::new(instrumented);
/// ```
pub fn instrument_channel(channel: tonic::transport::Channel) -> InstrumentedChannel {
    OtelGrpcClientService { inner: channel }
}

/// Tower [`Service`] wrapper that injects active tracing span context headers into
/// outgoing HTTP/gRPC client requests.
#[derive(Debug, Clone)]
pub struct OtelGrpcClientService<S> {
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for OtelGrpcClientService<S>
where
    S: Service<Request<ReqBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        global::get_text_map_propagator(|propagator| {
            let context = tracing::Span::current().context();
            let mut injector = HeaderInjector(req.headers_mut());
            propagator.inject_context(&context, &mut injector);
        });
        self.inner.call(req)
    }
}

/// Helper to adapt an `http::HeaderMap` into OpenTelemetry's [`Injector`] interface.
struct HeaderInjector<'a>(&'a mut HeaderMap);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(k) = http::header::HeaderName::from_bytes(key.to_lowercase().as_bytes()) {
            if let Ok(v) = http::header::HeaderValue::from_str(&value) {
                self.0.insert(k, v);
            }
        }
    }
}

// ============================================================================
// gRPC Inbound Server Middleware (Backend Microservices)
// ============================================================================

/// Tower [`Layer`] that intercepts incoming gRPC HTTP requests, extracts remote
/// OpenTelemetry trace context (`traceparent`), and links subsequent processing
/// within an active tracing span.
///
/// # Example
///
/// ```ignore
/// Server::builder()
///     .layer(OtelGrpcLayer)
///     .add_service(MyServiceServer::new(service_impl))
///     .serve(addr)
///     .await?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct OtelGrpcLayer;

impl<S> Layer<S> for OtelGrpcLayer {
    type Service = OtelGrpcService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OtelGrpcService { inner }
    }
}

/// Tower [`Service`] generated by [`OtelGrpcLayer`] to handle contextual trace extraction
/// on incoming requests.
#[derive(Debug, Clone)]
pub struct OtelGrpcService<S> {
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for OtelGrpcService<S>
where
    S: Service<Request<ReqBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = tracing::instrument::Instrumented<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let parent_cx =
            global::get_text_map_propagator(|prop| prop.extract(&HeaderExtractor(req.headers())));

        let span = tracing::info_span!("grpc_request", uri = %req.uri());
        span.set_parent(parent_cx);

        self.inner.call(req).instrument(span)
    }
}

/// Helper to adapt an `http::HeaderMap` into OpenTelemetry's [`Extractor`] interface.
struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
