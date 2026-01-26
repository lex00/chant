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
  model: claude-opus-4
  provider: claude

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
  model: null               # Default model name (e.g. "claude-opus-4", "claude-sonnet-4")
  split_model: null         # Default model for split operations (defaults to sonnet)
  main_branch: "main"       # Default main branch name for merges
  provider: claude          # Default provider: claude, ollama, openai

# Optional - git provider settings
git:
  provider: github          # PR provider: github, gitlab, bitbucket

# Optional - model provider endpoints
providers:
  ollama:
    endpoint: http://localhost:11434/v1  # Ollama API endpoint
  openai:
    endpoint: https://api.openai.com/v1  # OpenAI API endpoint

# Optional - schema validation (Planned)
# Note: Schema validation is on the roadmap but not yet implemented
schema:
  spec:
    required: [status]      # Required frontmatter fields (id comes from filename)
    status:
      enum: [pending, in_progress, completed, failed]

# Optional - scale deployment settings (Planned)
# Note: Scale features are on the roadmap but not yet implemented
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

## Global Configuration

Chant supports a global config file for user-wide defaults:

```
~/.config/chant/config.md
```

### Merge Behavior

Project config overrides global config. Values are merged at the key level:

```
~/.config/chant/config.md    <- Global defaults
.chant/config.md             <- Project overrides
```

### Example Global Config

```markdown
# ~/.config/chant/config.md
---
defaults:
  branch: true
  pr: true
  model: claude-opus-4
  provider: claude

git:
  provider: github

providers:
  openai:
    endpoint: https://api.openai.com/v1
---

# Global Chant Settings

My default settings for all projects.
```

### Example Project Override

```markdown
# .chant/config.md
---
project:
  name: quick-prototype

defaults:
  branch: false   # Override: direct commits for this project
  pr: false
---
```

In this example, the global config sets `branch: true` and `pr: true`, but the project config overrides both to `false`. The `git.provider: github` from global config is still applied since the project doesn't override it.

## Environment Overrides (Planned)

> **Status: Planned** - Environment variable overrides are on the roadmap but not yet implemented.

```bash
CHANT_BRANCH=true chant work 2026-01-22-001-x7m
CHANT_PROMPT=tdd chant work 2026-01-22-001-x7m
```

## Precedence

1. Spec frontmatter (highest)
2. Environment variables
3. Project config (`.chant/config.md`)
4. Global config (`~/.config/chant/config.md`)
5. Built-in defaults (lowest)
