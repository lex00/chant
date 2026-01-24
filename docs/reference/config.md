# Configuration Reference

## Config as Markdown

Configuration follows the same pattern as specs: markdown with YAML frontmatter.

```
.chant/config.md    ← Not config.yaml
```

Frontmatter is the config. Body is documentation.

## Example

```markdown
# .chant/config.md
---
project:
  name: my-app

defaults:
  prompt: standard
  branch: false
  pr: false
  branch_prefix: "chant/"

schema:
  spec:
    required: [status]
    status:
      enum: [pending, in_progress, completed, failed]
---

# Project Configuration

Direct commits to main by default. No PRs unless
explicitly requested per-spec.

## Prompts

- `standard` - Default for most specs
- `tdd` - Use for anything touching auth
- `security-review` - Required for external API changes

## Team Notes

Run `chant lint` before pushing.
```

## Why Markdown?

1. **Consistency** - Same format as specs and prompts
2. **Self-documenting** - Body explains the config
3. **Editable anywhere** - Any text editor works
4. **Git-friendly** - Readable diffs

## Minimal Config

```markdown
# .chant/config.md
---
project:
  name: my-app
---

# Config

Using all defaults.
```

## Full Schema

```yaml
---
# Required
project:
  name: string              # Project name for templates
  prefix: string            # Optional: ID prefix for scale deployments

# Optional - defaults shown
defaults:
  prompt: standard          # Default prompt
  branch: false             # Create branches?
  pr: false                 # Create PRs?
  branch_prefix: "chant/"   # Branch name prefix

# Optional - git provider settings
git:
  provider: github          # PR provider: github, gitlab, bitbucket

# Optional - schema validation
schema:
  spec:
    required: [status]      # Required frontmatter fields (id comes from filename)
    status:
      enum: [pending, in_progress, completed, failed]

# Optional - scale deployment settings
scale:
  # Project prefix auto-detection (monorepos)
  id_prefix:
    from: path              # or: explicit
    pattern: "packages/([^/]+)/"

  # Daemon settings
  daemon:
    enabled: false          # Auto-start daemon
    socket: /tmp/chant.sock
    metrics_port: 9090      # 0 = disabled
    api_port: 8080          # 0 = disabled

  # Worktree settings
  worktree:
    sparse: false           # Use sparse checkout
    pattern: "packages/{{project}}/"
    pool_size: 10           # Reusable worktree pool

  # Resource limits
  limits:
    max_agents: 100
    max_per_project: 10
    spec_timeout: 30m
---
```

## Environment Overrides

```bash
CHANT_BRANCH=true chant work 2026-01-22-001-x7m
CHANT_PROMPT=tdd chant work 2026-01-22-001-x7m
```

## Precedence

1. Spec frontmatter (highest)
2. Environment variables
3. Config file
4. Built-in defaults (lowest)
