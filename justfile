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

# Check documentation for broken links
docs-check-links:
    #!/usr/bin/env bash
    set -e
    echo "Building docs..."
    cd docs && mdbook build 2>&1
    echo "Build succeeded. Checking SUMMARY.md link references..."
    broken=0
    while IFS= read -r f; do
        if [ ! -f "$f" ]; then
            echo "BROKEN: SUMMARY.md -> $f"
            broken=$((broken + 1))
        fi
    done < <(grep -o '([^)]*\.md)' SUMMARY.md | tr -d '()')
    if [ "$broken" -eq 0 ]; then
        echo "All SUMMARY.md links are valid."
    else
        echo "$broken broken link(s) found."
        exit 1
    fi

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
    ~/.cargo/bin/cargo build
    ./target/debug/chant add "Audit docs for {{MODULE}}" --prompt doc-audit

# --- Release ---

# Release a new version (e.g., just release 0.2.0)
release VERSION:
    #!/usr/bin/env bash
    set -e

    # Validate version format (X.Y.Z)
    if ! [[ "{{VERSION}}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo "Error: Version must be in X.Y.Z format (e.g., 0.2.0)"
        exit 1
    fi

    # Check for dirty working directory
    if ! git diff --quiet; then
        echo "Error: Working directory has uncommitted changes"
        exit 1
    fi

    if ! git diff --cached --quiet; then
        echo "Error: Staging area has uncommitted changes"
        exit 1
    fi

    # Check if tag already exists
    if git tag -l "v{{VERSION}}" | grep -q .; then
        echo "Error: Tag v{{VERSION}} already exists"
        exit 1
    fi

    echo "Releasing version {{VERSION}}..."

    # Update version in Cargo.toml
    sed -i '' 's/^version = ".*"/version = "{{VERSION}}"/' Cargo.toml
    echo "✓ Updated Cargo.toml to version {{VERSION}}"

    # Build release to update Cargo.lock
    ~/.cargo/bin/cargo build --release
    echo "✓ Built release binary and updated Cargo.lock"

    # Commit changes
    git add Cargo.toml Cargo.lock
    git commit -m "Release v{{VERSION}}"
    echo "✓ Created commit: Release v{{VERSION}}"

    # Create annotated tag
    git tag -a "v{{VERSION}}" -m "Release v{{VERSION}}"
    echo "✓ Created annotated tag v{{VERSION}}"

    # Push commits and tags
    git push origin main
    git push origin "v{{VERSION}}"
    echo "✓ Pushed commits and tags to origin"

    echo ""
    echo "========================================="
    echo "Release v{{VERSION}} complete!"
    echo "========================================="
    echo ""
    echo "Next steps:"
    echo "1. Wait for GitHub Actions to build and publish releases"
    echo "2. Update homebrew formula with new SHA256 hashes from releases"
    echo "3. Run: brew audit --strict chant-dev/chant/chant"
    echo ""
