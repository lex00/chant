# Quick Start

Get started with chant in 5 minutes.

## Installation

See the [Installation Guide](installation.md) for complete installation instructions.

## Your First 5 Minutes

### 1. Initialize Your Project

```bash
chant init
```

The wizard guides you through:
- Project name (auto-detected)
- Model provider (Claude CLI, Ollama, OpenAI)
- Default model (opus, sonnet, haiku, or custom)
- Agent configuration (creates CLAUDE.md and .mcp.json)

> **Tip:** For CI/CD, use flags: `chant init --agent claude --provider claude --model opus`

### 2. Create a Spec

```bash
chant add "Add welcome message to homepage"
```

Creates `.chant/specs/2026-02-03-001-xyz.md`:

```markdown
---
status: pending
---

# Add welcome message to homepage
```

### 3. Execute the Spec

```bash
chant work 001
```

The agent:
1. Reads your spec and acceptance criteria
2. Explores the codebase
3. Makes the necessary changes
4. Commits with: `chant(001): <description>`

### 4. Review Changes

```bash
git log -1        # View the commit
chant status      # Check spec status
chant show 001    # View spec details
```

## Understanding Prompts

**Prompts define what agents do.**

Chant uses markdown prompts to control agent behavior:

```
.chant/prompts/
  bootstrap.md    ← Default (minimal, bootstraps full prompt)
  standard.md     ← Full implementation instructions
  split.md        ← Break down large specs
```

### Default Prompt

```bash
chant work 001  # Uses bootstrap.md
```

The bootstrap prompt tells the agent to run `chant prep 001` to load the full spec and instructions.

### Custom Prompts

```bash
chant work 001 --prompt standard  # Use full prompt directly
chant work 001 --prompt tdd       # Use TDD workflow
```

### Customizing Behavior

Edit `.chant/prompts/standard.md` to change agent behavior:

```markdown
---
name: standard
purpose: Default execution prompt
---

# Execute Spec

You are implementing a spec for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Instructions

1. Read the relevant code first
2. Make minimal changes
3. Run tests before committing
4. Commit with message: `chant({{spec.id}}): <description>`
```

Template variables like `{{spec.title}}` are replaced at runtime. See [Prompts](../concepts/prompts.md) for all variables.

## Working with Specs

### Creating Specs

```bash
chant add "Add user authentication"
```

This creates `.chant/specs/2026-02-03-001-xyz.md`:

```markdown
---
status: pending
---

# Add user authentication
```

### Adding Detail

Edit the spec to add context and acceptance criteria:

```markdown
---
status: pending
prompt: standard
---

# Add user authentication

## Context
JWT-based auth for the API.

## Acceptance Criteria
- [ ] Login endpoint returns JWT
- [ ] Protected routes check token
- [ ] 401 on invalid token
```

### Executing Specs

```bash
chant work 001                    # Default prompt
chant work 001 --prompt tdd       # TDD workflow
chant work 001 --prompt security  # Security review
```

### Checking Status

```bash
chant status    # Project summary
chant list      # All specs
chant show 001  # Spec details
chant log 001   # Execution log
```

## Key Concepts

### The Mental Model

```
┌─────────────────────────────────────────────────┐
│                    PROMPT                        │
│  "Read code, make changes, verify, commit"      │
├─────────────────────────────────────────────────┤
│                     SPEC                         │
│  "Add authentication to the API"                │
├─────────────────────────────────────────────────┤
│                    AGENT                         │
│  Any AI coding agent                            │
└─────────────────────────────────────────────────┘
```

- **Prompt** = Behavior (how to work)
- **Spec** = Goal (what to build)
- **Agent** = Executor (who does the work)

### Parallel Execution

Run multiple specs concurrently with isolated worktrees:

```bash
chant work --parallel      # Work all ready specs
chant work 001 002 003     # Work specific specs in parallel
```

### Chain Execution

Process specs sequentially:

```bash
chant work --chain         # Work ready specs one after another
chant work 001 --chain     # Work 001, then continue to next ready
```

## Agent Integration

### Using with Claude Code

```bash
chant init --agent claude
```

Creates:
- `.claude/CLAUDE.md` - Instructions for Claude Code
- `.claude/.mcp.json` - MCP server configuration

The MCP server exposes spec operations as tools Claude can use.

### Using with Cursor

```bash
chant init --agent cursor
```

Creates:
- `.cursorrules` - AI instructions for Cursor
- `.cursor/mcp.json` - MCP server configuration

See the [Cursor Guide](../guides/cursor-quickstart.md) for detailed setup.

## What's Next

| Want to... | Read... |
|------------|---------|
| **Set up Cursor** | [Cursor Guide](../guides/cursor-quickstart.md) |
| **Understand specs** | [Specs](../concepts/specs.md) |
| **Customize prompts** | [Prompts](../concepts/prompts.md) |
| **Go autonomous** | [Autonomy](../concepts/autonomy.md) |
| **Run in parallel** | [Dependencies](../concepts/deps.md) |
| **Use different providers** | [Protocol](../architecture/protocol.md) |

## The Path to Autonomy

Chant starts in **supervised mode** — you review every change. The goal is **autonomous workflows**:

1. **Start**: Review everything
2. **Progress**: Trivial specs auto-merge
3. **Later**: Most specs auto-merge, review exceptions
4. **Goal**: Agents work overnight, review summaries

See [Autonomy](../concepts/autonomy.md) for the journey.
