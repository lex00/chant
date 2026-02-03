# Contributing to Chant

Thank you for your interest in contributing to Chant! This document provides guidelines and workflows for contributors.

## Getting Started

### Prerequisites

- Rust toolchain (1.70 or later)
- Git
- mdbook (for documentation)
- `just` command runner (optional, but recommended)

Install dependencies:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install mdbook
cargo install mdbook

# Install just (optional)
cargo install just
```

### Setting Up the Development Environment

1. **Fork and clone the repository:**

```bash
git clone https://github.com/YOUR_USERNAME/chant.git
cd chant
```

2. **Build the project:**

```bash
just build
# or
cargo build
```

3. **Run tests:**

```bash
just test
# or
cargo test
```

4. **Install locally for testing:**

```bash
just install
# or
cargo install --path .
```

## Development Workflow

### Making Changes

1. **Create a feature branch:**

```bash
git checkout -b feature/your-feature-name
```

2. **Make your changes** following the project conventions (see below)

3. **Format your code:**

```bash
just fmt
# or
cargo fmt
```

4. **Run the linter:**

```bash
just lint
# or
cargo clippy -- -D warnings
```

5. **Run tests:**

```bash
just test
# or
cargo test
```

6. **Commit your changes** with a descriptive message:

```bash
git commit -m "Add feature: description of your changes"
```

### Commit Message Guidelines

- Use clear, descriptive commit messages
- Start with a verb in the present tense (e.g., "Add", "Fix", "Update", "Refactor")
- Keep the first line under 72 characters
- Add a blank line and detailed description for complex changes

Examples:
- `Add support for custom spec templates`
- `Fix race condition in worktree cleanup`
- `Update documentation for approval workflow`

### Running Tests

```bash
# Run all tests
just test

# Run tests with coverage
just test-coverage

# Run specific test
cargo test test_name
```

### Building Documentation

```bash
# Build documentation
just docs-build

# Serve documentation locally with live reload
just docs-serve

# Check for broken links
just docs-check-links
```

## Code Style and Conventions

### Rust Code

- Follow standard Rust formatting (`cargo fmt`)
- Address all clippy warnings (`cargo clippy`)
- Use descriptive variable and function names
- Add doc comments for public APIs
- Keep functions focused and concise
- Write tests for new functionality

### Documentation

- Update relevant documentation for new features
- Keep documentation clear and concise
- Include code examples where helpful
- Check for broken links before submitting

### Error Handling

- Use `anyhow::Result` for error propagation
- Provide informative error messages
- Include context when wrapping errors
- Avoid unwrap/expect except in tests or when panic is intended

## Submitting Changes

### Pull Request Process

1. **Ensure all checks pass:**
   - [ ] Code is formatted (`just fmt` or `cargo fmt`)
   - [ ] No clippy warnings (`just lint` or `cargo clippy`)
   - [ ] All tests pass (`just test` or `cargo test`)
   - [ ] Documentation is updated (if needed)

2. **Push your branch:**

```bash
git push origin feature/your-feature-name
```

3. **Create a pull request:**
   - Use a clear, descriptive title
   - Fill out the PR template
   - Reference any related issues
   - Describe what changed and why

4. **Address review feedback:**
   - Respond to comments promptly
   - Make requested changes
   - Push updates to your branch (PR will update automatically)

### PR Review

- Maintainers will review your PR when possible
- Reviews may request changes or ask questions
- CI checks must pass before merging
- At least one maintainer approval is required

## Project Structure

```
chant/
├── src/               # Source code
│   ├── main.rs       # CLI entry point
│   ├── lib.rs        # Library entry point
│   ├── commands/     # CLI commands
│   ├── core/         # Core functionality
│   └── utils/        # Utility modules
├── docs/             # Documentation (mdbook)
├── examples/         # Example workflows
├── .chant/           # Chant spec directory
│   ├── specs/        # Active specs
│   └── archive/      # Archived specs
└── tests/            # Integration tests
```

## Testing

### Types of Tests

- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test complete workflows in `tests/`
- **Documentation tests**: Examples in doc comments

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = "test";

        // Act
        let result = your_function(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Test Organization

- Place unit tests in the same file as the code they test
- Use `#[cfg(test)]` module for unit tests
- Use `tests/` directory for integration tests
- Use `serial_test` crate for tests that can't run in parallel

## Documentation

### Code Documentation

- Add doc comments (`///`) for all public items
- Include examples in doc comments where helpful
- Document parameters, return values, and errors
- Keep documentation up-to-date with code changes

### User Documentation

User documentation lives in `docs/` and is built with mdbook:

- **Getting Started**: Installation and quickstart guides
- **User Guide**: Feature documentation and tutorials
- **Reference**: Command reference and API docs
- **Development**: Architecture and contributor docs

## Useful Commands

```bash
# Build and test
just build              # Build the binary
just test              # Run tests
just check             # Run all checks (format, lint, test)

# Code quality
just fmt               # Format code
just lint              # Run clippy

# Documentation
just docs-serve        # Serve docs locally
just docs-check-links  # Check for broken links

# Release
just install           # Install chant locally
just chant ARGS        # Build and run chant
```

## Getting Help

- **Documentation**: https://lex00.github.io/chant
- **Issues**: https://github.com/lex00/chant/issues
- **Discussions**: Use GitHub Discussions for questions

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Security

Please report security vulnerabilities according to our [Security Policy](SECURITY.md). Do not report security issues through public GitHub issues.

## License

By contributing to Chant, you agree that your contributions will be licensed under the Apache-2.0 License.
