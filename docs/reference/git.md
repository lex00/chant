# Git Integration

## Explicit Flags

Git behavior is controlled by explicit boolean flags, not named presets.

```yaml
# .chant/config.md frontmatter
defaults:
  branch: false      # Create a branch for each spec?
  pr: false          # Create a PR on completion?
  branch_prefix: "chant/"
```

## Provider Configuration

Chant supports multiple git hosting providers for PR/MR creation:

```yaml
# .chant/config.md frontmatter
git:
  provider: github   # github (default), gitlab, or bitbucket
```

| Provider | CLI Tool | PR Type |
|----------|----------|---------|
| `github` | `gh` | Pull Request |
| `gitlab` | `glab` | Merge Request |
| `bitbucket` | `bb` | Pull Request |

Each provider requires its respective CLI tool to be installed and authenticated.

## Spec Override

Individual specs can override defaults:

```yaml
# Spec frontmatter (filename: 2026-01-22-001-x7m.md)
---
branch: true         # Override: this spec needs a branch
pr: true             # Override: this spec needs a PR
---
```

## Git Modes

| branch | pr | Behavior |
|--------|-----|----------|
| `false` | `false` | Commit directly to current branch (default) |
| `true` | `false` | Create branch, user merges manually |
| `true` | `true` | Create branch, create PR |

## Commit Flow

```
1. Agent commits work
   └── git commit -m "chant(2026-01-22-001-x7m): Add authentication"

2. CLI captures hash
   └── git rev-parse HEAD → a1b2c3d4

3. CLI updates frontmatter
   └── Adds: commit, completed_at, branch (if applicable)

4. CLI commits metadata update
   └── git commit -m "chant: mark 2026-01-22-001-x7m complete"
```

Two commits per spec completion:
1. The work (by agent)
2. The metadata update (by CLI)
