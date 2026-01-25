<img src="docs/assets/chant-logo.svg" alt="Chant" width="120" align="right">

# Chant

**Idempotent Intention**

[![CI](https://github.com/lex00/chant/actions/workflows/ci.yml/badge.svg)](https://github.com/lex00/chant/actions/workflows/ci.yml)
[![Release](https://github.com/lex00/chant/actions/workflows/release.yml/badge.svg)](https://github.com/lex00/chant/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Latest Release](https://img.shields.io/github/v/release/lex00/chant)](https://github.com/lex00/chant/releases/latest)

Chant is a spec execution platform for AI-assisted development. Specs are markdown files that agents execute.

## Installation

Get Chant up and running in seconds:

**Quick Install (Linux/macOS):**
```bash
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-linux-x86_64 -o chant
chmod +x chant
sudo mv chant /usr/local/bin/
```

**Other methods:**
- [Homebrew](#): `brew install lex00/tap/chant`
- [Cargo](#): `cargo install --git https://github.com/lex00/chant`
- [Download binaries](#): Visit the [Releases page](https://github.com/lex00/chant/releases/latest)
- [Build from source](#): Clone and run `cargo build --release`

For detailed platform-specific instructions and troubleshooting, see the [Installation Guide](https://lex00.github.io/chant/getting-started/installation.html).

## Documentation

Full documentation is available at **[lex00.github.io/chant](https://lex00.github.io/chant)**

To build and preview docs locally:

```bash
just docs-serve
```

## Quick Start

```bash
# Initialize chant in your project
chant init

# Add a spec
chant add "Add user authentication"

# Execute the spec
chant work 001
```

## Core Concepts

- **Specs** - Markdown files with YAML frontmatter describing work to be done
- **Execution** - Agents implement specs following acceptance criteria
- **Verification** - Continuous checking that work meets intent
- **Drift Detection** - Know when reality diverges from specs

## Development

```bash
# Build the binary
just build

# Run tests
just test

# Build and serve docs
just docs-serve
```

## License

MIT
