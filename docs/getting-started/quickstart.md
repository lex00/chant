# Quickstart

## The One Thing to Understand

**Prompts define what agents do.**

Everything else (specs, git, daemon) is infrastructure. Prompts are the behavior.

```
.chant/prompts/
  standard.md     ← "How to implement a spec"
  review.md       ← "How to review code"
  split.md        ← "How to break down work"
```

A prompt is a markdown file that tells the agent what to do.

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

## Customizing Behavior

Want different agent behavior? Edit the prompt.

```markdown
# .chant/prompts/standard.md
---
name: standard
---

# Your Spec

{{spec.body}}

## How to Work

1. Read the code first
2. Make minimal changes
3. Run tests before committing
```

That's it. No plugins, no framework code.

## Using Community Prompts

```bash
# Install from registry
chant prompt add tdd --from chant-prompts/tdd

# Or from git
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
