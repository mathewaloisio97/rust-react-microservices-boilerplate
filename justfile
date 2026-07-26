# Configure the execution shell safely for Windows environments.
set windows-shell := ["cmd.exe", "/c"]

default:
    @just --list

# Run the full validation pipeline locally. (Use `just ci local-dev` to allow dev secrets)
ci mode="prod": check-contracts (check-rust mode) check-frontend

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

# Spins up the database and ensures all schemas are migrated.
db-up:
    docker compose up -d postgres
    @echo Waiting for Postgres to boot...
    timeout 3 >nul 2>&1 || ping -n 4 127.0.0.1 >nul
    @echo Running Identity Migrations...
    cd backend-services/crates/identity && cargo sqlx migrate run
    @echo Running Auth Migrations...
    cd backend-services/crates/auth && cargo sqlx migrate run
    @echo Running Email Migrations...
    cd backend-services/crates/email && cargo sqlx migrate run

# Updates the .sqlx offline caches for all microservices, then stops the DB.
db-prepare: db-up
    @echo Preparing offline cache for Identity...
    cd backend-services/crates/identity && cargo sqlx prepare
    @echo Preparing offline cache for Auth...
    cd backend-services/crates/auth && cargo sqlx prepare
    @echo Preparing offline cache for Email...
    cd backend-services/crates/email && cargo sqlx prepare
    @echo Stopping Postgres...
    docker compose stop postgres
    @echo SUCCESS: Offline caches updated. You can now commit the .sqlx folders!

# Destroys the database volume entirely (Useful if initialization scripts change).
db-clean:
    docker compose down -v
