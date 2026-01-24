# Technology Stack

## Implementation: Rust

Single native binary. No runtime dependencies.

## Dependencies

```toml
[dependencies]
# Parsing
pulldown-cmark = "0.9"        # Markdown parsing
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"            # YAML frontmatter

# Search & Storage
tantivy = "0.21"              # Full-text search (archive)
sled = "0.34"                 # Optional: daemon mode

# CLI
clap = { version = "4.0", features = ["derive"] }
colored = "2.0"               # Terminal colors
indicatif = "0.17"            # Progress bars

# Async / Parallel
rayon = "1.8"                 # Parallel parsing
notify = "6.0"                # File watching (optional)

# Utilities
glob = "0.3"
chrono = "0.4"
anyhow = "1.0"
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      CLI (clap)                         │
├─────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────┐ │
│  │ Active Specs │  │ Archive Search│  │    Config    │ │
│  │ (in-memory)  │  │  (tantivy)    │  │   (serde)    │ │
│  └──────┬───────┘  └───────┬───────┘  └──────┬───────┘ │
│         │                  │                  │         │
│  ┌──────▼───────────────────────────────────────────┐  │
│  │         .chant/specs/*.md (Source of Truth)       │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  ┌────────────────┐  ┌─────────────────┐               │
│  │ Git (shell out)│  │ Agent (shell)   │               │
│  └────────────────┘  └─────────────────┘               │
└─────────────────────────────────────────────────────────┘
```

## External Dependencies

| Dependency | Integration | Notes |
|------------|-------------|-------|
| Git | Shell out | Uses user's git config/auth |
| AI Agent | Shell out | Provider CLI invocation |

Shell out is simpler than library bindings and uses user's existing setup.

## Delivery

| Method | Support |
|--------|---------|
| `brew install chant` | Yes |
| `cargo install chant` | Yes |
| `curl \| sh` | Yes |
| GitHub releases | Yes (cargo-dist) |

Single static binary. Cross-platform (macOS, Linux, Windows).
