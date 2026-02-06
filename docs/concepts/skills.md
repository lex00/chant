# Skills

## Overview

Skills are portable instruction packages that teach AI agents how to perform specific tasks. They follow the [Agent Skills open standard](https://agentskills.io/specification) — an industry-wide format adopted by Claude Code, Kiro, Cursor, GitHub Copilot, Codex, and others.

Chant uses skills to give your IDE's agent context about your project's chant workflow, spec structure, and conventions. When you run `chant init --agent <provider>`, chant deposits a skill into the provider's skills directory so the agent automatically knows how to work with specs.

## The Agent Skills Standard

The Agent Skills format is an open standard published at [agentskills.io](https://agentskills.io). It defines a simple, portable structure for packaging agent instructions.

### Directory Structure

Every skill is a directory containing at minimum a `SKILL.md` file:

```
skill-name/
├── SKILL.md          # Required — instructions and metadata
├── scripts/          # Optional — executable code (Python, Bash, etc.)
├── references/       # Optional — additional documentation
└── assets/           # Optional — templates, schemas, static files
```

### SKILL.md Format

The `SKILL.md` file uses YAML frontmatter followed by markdown instructions:

```yaml
---
name: my-skill
description: What this skill does and when to use it.
---

## Instructions

Step-by-step guidance for the agent...
```

### Required Fields

| Field | Constraints |
|-------|------------|
| `name` | Max 64 chars. Lowercase letters, numbers, hyphens. Must match directory name. |
| `description` | Max 1024 chars. Describes what the skill does and when to activate it. |

### Optional Fields

| Field | Purpose |
|-------|---------|
| `license` | License name or reference to bundled file |
| `compatibility` | Environment requirements (tools, network, etc.) |
| `metadata` | Arbitrary key-value pairs (author, version) |
| `allowed-tools` | Pre-approved tools the skill may use (experimental) |

### Progressive Disclosure

Skills are designed for efficient context usage:

1. **Discovery** (~100 tokens): Only `name` and `description` load at startup for all skills
2. **Activation** (< 5000 tokens): Full `SKILL.md` body loads when the agent matches a request
3. **Resources** (as needed): Files in `scripts/`, `references/`, `assets/` load on demand

This means you can have many skills installed without overwhelming the agent's context window.

## Skills in Chant

### How Chant Uses Skills

When you run `chant init --agent <provider>`, chant creates a skill in your provider's skills directory:

| Provider | Skills Directory |
|----------|-----------------|
| Claude Code | `.claude/skills/chant/SKILL.md` |
| Kiro | `.kiro/skills/chant/SKILL.md` |
| Cursor | `.cursor/skills/chant/SKILL.md` |

The chant skill teaches the agent about:
- Spec structure and lifecycle
- How to read and execute specs
- Commit message conventions
- Acceptance criteria workflow

Because the format is identical across providers, chant uses a single skill template that works everywhere — only the destination directory differs.

### Skills vs Prompts

Chant has two distinct instruction systems:

| Aspect | Skills | Prompts |
|--------|--------|---------|
| Standard | Agent Skills (open) | Chant-specific |
| Location | Provider's skills dir | `.chant/prompts/` |
| Loaded by | IDE/agent at startup | `chant work` at execution time |
| Scope | Interactive sessions | Single spec execution |
| Purpose | General chant awareness | Specific agent behavior |
| Examples | `chant/SKILL.md` | `standard.md`, `bugfix.md` |

**Skills** give the agent ambient knowledge about chant — activated when the user mentions specs, acceptance criteria, or chant workflows during interactive sessions.

**Prompts** are injected by `chant work` to control agent behavior during spec execution — they define the execution loop, constraints, and output format.

### Skills vs Rules

Some providers have a separate "rules" or "steering" concept (e.g., `.kiro/rules.md`, `CLAUDE.md`, `.cursorrules`). These are always-loaded project instructions that apply to every interaction.

Skills differ from rules:
- **Rules** are always loaded — every token counts against context
- **Skills** are selectively activated — only loaded when relevant
- **Rules** are project-wide — apply to all tasks
- **Skills** are task-specific — activated by matching description

Chant may write both: rules for essential project conventions, and skills for chant-specific workflows that only activate when needed.

## Provider Skills Directories

### Workspace vs Global

Most providers support two skill scopes:

| Scope | Location | Purpose |
|-------|----------|---------|
| Workspace | `.{provider}/skills/` | Project-specific skills |
| Global | `~/.{provider}/skills/` | Personal skills across all projects |

Workspace skills override global skills when names conflict.

`chant init` writes workspace skills so they're scoped to the project and version-controlled with the codebase.

### Per-Provider Details

**Claude Code**: Skills in `.claude/skills/`. Supports slash commands, MCP tools, and the full Agent Skills spec.

**Kiro**: Skills in `.kiro/skills/`. Shipped in Kiro v0.9.0. Supports progressive disclosure and auto-activation.

**Cursor**: Skills support in progress. Expected at `.cursor/skills/`.

**GitHub Copilot / VS Code**: Supports Agent Skills via the VS Code extensions API.

## Creating Custom Skills

You can create project-specific skills alongside the chant skill:

```
.claude/skills/
├── chant/
│   └── SKILL.md          # Created by chant init
├── deploy/
│   ├── SKILL.md           # Your custom deployment skill
│   └── scripts/
│       └── deploy.sh
└── code-review/
    ├── SKILL.md           # Your code review standards
    └── references/
        └── style-guide.md
```

### Skill Authoring Tips

1. **Keep SKILL.md under 500 lines** — move detailed docs to `references/`
2. **Write a good description** — this determines when the skill activates
3. **Include keywords** — the agent matches descriptions against user requests
4. **Be specific** — "Review pull requests for security issues and test coverage" beats "Helps with PRs"
5. **Use scripts for automation** — put executable logic in `scripts/`, not inline

### Example: Custom Skill

```yaml
---
name: api-conventions
description: Enforce API design conventions including REST patterns,
  error response formats, and pagination. Use when creating or
  modifying API endpoints.
metadata:
  author: my-team
  version: "1.0"
---

## API Design Rules

All endpoints must follow these conventions:

### URL Structure
- Use plural nouns: `/users`, `/orders`
- Nest for relationships: `/users/{id}/orders`
...
```

## Advanced: The Open Standard

### Specification

The full Agent Skills specification is at [agentskills.io/specification](https://agentskills.io/specification). Key design principles:

- **Portability**: Same format works across all supporting agents
- **Progressive disclosure**: Metadata loads first, instructions on activation, resources on demand
- **Composability**: Skills are independent units that can be mixed and matched
- **Simplicity**: A directory with a SKILL.md is a valid skill

### Validation

Use the reference library to validate skills:

```bash
# Install the validator
npm install -g @agentskills/skills-ref

# Validate a skill
skills-ref validate ./my-skill
```

### Ecosystem

The Agent Skills standard is supported by:

- **Anthropic**: Claude Code, Claude Desktop
- **Amazon**: Kiro
- **Microsoft**: GitHub Copilot, VS Code
- **OpenAI**: Codex, ChatGPT
- **Cursor**: Cursor IDE
- **Community**: Antigravity, OpenCode, and others

Skills can be shared via GitHub repositories and imported into any supporting agent.

### Resources

- [Agent Skills Specification](https://agentskills.io/specification)
- [Anthropic Skills Repository](https://github.com/anthropics/skills)
- [Agent Skills Community](https://github.com/agentskills/agentskills)
- [Claude Code Skills Docs](https://code.claude.com/docs/en/skills)
- [Kiro Skills Docs](https://kiro.dev/docs/skills/)
