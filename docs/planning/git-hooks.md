# Git Hooks Automation

**Status:** Planning

## Overview

Automated git hook integration for chant workflows.

## Current State

Git hooks are documented as example scripts that users can manually install. There is no automated installation or management.

## Proposed Features

### Hook Installation

```bash
chant hooks install          # Install all hooks
chant hooks install pre-commit  # Install specific hook
chant hooks uninstall        # Remove all hooks
```

### Hook Scripts

The following hooks would be provided:

| Hook | Purpose |
|------|---------|
| `pre-commit` | Lint spec files before commit |
| `commit-msg` | Validate `chant(id): msg` format |
| `post-commit` | Update spec status after commit |
| `pre-push` | Warn about incomplete specs |

### Team Setup

Integration with hook managers like Lefthook:

```bash
chant hooks export lefthook > lefthook.yml
```

## Design Questions

1. Should hooks modify spec files directly or just validate?
2. How to handle hook failures gracefully?
3. Should hooks be opt-in or installed by default with `chant init`?

## Related

- Current manual hook examples in docs/reference/git.md
