# Spec Templates

**Status:** Planning

## Overview

A template system for creating specs with predefined structure and defaults.

## Proposed Usage

```bash
# Create spec from template
chant add "Fix login bug" --template bugfix

# List available templates
chant templates list

# Create custom template
chant templates create feature
```

## Proposed Template Location

```
.chant/
├── templates/
│   ├── bugfix.md
│   ├── feature.md
│   └── refactor.md
```

## Template Format

```markdown
---
type: code
labels:
  - {{label}}
---
# {{title}}

## Problem

{{description}}

## Solution

[Describe approach]

## Acceptance Criteria

- [ ] Issue reproduced
- [ ] Fix implemented
- [ ] Tests added
- [ ] No regressions
```

## Design Questions

1. What templating engine to use? (Handlebars, Tera, simple substitution)
2. Should templates support conditionals?
3. How to handle template variables interactively?
4. Should there be built-in default templates?
5. How do templates interact with `--type` flag?

## Current Workaround

Users create specs manually or copy from existing specs.
