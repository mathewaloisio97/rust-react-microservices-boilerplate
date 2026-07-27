# Enterprise Rust Microservices & Auth Boilerplate

This repository provides a production-ready, distributed systems template consisting of a decoupled **Rust backend cluster** and a **TypeScript / React frontend**.

Designed as a foundational starter kit for SaaS, E-commerce, and Enterprise Web platforms, the project utilizes a centralized Protobuf pipeline to handle contract generation. This keeps the TypeScript clients and Rust backend microservices perfectly in sync to prevent API breaking changes.

---

## Key Technologies & Architecture

This boilerplate is built on a high-performance, secure-by-default technology stack:

* **Frontend:** React 19, TypeScript, Vite, and React Router.
* **Backend:** Rust, Axum (HTTP Edge Gateway), and Tonic (Internal gRPC communication).
* **Observability & Tracing:** OpenTelemetry (OTLP context propagation across gRPC/HTTP) integrated with Jaeger for centralized distributed tracing.
* **Databases & ORM:** PostgreSQL managed via SQLx (with offline compilation support).
* **Message Broker:** RabbitMQ (AMQP) for decoupled, asynchronous domain events (e.g., welcome emails, session revocations).
* **Authentication & Identity:**
  * **Enterprise SSO:** Built-in OAuth integration (Google, Apple, Facebook) via OIDC cryptographic verification and Graph API introspection.
  * **Local Credentials:** Traditional email/password fallback utilizing Argon2id hashing.
  * **Session Management:** Opaque stateful session tokens exchanged for stateless, short-lived JSON Web Tokens (JWTs) via RS256 asymmetric signing.
* **Bot Protection:** Seamless human verification integrated at the edge, supporting both **Cloudflare Turnstile** and **Google reCAPTCHA (v2/v3/Enterprise)**.
* **Local Development Tools:** Mailpit (local SMTP sink for intercepting verification emails) and a custom Node.js multiplexer for concurrent cluster booting.

---

## Repository Architecture

```text
project-root/
├── .github/workflows/      # CI/CD pipelines enforcing offline validation and testing
├── backend-services/       # Rust microservice workspace (Identity, Gateway, Auth, etc.)
├── contracts/              # Central source of truth for all network schemas (.proto)
├── docker/                 # Infrastructure provisioning scripts (e.g., Postgres initialization)
├── scripts/                # Utility scripts (e.g., local development cluster multiplexer)
├── website/                # React Frontend for the web portal
├── .editorconfig           # Enforces unified cross-platform formatting rules
├── .env.example            # Master runtime configuration template
├── docker-compose.yml      # Local development infrastructure definition
└── justfile                # Root task runner for repository-wide pipeline automation
```

## Infrastructure & Development Philosophy

### CI/CD Parity & Offline Compilation
To guarantee parity between local development machines and GitHub Actions / GitLab CI, the Rust backend enforces **SQLx Offline Mode** via `.cargo/config.toml`. Running a standard compilation (`cargo check` or `cargo build`) requires zero running infrastructure. Database schemas are validated against committed `.sqlx` cache directories, ensuring rapid, deterministic builds.

### Passwordless Local Development
The local PostgreSQL infrastructure utilizes `POSTGRES_HOST_AUTH_METHOD=trust`. This explicitly bypasses password authentication for localized Docker connections. This architectural decision permanently eliminates dummy credentials (e.g., `dev_db_pass`) from the repository's `.env` and `docker-compose.yml` files, preventing false-positive alerts from automated enterprise secret scanners.

### Secure-by-Default Configuration
To prevent the accidental deployment of unsecure development keys into production environments, the cryptographic engine enforces a strict fail-safe mechanism. The default execution path actively monitors for fallback development secrets at boot. If detected, the application will intentionally panic and crash. To utilize localized fallback keys (and official Turnstile/reCAPTCHA dummy testing keys), developers must explicitly authorize the execution by compiling with the `local-dev` feature flag, guaranteeing that production releases remain secure by design.

---

## Developer Setup

The project utilizes `just` as the cross-platform command runner. This abstracts complex build requirements and ensures consistent execution across macOS, Linux, and Windows environments.

### Prerequisites

Ensure the following core runtimes are installed on your host machine:
1. **Python 3.14+**
2. **Rust & Cargo** (Installed via [rustup.rs](https://rustup.rs/))
3. **Docker Desktop** (or an equivalent container daemon)
4. **Node.js 24+**
5. **Protobuf Compiler (`protoc`)** *(Install via `brew install protobuf`, `apt install protobuf-compiler`, or `winget install Google.Protobuf`)*

### Step 1: Install CLI Tooling
Execute the following commands to install the required polyglot build utilities:

```bash
# Install 'just' and 'sqlx-cli' via Cargo
cargo install just
cargo install sqlx-cli --no-default-features --features rustls,postgres

# Install pnpm for deterministic frontend dependency management
npm install -g pnpm
```

### Step 2: Initialize the Environment
Copy the example environment configuration to establish your local routing variables.

```bash
cp .env.example .env
```

### Step 3: Database Preparation & Caching
Whenever a database schema is modified, or when initializing the repository for the first time, you must prepare the offline caches.

```bash
just db-prepare
```
*This command automates the entire infrastructure lifecycle: it boots the local PostgreSQL, RabbitMQ, and Mailpit containers, provisions the isolated logical databases, runs all SQL migrations, updates the `.sqlx` cache directories for the Rust compiler, and cleanly halts the containers.*

### Step 4: Execute the Monorepo Validation Pipeline
To compile the Protobuf contracts into TypeScript interfaces, compile the Rust backend offline, and build the React frontend, run the continuous integration simulation:

```bash
# Standard production-ready validation
just ci

# Localized validation permitting development secrets and dummy testing keys
just ci local-dev
```

### Step 5: Code Formatting
To keep Protobuf contracts, Rust crates, and TypeScript/React code aligned with the repository's style guidelines:

```bash
just format
```

---

## Running the Services Locally

We utilize a custom Node.js multiplexer to boot the entire microservice cluster, frontend, and Docker infrastructure concurrently in a single, color-coordinated terminal feed.

**1. Boot the Entire Development Cluster:**
```bash
just dev
```
*This automatically starts Postgres, RabbitMQ, and Mailpit, applies any pending database migrations, boots all 5 Rust microservices, and starts the Vite React development server.*

**2. Reset the Environment (Optional):**
If you need to wipe your local database data or clear corrupted message queues, completely destroy the docker volumes:
```bash
just db-clean
```
*(Note: Isolated `just db-up` and `just db-down` commands are still available if you only need the infrastructure running without the microservices).*

### Key Local Endpoints
* **React Web Portal:** `http://localhost:5173`
* **Edge Gateway API:** `http://localhost:3000`
* **Swagger OpenAPI Docs:** `http://localhost:3000/swagger-ui`
* **Mailpit (Intercepted Emails):** `http://localhost:8025`
* **RabbitMQ Management UI:** `http://localhost:15672`

---

## Project & Contact Information

* **Author:** Mathew Aloisio
* **Project Purpose:** A general-purpose, enterprise-grade identity and authentication foundation. Features include enterprise OAuth single sign-on, frictionless human verification, automated email pipelines, and secure JWT token issuance. Architecturally, the project demonstrates cross-runtime decoupling between a Rust backend and TypeScript React clients using automated gRPC/Protobuf contract workflows.

### Links
* **Portfolio:** [mathewaloisio.com](https://mathewaloisio.com)
* **LinkedIn:** [linkedin.com/in/mathew-aloisio-594025404](https://www.linkedin.com/in/mathew-aloisio-594025404/)
* **Email:** [mathew.aloisio97@gmail.com](mailto:mathew.aloisio97@gmail.com)