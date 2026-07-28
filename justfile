# Configure the execution shell safely for Windows environments.
set windows-shell := ["cmd.exe", "/c"]

default:
    @just --list

# Database connection strings targeting host-mapped 127.0.0.1:5432
IDENTITY_DB_URL := "postgres://postgres:postgres@127.0.0.1:5432/your_app_identity?sslmode=disable"
AUTH_DB_URL := "postgres://postgres:postgres@127.0.0.1:5432/your_app_auth?sslmode=disable"
EMAIL_DB_URL := "postgres://postgres:postgres@127.0.0.1:5432/your_app_email?sslmode=disable"

# Run the full validation pipeline locally.
ci mode="prod": check-contracts (check-rust mode) check-frontend

# Auto-format all subsystems across the repository.
format:
    cd contracts && just format
    cd backend-services && cargo fmt
    cd website && pnpm run format

# Compile Protobuf schemas into TypeScript interfaces.
check-contracts:
    cd contracts && just build

# Format, lint, and build the Rust backend.
check-rust mode="prod":
    cd backend-services && cargo fmt -- --check
    cd backend-services && cargo clippy --all-targets --all-features -- -D warnings
    cd backend-services && cargo build --release {{ if mode == "local-dev" { "--features local-dev" } else { "" } }}

# Install dependencies and build the React portal.
check-frontend:
    cd website && pnpm install --frozen-lockfile
    cd website && pnpm run format:check
    cd website && pnpm run lint
    cd website && pnpm build

# --- Developer Database & SQLx Utilities ---

# Spins up the infrastructure and waits reliably for PostgreSQL health.
db-up:
    docker compose up -d postgres rabbitmq mailpit jaeger
    @echo Waiting for PostgreSQL to accept connections...
    for /L %i in (1,1,30) do @(docker exec your_app-postgres pg_isready -U postgres >nul 2>&1 && (echo PostgreSQL is alive... && exit /b 0) || ping -n 2 127.0.0.1 >nul)
    @echo Waiting for database engine to settle...
    ping -n 3 127.0.0.1 >nul
    @echo Ensuring databases exist...
    cd backend-services/crates/identity && cargo sqlx database create --database-url {{ IDENTITY_DB_URL }}
    cd backend-services/crates/auth && cargo sqlx database create --database-url {{ AUTH_DB_URL }}
    cd backend-services/crates/email && cargo sqlx database create --database-url {{ EMAIL_DB_URL }}
    @echo Running Identity Migrations...
    cd backend-services/crates/identity && cargo sqlx migrate run --database-url {{ IDENTITY_DB_URL }}
    @echo Running Auth Migrations...
    cd backend-services/crates/auth && cargo sqlx migrate run --database-url {{ AUTH_DB_URL }}
    @echo Running Email Migrations...
    cd backend-services/crates/email && cargo sqlx migrate run --database-url {{ EMAIL_DB_URL }}

# Gracefully stops all infrastructure containers without deleting volume data.
db-down:
    @echo Stopping dev infrastructure containers...
    docker compose stop postgres rabbitmq mailpit jaeger

# Updates the .sqlx offline caches for all microservices, then stops the DB.
db-prepare: db-up
    @echo Preparing offline cache for Identity...
    cd backend-services/crates/identity && cargo sqlx prepare --database-url {{ IDENTITY_DB_URL }}
    @echo Preparing offline cache for Auth...
    cd backend-services/crates/auth && cargo sqlx prepare --database-url {{ AUTH_DB_URL }}
    @echo Preparing offline cache for Email...
    cd backend-services/crates/email && cargo sqlx prepare --database-url {{ EMAIL_DB_URL }}
    @echo Stopping infrastructure...
    just db-down
    @echo SUCCESS: Offline caches updated. You can now commit the .sqlx folders!

# Destroys the database volume entirely.
db-clean:
    docker compose down -v

# Complete reset: wipes DB volumes, applies fresh migrations, and re-prepares .sqlx
db-rebuild: db-clean db-prepare

# Boots infrastructure and runs full stack concurrently.
dev *args: db-up
    node --env-file=.env scripts/dev.js {{args}}

