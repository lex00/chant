<img src="docs/assets/chant-logo.svg" alt="Chant" width="120" align="right">

# Chant

**Idempotent Intention**

[![CI](https://github.com/lex00/chant/actions/workflows/ci.yml/badge.svg)](https://github.com/lex00/chant/actions/workflows/ci.yml)
[![Release](https://github.com/lex00/chant/actions/workflows/release.yml/badge.svg)](https://github.com/lex00/chant/actions/workflows/release.yml)
[![codecov](https://codecov.io/gh/lex00/chant/branch/main/graph/badge.svg)](https://codecov.io/gh/lex00/chant)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Latest Release](https://img.shields.io/github/v/release/lex00/chant)](https://github.com/lex00/chant/releases/latest)
[![API Docs](https://img.shields.io/badge/docs-rustdoc-blue.svg)](https://lex00.github.io/chant/api/chant/)

Chant is a spec execution platform for AI-assisted development. Specs are markdown files that agents execute.

## Installation

See the [Installation Guide](https://lex00.github.io/chant/getting-started/installation.html) for detailed platform-specific instructions.

## Documentation

Full documentation is available at **[lex00.github.io/chant](https://lex00.github.io/chant)**

To build and preview docs locally:

```bash
just docs-serve
```

## Quick Start

**1. Initialize your project:**

```bash
chant init
```

**2. Create a spec:**

```bash
chant add "Add user authentication"
```

**3. Execute the spec:**

```bash
chant work 001
```

For detailed setup instructions, see the [Quick Start Guide](https://lex00.github.io/chant/getting-started/quickstart.html).

## Core Concepts

- **Specs** - Markdown files with YAML frontmatter describing work to be done
- **Execution** - Agents implement specs following acceptance criteria
- **Verification** - Continuous checking that work meets intent
- **Drift Detection** - Know when reality diverges from specs

## Key Features

- **Parallel execution** - Run multiple specs concurrently with isolated worktrees
- **Chain execution** - Process specs sequentially (`chant work --chain`) or chain through specific IDs
- **Approval workflow** - Gate spec execution with human approval, auto-detect agent-assisted work
- **Branch mode** - Execute specs in feature branches with automatic merge

## Examples

See the [examples/](./examples/) folder for real-world workflows (coming soon).

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

Apache-2.0. See [LICENSE](LICENSE) for details.
