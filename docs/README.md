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
- [Technology Stack](architecture/stack.md)
- [Agent Protocol](architecture/protocol.md)
- [Agent Invocation](architecture/invoke.md)
- [Storage & Indexing](architecture/storage.md)

### Guides

- [Prompt Authoring Guide](guides/prompt-authoring.md)
- [Prompt Examples](guides/prompt-examples.md)
- [Advanced Prompting Guide](guides/prompt-advanced.md)
- [Research Workflows Guide](guides/research.md)
- [Examples](guides/examples.md)
- [Ecosystem Integration](guides/ecosystem.md)
- [Approvals](roadmap/planned/approvals.md) *(Planned)*
- [Recovery & Resume](guides/recovery.md)

### Reference

- [CLI Reference](reference/cli.md)
- [Configuration Reference](reference/config.md)
- [Errors](reference/errors.md)
- [Search Syntax](reference/search.md)
- [Git Integration](reference/git.md)
- [Git Hooks](reference/git-hooks.md)
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

## Installation

```bash
# Coming soon
brew install chant
# or
cargo install chant
```

---

<p align="center">
  <img src="assets/chant-logo.svg" alt="Chant" width="50">
  <br>
  <em>Intent Driven Development</em>
</p>
