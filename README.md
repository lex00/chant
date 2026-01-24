<img src="docs/assets/chant-logo.svg" alt="Chant" width="120" align="right">

# Chant

**Intent driven development**

Chant is a spec execution platform for AI-assisted development. Specs are markdown files that agents execute.

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
