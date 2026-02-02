Write specs in markdown. Agents execute them. Everything is git-tracked.

```bash
chant add "Add user authentication"
chant work 001
# Agent implements the spec
# Changes committed automatically
```

## Documentation

### Getting Started

- [Quickstart](getting-started/quickstart.md)
- [Philosophy](getting-started/philosophy.md)
- [Value Proposition](getting-started/value.md)

### Core Concepts

- [Specs](concepts/specs.md)
- [Spec Types](concepts/spec-types.md)
- [Prompts](concepts/prompts.md)
- [Spec IDs](concepts/ids.md)
- [Spec Groups](concepts/groups.md)
- [Dependencies](concepts/deps.md)
- [Hooks](roadmap/planned/hooks.md) *(Planned)*
- [Triggers](roadmap/planned/triggers.md) *(Planned)*
- [Autonomous Workflows](concepts/autonomy.md)
- [Data Lifecycle](concepts/lifecycle.md)
- [Skills](concepts/skills.md)

### Architecture

- [Architecture Overview](architecture/architecture.md)
- [Agent Protocol](architecture/protocol.md)

### Guides

- [Prompt Guide](guides/prompts.md)
- [Research Workflows Guide](guides/research.md)
- [OSS Maintainer Workflow](guides/oss-maintainer-workflow/index.md)
- [Examples](guides/examples.md)
- [Ecosystem Integration](guides/ecosystem.md)
- [Approval Workflow](guides/approval-workflow.md)
- [Recovery & Resume](guides/recovery.md)

### Reference

- [CLI Reference](reference/cli.md)
- [Configuration Reference](reference/config.md)
- [Errors](reference/errors.md)
- [Search Syntax](reference/search.md)
- [Git Integration](reference/git.md)
- [Templates](reference/templates.md)
- [Schema & Validation](reference/schema.md)
- [Notifications](roadmap/planned/notifications.md) *(Planned)*
- [Export](reference/reports.md)
- [Cost Tracking](roadmap/planned/costs.md) *(Planned)*
- [Initialization](reference/init.md)
- [MCP Server](reference/mcp.md)
- [Versioning](reference/versioning.md)
- [Output & Progress](reference/output.md)

### Scale

- [Chant at Scale](scale/scale.md)
- [Daemon Mode](roadmap/planned/daemon.md) *(Planned)*
- [Multi-Project Support](scale/multi-project.md)
- [Work Isolation](scale/isolation.md)
- [Locking & Recovery](scale/locks.md)
- [Queue Architecture](scale/queue.md)
- [Metrics](roadmap/planned/metrics.md) *(Planned)*
- [Observability](scale/observability.md)

### Enterprise

- [Enterprise Features](enterprise/enterprise.md)
- [Security](enterprise/security.md)

### Roadmap

- [Roadmap](roadmap/roadmap.md)

## Building Documentation

This documentation is built using [mdbook](https://rust-lang.github.io/mdBook/).

To build the documentation locally:

```bash
# Install mdbook if needed
cargo install mdbook

# Build the documentation
mdbook build docs

# Or serve with live reload
mdbook serve docs
```

The built documentation will be in `docs/book/`.

## Installation

See the [Installation Guide](https://lex00.github.io/chant/getting-started/installation.html) for detailed instructions.

**Quick options:**
- **Homebrew:** `brew tap lex00/tap && brew install chant`
- **Cargo:** `cargo install --git https://github.com/lex00/chant`
- **Direct download:** Visit the [Releases page](https://github.com/lex00/chant/releases/latest)

---

<p align="center">
  <img src="assets/chant-logo.svg" alt="Chant" width="50">
  <br>
  <em>Intent Driven Development</em>
</p>
