# Lint Files Flag

**Status:** Planning

## Overview

Add `--files` flag to `chant lint` for targeted validation of specific spec files.

## Proposed Usage

```bash
# Lint specific files (useful in git hooks)
chant lint --files path/to/spec1.md path/to/spec2.md

# Lint files from stdin
git diff --cached --name-only -- '.chant/specs/*.md' | chant lint --files -
```

## Use Case

Pre-commit hooks need to validate only the staged spec files, not all specs:

```bash
#!/bin/sh
# .git/hooks/pre-commit
staged=$(git diff --cached --name-only -- '.chant/specs/*.md')
if [ -n "$staged" ]; then
    echo "$staged" | xargs chant lint --files
fi
```

## Current Workaround

Users must lint all specs with `chant lint`, which is slower and may report unrelated issues.

## Design Questions

1. Should invalid paths be errors or warnings?
2. Should the flag accept glob patterns?
3. How to handle files outside `.chant/specs/`?
