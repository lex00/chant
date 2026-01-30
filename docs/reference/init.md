# Initialization

## Interactive Setup Wizard (Recommended)

For first-time setup, run `chant init` with no arguments:

```bash
chant init
```

The wizard guides you through all configuration options:
- **Project name**: Auto-detected from package.json, Cargo.toml, go.mod, or directory name
- **Prompt templates**: Include ready-to-use prompts (recommended) or skip for minimal setup
- **Silent mode**: Keep .chant/ local only (gitignored) for enterprise environments
- **Model provider**: Claude CLI (recommended), Ollama (local), or OpenAI API
- **Default model**: opus, sonnet, haiku, or custom model name
- **Agent configuration**: Claude Code (CLAUDE.md), Cursor, Amazon Q, Generic, or all

The wizard is the best path for new users because it:
- Asks all the right questions in order
- Explains each option with clear prompts
- Automatically creates MCP config when Claude is selected
- Provides sensible defaults
- Prevents configuration mistakes

## What the Wizard Creates

When you select Claude agent configuration, the wizard creates:
- `.chant/` directory with config, prompts, and specs
- `CLAUDE.md` with agent instructions
- `.mcp.json` for MCP server integration

## Direct Configuration (for scripts/automation)

For CI/CD pipelines or scripted setups, use flags directly:

```bash
chant init --agent claude --provider claude --model opus
```

This creates the `.chant/` directory structure in current repo.

## What Gets Created

```
.chant/
├── config.md             # Project configuration
├── prompts/              # Prompt files
│   └── standard.md       # Default prompt
├── specs/                # Spec files (empty)
├── .locks/               # PID files (gitignored)
├── .store/               # Index cache (gitignored)
└── .gitignore            # Ignores local state
```

## Generated Files

### config.md

```markdown
---
project:
  name: {detected from package.json, Cargo.toml, or dirname}

defaults:
  prompt: standard
  branch: false
---

# Chant Configuration

Project initialized on {date}.
```

### prompts/standard.md

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

## Acceptance Criteria

{{#each spec.acceptance}}
- [ ] {{this}}
{{/each}}

## Instructions

1. **Read** the relevant code first
2. **Plan** your approach before coding
3. **Implement** the changes
4. **Verify** each acceptance criterion
5. **Commit** with message: `chant({{spec.id}}): <description>`

## Constraints

- Only modify files related to this spec
- Follow existing code patterns
- Do not refactor unrelated code
```

### .gitignore

```gitignore
# Local state (not shared)
.locks/
.store/
```

## Detection

`chant init` detects project info:

| Source | Field |
|--------|-------|
| `package.json` → `name` | project.name |
| `Cargo.toml` → `[package] name` | project.name |
| `go.mod` → module path | project.name |
| Directory name | project.name (fallback) |

## Idempotent

Running `chant init` twice is safe:
- Existing files are not overwritten
- Missing files are created
- Reports what was created/skipped

```bash
$ chant init
Created .chant/config.md
Created .chant/prompts/standard.md
Skipped .chant/specs/ (exists)
```

## Flags

```bash
chant init --force      # Overwrite existing files
chant init --minimal    # Only config.md, no prompts
chant init --name foo   # Override detected name
chant init --silent     # Local only, not committed
```

## Silent Mode

For enterprise users on shared repos they don't control:

```bash
chant init --silent
```

Creates `.chant/` but keeps it local:

1. Adds `.chant/` to `.git/info/exclude` (local gitignore, not committed)
2. Specs stay on your machine only
3. No trace in shared repo
4. Personal AI workflow within rigid enterprise environment

```bash
$ chant init --silent
Created .chant/config.md
Created .chant/prompts/standard.md
Added .chant/ to .git/info/exclude (silent mode)

Note: Specs will not be committed. Local use only.
```

**Trade-offs:**
- No git history for specs
- No collaboration via git
- Lost if machine fails

**Use case:** Enterprise developer using AI assistance on a project that doesn't officially support it. Personal productivity without changing shared repo.

## Post-Init

After init, typical flow:

```bash
chant init
chant add "First spec"
chant work 2026-01-22-001-x7m
```
