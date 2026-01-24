# Initialization

## Command

```bash
chant init
```

Creates the `.chant/` directory structure in current repo.

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
  pr: false
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
chant init --stealth    # Local only, not committed
```

## Stealth Mode

For enterprise users on shared repos they don't control:

```bash
chant init --stealth
```

Creates `.chant/` but keeps it local:

1. Adds `.chant/` to `.git/info/exclude` (local gitignore, not committed)
2. Specs stay on your machine only
3. No trace in shared repo
4. Personal AI workflow within rigid enterprise environment

```bash
$ chant init --stealth
Created .chant/config.md
Created .chant/prompts/standard.md
Added .chant/ to .git/info/exclude (stealth mode)

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
