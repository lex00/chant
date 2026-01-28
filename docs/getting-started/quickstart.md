# Quickstart

## The One Thing to Understand

**Prompts define what agents do.**

Everything else (specs, git, daemon) is infrastructure. Prompts are the behavior.

```
.chant/prompts/
  standard.md     ← "How to implement a spec"
  split.md        ← "How to break down work"
```

A prompt is a markdown file that tells the agent what to do. It contains instructions on how to complete work, what to check, how to test, and when to commit.

## Your First 5 Minutes

```bash
# 1. Initialize
chant init

# 2. Look at the default prompt
cat .chant/prompts/standard.md

# 3. Create a spec
chant add "Fix the login bug"

# 4. Run it (agent uses standard.md prompt)
chant work 001
```

The agent reads `standard.md`, sees your spec, and executes.

## Built-in Prompts

Chant comes with ready-to-use prompts for different workflows:

### bootstrap.md (Default)
A minimal prompt that tells the agent to run `chant prep <spec-id>` to get the actual spec content. This:
- Reduces initial prompt size (helps with API rate limits)
- Supports replay/resume scenarios cleanly
- Separates spec content from agent instructions

**Used when:** `chant work <spec-id>` (no prompt specified)

### standard.md
The full prompt for implementing specs. It instructs the agent to:
- Read relevant code first
- Plan the approach
- Implement changes
- Verify the implementation works
- Commit with a proper message

**Used when:** `chant work <spec-id> --prompt standard`

### split.md
A specialized prompt for analyzing driver specs and proposing how to break them down into smaller member specs. It:
- Analyzes the specification and acceptance criteria
- Proposes a sequence of member specs
- Ensures each member leaves code in compilable state
- Provides detailed acceptance criteria for each member

**Used when:** `chant split <spec-id>` or `chant work <spec-id> --prompt split`

## Customizing Behavior

Want different agent behavior? Edit the prompt.

```markdown
# .chant/prompts/standard.md
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

Template variables like `{{spec.title}}` and `{{project.name}}` are replaced with actual values when the prompt runs. See [prompts.md](../concepts/prompts.md#template-variables) for all available variables.

That's it. No plugins, no framework code.

## Using Community Prompts (Planned)

> **Status: Planned** - The prompt registry is on the roadmap but not yet implemented. For now, create prompts manually in `.chant/prompts/`.

```bash
# Install from registry (planned)
chant prompt add tdd --from chant-prompts/tdd

# Or from git (planned)
chant prompt add security --from github:acme/security-prompts

# Use it
chant work 001 --prompt tdd
```

## Creating Specs

```bash
chant add "Add user authentication"
```

Creates `.chant/specs/2026-01-22-001-x7m.md`:

```markdown
---
status: pending
---

# Add user authentication
```

Edit to add detail:

```markdown
---
status: pending
prompt: standard
---

# Add user authentication

## Context
We need JWT-based auth for the API.

## Acceptance Criteria
- [ ] Login endpoint returns JWT
- [ ] Protected routes check token
- [ ] 401 on invalid token
```

## Running Work

```bash
chant work 001                    # Run with default prompt
chant work 001 --prompt tdd       # Run with TDD prompt
chant work 001 --prompt security  # Run with security review prompt
```

## The Mental Model

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

- **Prompt** = Behavior (how)
- **Spec** = Goal (what)
- **Agent** = Executor (who)

## What's Next

| Want to... | Read... |
|------------|---------|
| **Go autonomous** | [autonomy.md](../concepts/autonomy.md) |
| Write better prompts | [prompts.md](../concepts/prompts.md) |
| Use different AI providers | [protocol.md](../architecture/protocol.md) |
| Run specs in parallel | [deps.md](../concepts/deps.md) |
| Set up for a team | [examples.md](../guides/examples.md) |

## The Goal: Autonomy

Chant starts in **supervised mode** — you review every change. The goal is **autonomous workflows**:

- Start: Review everything
- Progress: Trivial specs auto-merge
- Later: Most specs auto-merge, review exceptions
- Goal: Agents work overnight, review summaries

See [autonomy.md](../concepts/autonomy.md) for the full journey.
