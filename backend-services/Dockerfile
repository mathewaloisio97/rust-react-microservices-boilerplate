# --- Builder Stage ---
FROM rust:bookworm AS builder
WORKDIR /usr/src/app

# Accept feature flag from docker-compose.yml (e.g., local-dev)
ARG CARGO_FEATURES=""

# Force SQLx to build queries offline using checked-in .sqlx cache files
ENV SQLX_OFFLINE=true

# Install system dependencies needed for compilation (Protobuf, TLS/OpenSSL, PkgConfig)
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire workspace context (contracts/ + backend-services/)
COPY . .

# Move into backend-services directory where Cargo workspace resides
WORKDIR /usr/src/app/backend-services

# Conditionally compile workspace with features if passed, otherwise standard release
RUN if [ -n "$CARGO_FEATURES" ]; then \
        cargo build --release --workspace --locked --features "$CARGO_FEATURES"; \
    else \
        cargo build --release --workspace --locked; \
    fi

# --- Runtime Stage ---
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime certificates for HTTPS and outgoing connections
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from the target directory
COPY --from=builder /usr/src/app/backend-services/target/release/your_app_gateway ./
COPY --from=builder /usr/src/app/backend-services/target/release/your_app_identity ./
COPY --from=builder /usr/src/app/backend-services/target/release/your_app_auth ./
COPY --from=builder /usr/src/app/backend-services/target/release/your_app_email ./
COPY --from=builder /usr/src/app/backend-services/target/release/your_app_human_verification ./
COPY --from=builder /usr/src/app/backend-services/target/release/your_app_access_tokens ./

# Default fallback entrypoint (overridden by command directive in docker-compose)
CMD ["./your_app_gateway"]
