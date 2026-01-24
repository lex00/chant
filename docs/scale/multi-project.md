# Multi-Project Support

## Use Cases

### 1. Monorepo with Multiple Projects

```
mycompany/
├── packages/
│   ├── auth/           ← Project: auth
│   ├── payments/       ← Project: payments
│   └── notifications/  ← Project: notifications
├── services/
│   ├── api/            ← Project: api
│   └── worker/         ← Project: worker
└── .chant/
    └── tasks/
        ├── auth-2026-01-22-001-x7m.md
        ├── payments-2026-01-22-002-q2n.md
        └── api-2026-01-22-003-abc.md
```

### 2. Multiple Repos, One Chant

```
~/work/
├── frontend/           ← Repo 1
│   └── .chant/
├── backend/            ← Repo 2
│   └── .chant/
└── shared/             ← Repo 3
    └── .chant/

# Want unified view across all
```

### 3. Team with Project Ownership

```
Team: Platform
  └── Projects: auth, payments

Team: Product
  └── Projects: api, notifications

Specs assigned by project → routed to team
```

## Project Identification

### Explicit in Config

```yaml
# .chant/config.md
project:
  name: auth
  prefix: auth           # Spec IDs: auth-2026-01-22-001-x7m
```

### Derived from Path (Monorepo)

```yaml
# .chant/config.md
project:
  derive:
    from: path
    pattern: "packages/([^/]+)/"
    # packages/auth/foo.go → project: auth
```

### Derived from Spec Target Files

```yaml
# Spec frontmatter
---
target_files:
  - packages/auth/middleware.go
---

# Chant derives: project: auth
```

## Spec ID Format

### Local (Within Repo)

```
2026-01-22-001-x7m              # No prefix needed
auth-2026-01-22-001-x7m         # With project prefix
```

### Cross-Repo Reference

```
backend:2026-01-22-001-x7m      # repo:id
backend:auth-2026-01-22-001-x7m # repo:project-id
```

The `repo:` prefix is **optional** - only needed when referencing specs in other repos.

### With Project Prefix

```
auth-2026-01-22-001-x7m
└─┬─┘
  project prefix
```

### File Location

```
.chant/specs/
├── auth-2026-01-22-001-x7m.md
├── auth-2026-01-22-002-q2n.md
├── payments-2026-01-22-001-abc.md
└── api-2026-01-22-001-def.md
```

Or organized by folder:

```yaml
# config.md
project:
  spec_layout: folders   # flat | folders
```

```
.chant/specs/
├── auth/
│   ├── 2026-01-22-001-x7m.md
│   └── 2026-01-22-002-q2n.md
├── payments/
│   └── 2026-01-22-001-abc.md
└── api/
    └── 2026-01-22-001-def.md
```

## Cross-Project Dependencies

Specs can depend on specs in other projects:

```yaml
# payments-2026-01-22-001-abc.md
---
status: pending
depends_on:
  - auth-2026-01-22-001-x7m    # Different project
---

# Add payment processing

Requires auth middleware from auth project.
```

## Project-Specific Config

### Per-Project Prompts

```
.chant/
├── config.md                 # Global config
├── prompts/
│   ├── standard.md           # Default
│   └── projects/
│       ├── auth.md           # Auth-specific
│       └── payments.md       # Payments-specific
```

```yaml
# config.md
projects:
  auth:
    prompt: projects/auth
    labels: [security, critical]

  payments:
    prompt: projects/payments
    labels: [pci, financial]
```

### Per-Project Agent

```yaml
projects:
  auth:
    agent:
      model: high-capability   # Critical, use best model

  notifications:
    agent:
      model: standard          # Less critical
```

## Filtering by Project

```bash
# List specs for project
chant list --project auth
chant list --project payments

# Work on project
chant work --project auth     # Next ready spec in auth
chant work --parallel --project auth --max 3

# Search within project
chant search "status:pending project:auth"
```

## Multi-Repo View

### Global Config

```yaml
# ~/.config/chant/config.yaml
repos:
  - path: ~/work/frontend
    name: frontend

  - path: ~/work/backend
    name: backend

  - path: ~/work/shared
    name: shared
```

### Global State Layout

```
~/.config/chant/
├── config.yaml      # User-managed (repos list, global settings)
├── cache/           # Ephemeral index (rebuilt from repos)
│   ├── index.db     # Unified spec index
│   └── deps.db      # Cross-repo dependency graph
└── logs/            # Global daemon logs
```

**Not git-tracked** - index and cache are derived from repos, can be rebuilt.

### Unified Spec List

```bash
chant list --global           # All repos
chant list --global --project backend:auth

# Spec IDs include repo
# backend:auth-2026-01-22-001-x7m
```

### Cross-Repo Dependencies

```yaml
# In backend repo spec
---
depends_on:
  - shared:types-2026-01-22-001-abc   # Spec in different repo
---
```

**Constraint**: Cross-repo deps require both repos checked out.

## Project Permissions

### Who Can Work on What

```yaml
# config.md
projects:
  auth:
    owners: [alice, bob]
    reviewers: [security-team]

  payments:
    owners: [carol]
    reviewers: [finance-team, security-team]
```

### Enforcement

```bash
$ chant work payments-001 --as dave
Error: dave is not an owner of project 'payments'

Owners: carol
Use --override to bypass (logged)
```

## Queue by Project

### Project Priority

```yaml
# config.md
projects:
  critical-fix:
    priority: 100              # High

  nice-to-have:
    priority: 10               # Low
```

### Project Quotas

```yaml
# config.md
scale:
  limits:
    max_agents: 10
    per_project:
      auth: 3                  # Max 3 agents on auth
      payments: 2
      "*": 1                   # Others: 1 each
```

## Daemon Modes

### Per-Repo (Default)

```bash
cd ~/work/backend
chant daemon            # Watches only this repo
```

### Global (Multi-Repo)

```bash
chant daemon --global   # Watches all repos in ~/.config/chant/config.yaml
```

### Daemon Status

```bash
$ chant daemon status
Repos indexed:
  frontend:   32 specs (8 pending, 2 in_progress)
  backend:    47 specs (12 pending, 3 in_progress)
  shared:     15 specs (2 pending, 0 in_progress)

Projects indexed:
  auth:       47 specs (12 pending, 3 in_progress)
  payments:   23 specs (5 pending, 1 in_progress)
  api:        89 specs (20 pending, 0 in_progress)

Total: 94 specs, 5 agents active
```

## SCM Integration per Project

```yaml
projects:
  auth:
    scm:
      repo: mycompany/auth-service

  payments:
    scm:
      project: PAY
```

## CLI Examples

```bash
# Create spec in project
chant add "Fix auth bug" --project auth
# Creates: auth-2026-01-22-001-x7m

# Work on specific project
chant work --project payments

# Status by project
chant status --by-project

# Project overview
chant project list
chant project show auth
chant project stats
```

## Monorepo Best Practices

1. **Derive project from path** - Less manual tagging
2. **Project-specific prompts** - Different conventions per project
3. **Sparse checkout** - Only fetch relevant project files
4. **Project quotas** - Prevent one project hogging all agents
5. **Clear ownership** - Know who to ask about each project

## Configuration Reference

```yaml
# config.md
project:
  name: mycompany              # Root project name (optional)

  derive:
    from: path                 # path | target_files | explicit
    pattern: "packages/([^/]+)/"

  task_layout: flat            # flat | folders

projects:
  auth:
    prompt: prompts/auth
    labels: [security]
    owners: [alice]
    agent:
      model: opus
    scm:
      provider: github
    priority: 80

  payments:
    prompt: prompts/payments
    labels: [pci]
    owners: [carol]

scale:
  limits:
    per_project:
      auth: 3
      payments: 2
      "*": 1
```

## Implementation Phase

Multi-repo is part of **Phase 1** (Git+, MCP, Multi-Repo).

Core requirements:
- Spec ID parser handles optional `repo:` prefix
- Global config at `~/.config/chant/config.yaml`
- Dependency resolver supports cross-repo references
- Daemon supports `--global` mode for multi-repo watching
