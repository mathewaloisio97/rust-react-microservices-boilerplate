FROM rust:1.76-slim as builder
WORKDIR /usr/src/app
# Pre-install protoc for Rust build.rs macros
RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev

# Copy workspace and build
COPY . .
RUN cargo build --release --bin your_app_gateway

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/your_app_gateway /usr/local/bin/
CMD ["your_app_gateway"]
