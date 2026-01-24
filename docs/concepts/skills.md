# Skills

## Overview

Skills extend Claude Code with chant-specific capabilities. They provide slash commands and contextual knowledge for working with chant.

## What are Skills?

Skills are markdown files that teach Claude Code about specific domains or workflows. When loaded, they add:

- **Slash commands** - New commands like `/bootstrap`, `/spec`
- **Context** - Background knowledge about the domain
- **Workflows** - How to accomplish common tasks

## Chant Skills

### Bootstrap Skill

The bootstrap skill (`/bootstrap`) executes specs without the chant binary:

```
/bootstrap           # Execute next pending spec
/bootstrap 001       # Execute specific spec
/bootstrap --list    # List pending specs
```

Used during initial development before `chant work` exists.

### Chant Dev Skill

The chant-dev skill provides context for developing chant itself:

- Workflow discipline (all changes through specs)
- Execution loop: Read → Plan → Change → Verify → Commit
- Project structure and patterns
- CLI command reference

## Installing Skills

Skills are loaded via Claude Code's skill system:

```bash
# In Claude Code, load a skill
/skill add chant
```

Or add to Claude Code configuration:

```json
{
  "skills": ["chant"]
}
```

## Skill Location

Skills live in `.claude/skills/` within the chant repository:

```
.claude/
└── skills/
    ├── bootstrap/
    │   └── SKILL.md
    └── chant-dev/
        └── SKILL.md
```

## Creating Custom Skills

Skills are markdown files with a specific structure:

```markdown
# My Skill

## Commands

### /mycommand

Description of what the command does.

**Usage:**
- `/mycommand` - Basic usage
- `/mycommand --flag` - With options

## Context

Background knowledge the agent needs...

## Workflows

### Common Task

Steps to accomplish the task...
```

## Skill vs Prompt

| Aspect | Skill | Prompt |
|--------|-------|--------|
| Scope | Claude Code session | Single spec execution |
| Purpose | Interactive commands | Agent behavior |
| Loaded by | User / config | `chant work` |
| Examples | `/bootstrap`, `/spec` | `standard.md`, `bugfix.md` |

Skills are for human interaction with Claude Code. Prompts are for agent execution of specs.

## Built-in Skills

| Skill | Purpose | Commands |
|-------|---------|----------|
| `bootstrap` | Initial implementation | `/bootstrap` |
| `chant-dev` | Developing chant | Context only |

## Future Skills

Planned skills for chant users:

- **chant** - General chant usage (`/spec`, `/work`, `/status`)
- **chant-research** - Research workflows (`/synthesis`, `/analysis`)
- **chant-docs** - Documentation tracking (`/track`, `/verify`)

## Relationship to Chant

Skills complement chant but don't replace it:

```
┌─────────────────────────────────────────────┐
│              Claude Code                     │
│                                             │
│  Skills ──────────────────────────────────┐ │
│  (Interactive, human-facing)              │ │
│                                           │ │
│  ┌─────────────────────────────────────┐  │ │
│  │           Chant CLI                  │  │ │
│  │                                      │  │ │
│  │  Prompts ─────────────────────────┐ │  │ │
│  │  (Agent execution)                │ │  │ │
│  │                                   │ │  │ │
│  │  ┌────────────────────────────┐  │ │  │ │
│  │  │         Spec               │  │ │  │ │
│  │  │    (Source of truth)       │  │ │  │ │
│  │  └────────────────────────────┘  │ │  │ │
│  │                                   │ │  │ │
│  └───────────────────────────────────┘ │  │ │
│                                        │  │ │
└────────────────────────────────────────┘──┘ │
│                                             │
└─────────────────────────────────────────────┘
```

Skills help humans interact with Claude Code around chant workflows. Chant prompts control agent behavior during spec execution.
