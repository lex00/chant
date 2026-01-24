# Chant development commands

# Default recipe - show available commands
default:
    @just --list

# Build the binary
build:
    cargo build

# Build with optimizations
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with coverage
test-coverage:
    cargo llvm-cov --html

# Run linter
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Clean build artifacts
clean:
    cargo clean

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
    cargo install --path .
