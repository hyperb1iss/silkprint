# SilkPrint — Development Tasks
# Install just: cargo install just

set dotenv-load := false

# Show available recipes
default:
    @just --list --unsorted

# ── Rust ──────────────────────────────────────────────────────

# Run format check + clippy
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format all Rust code
fmt:
    cargo fmt --all

# Run tests with cargo-nextest
test:
    cargo nextest run --workspace

# Run doc tests
test-doc:
    cargo test --doc --workspace

# Run all tests (nextest + doc)
test-all: test test-doc

# Build CLI in release mode
build:
    cargo build --release --locked

# Run the CLI with arguments
run *ARGS:
    cargo run -- {{ ARGS }}

# ── WASM ──────────────────────────────────────────────────────

# Build WASM module and install bindings into web/
wasm:
    cargo build --release --locked -p silkprint-wasm --target wasm32-unknown-unknown
    mkdir -p web/src/lib/wasm web/public/wasm
    wasm-bindgen --out-dir web/src/lib/wasm --target web \
        target/wasm32-unknown-unknown/release/silkprint_wasm.wasm
    mv web/src/lib/wasm/silkprint_wasm_bg.wasm web/public/wasm/

# ── Web ───────────────────────────────────────────────────────

# Install web dependencies
web-install:
    cd web && pnpm install

# Start the web dev server (port 7455)
web-dev:
    cd web && pnpm dev

# Build the web app for production
web-build:
    cd web && pnpm build

# Lint web app (Biome)
web-lint:
    cd web && pnpm lint

# Fix web lint issues
web-fix:
    cd web && pnpm lint:fix

# Typecheck web app
web-typecheck:
    cd web && pnpm typecheck

# ── Security ──────────────────────────────────────────────────

# Run cargo-deny supply-chain audit
deny:
    cargo deny check

# ── Full Pipeline ─────────────────────────────────────────────

# Run the full CI pipeline locally
ci: check test-all deny wasm web-typecheck web-lint web-build
