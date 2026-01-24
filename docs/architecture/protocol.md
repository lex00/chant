# Agent Protocol

## Overview

How chant invokes AI agents and exchanges information.

## Invocation

```bash
# Primary: CLI with environment context
CHANT_SPEC_ID=2026-01-22-001-x7m \
CHANT_PROJECT=auth \
CHANT_PROMPT=standard \
claude --print "$(cat .chant/prompts/standard.md)" < spec.md
```

## Environment Variables

Chant sets these before invoking the agent:

| Variable | Description |
|----------|-------------|
| `CHANT_SPEC_ID` | Current spec ID |
| `CHANT_SPEC_FILE` | Absolute path to spec markdown |
| `CHANT_PROJECT` | Project prefix (if any) |
| `CHANT_PROMPT` | Prompt name being used |
| `CHANT_PROMPT_FILE` | Absolute path to prompt markdown |
| `CHANT_WORKTREE` | Worktree path (if isolated) |
| `CHANT_ATTEMPT` | Attempt number (1, 2, 3...) |
| `CHANT_TIMEOUT` | Remaining timeout in seconds |

## Input

Agent receives:

1. **System prompt** - The prompt markdown (agent behavior)
2. **User prompt** - The spec markdown (what to do)

```bash
# Conceptual
claude \
  --system-prompt .chant/prompts/standard.md \
  --user-prompt .chant/specs/2026-01-22-001-x7m.md
```

## Output

Agent writes directly to:

1. **Working directory** - Code changes
2. **Spec file** - Progress updates (output section)
3. **Git** - Commits (if prompt instructs)

## Exit Handling

| Agent Exit | Chant Action |
|------------|--------------|
| 0 | Mark spec `completed` |
| Non-zero | Mark spec `failed`, capture error |
| Timeout | Kill agent, mark `failed` |
| Signal | Mark `failed`, note signal |

## MCP Server

Chant exposes an MCP server (`chant-mcp`) for tool integration with agents.

**Required for**: Some agents (only way to provide tools)
**Optional for**: CLI-based agents (enhancement for structured tool access)

See [mcp.md](../reference/mcp.md) for full design.

## Invocation Example

```bash
# Non-interactive execution (provider-specific)
CHANT_SPEC_ID=2026-01-22-001-x7m \
CHANT_SPEC_FILE=/path/to/spec.md \
CHANT_PROMPT_FILE=/path/to/prompt.md \
agent-cli --autonomous --spec $CHANT_SPEC_FILE
```

Each provider has its own CLI flags. Chant abstracts this through the provider interface.

## Providers

Chant supports multiple AI providers via adapters:

### Built-in Providers

Chant supports multiple AI providers via adapters. Providers are pluggable - see the protocol specification for details on adding new ones.

```yaml
# config.md
agent:
  provider: default       # Built-in provider
```

### Provider Configuration

```yaml
# config.md
agent:
  provider: my-provider
  model: model-name
  api_key: ${PROVIDER_API_KEY}
```

### Custom Providers (Plugins)

For unsupported agents, define custom command:

```yaml
# config.md
agent:
  provider: custom
  command: "my-agent --execute"
  # Chant sets env vars, pipes spec to stdin
```

### Provider Interface

Any provider must:

1. Accept spec content (stdin or file)
2. Accept prompt/system instruction (env var or flag)
3. Write changes to working directory
4. Exit 0 on success, non-zero on failure

```bash
# Provider receives:
CHANT_SPEC_ID=2026-01-22-001-x7m
CHANT_SPEC_FILE=/path/to/spec.md
CHANT_PROMPT_FILE=/path/to/prompt.md
CHANT_WORKTREE=/path/to/worktree

# Provider must:
# 1. Read spec and prompt
# 2. Execute changes
# 3. Exit with status
```

### Provider Plugins

Plugins live in `.chant/providers/`:

```
.chant/providers/
  my-provider.md       # Plugin definition
```

```markdown
# .chant/providers/my-provider.md
---
name: my-provider
command: my-agent
args:
  - "--autonomous"
  - "--spec-file"
  - "{{spec_file}}"
env:
  MY_AGENT_KEY: ${MY_AGENT_KEY}
---

# My Provider

Custom agent for specialized work.

## Setup

1. Install my-agent: `brew install my-agent`
2. Set MY_AGENT_KEY environment variable

## Capabilities

- Supports TypeScript
- Runs tests automatically
```

### Per-Spec Provider Override

```yaml
# spec frontmatter
---
status: pending
agent:
  provider: alternate-provider
  model: model-name
---
```

### Provider Selection Logic

1. Spec frontmatter `agent.provider` (highest)
2. Config `agent.provider`
3. Environment `CHANT_PROVIDER`
4. Built-in default

## Custom Command (Legacy)

Simple command override (deprecated, use providers):

```yaml
# config.md
agent:
  command: "my-agent --autonomous"
```

Spec and prompt are passed via stdin/env. Agent writes to filesystem.
