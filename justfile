# Chant development commands

# Default recipe - show available commands
default:
    @just --list

# Build the binary
build:
    ~/.cargo/bin/cargo build

# Build with optimizations
build-release:
    ~/.cargo/bin/cargo build --release

# Run tests
test:
    ~/.cargo/bin/cargo test

# Test new developer clone and build experience (slow, manual only)
test-new-dev:
    ~/.cargo/bin/cargo test test_new_developer_experience -- --ignored --nocapture

# Test new user workflow with ollama (requires ollama to be running)
test-ollama:
    ~/.cargo/bin/cargo test test_new_user_workflow_ollama -- --ignored --nocapture

# Run tests with coverage
test-coverage:
    ~/.cargo/bin/cargo llvm-cov --html

# Run linter
lint:
    ~/.cargo/bin/cargo clippy -- -D warnings

# Format code
fmt:
    ~/.cargo/bin/cargo fmt

# Check formatting without modifying
fmt-check:
    ~/.cargo/bin/cargo fmt -- --check

# Clean build artifacts
clean:
    ~/.cargo/bin/cargo clean

# --- Documentation ---

# Build documentation
docs-build:
    cd docs && mdbook build

# Serve documentation locally with live reload
docs-serve:
    cd docs && mdbook serve --open

# Clean documentation build
docs-clean:
    rm -rf docs/book

# --- Development Workflow ---

# Run all checks (format, lint, test)
check: fmt-check lint test

# Full build (check + build)
all: check build

# Install development dependencies
deps:
    @echo "Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    @echo "Install mdbook: cargo install mdbook"
    @echo "Install clippy: rustup component add clippy"
    @echo "Install llvm-cov: cargo install cargo-llvm-cov"

# Install chant locally
install:
    ~/.cargo/bin/cargo install --path .

# Build and run chant with arguments
chant *ARGS:
    ~/.cargo/bin/cargo build
    ./target/debug/chant {{ARGS}}

# --- Documentation Audit ---

# Show doc audit status for all modules
doc-audit-status:
    ./scripts/doc-audit.sh status

# Show only stale modules
doc-audit-stale:
    ./scripts/doc-audit.sh stale

# Check for orphaned mappings (useful after refactors)
doc-audit-orphans:
    ./scripts/doc-audit.sh orphans

# Mark a module as audited
doc-audit-mark MODULE:
    ./scripts/doc-audit.sh mark {{MODULE}}

# Create a spec to audit docs for a module
doc-audit MODULE:
    just chant add "Audit docs for {{MODULE}}" --prompt doc-audit
