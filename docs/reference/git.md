# Git Integration

## Configuration

Git behavior is controlled by explicit boolean flags in `.chant/config.md`:

```yaml
defaults:
  branch: false      # Create a branch for each spec?
  branch_prefix: "chant/"
```

### Spec Override

Individual specs can override defaults:

```yaml
# Spec frontmatter
---
branch: true         # This spec needs a branch
---
```

## Git Modes

| branch | Behavior |
|--------|----------|
| `false` | Commit directly to current branch (default) |
| `true` | Create branch, user merges manually |

> **Note:** Specs serve as the review primitive in chant. Each spec has a title, acceptance criteria, branch, commits, and review workflow — fulfilling the same role as a pull request but working offline and without external dependencies.

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

---

## Git Hooks

> **Note:** Git hooks are optional and can be set up manually using the scripts below.

### Philosophy

Git hooks enhance the workflow but are **optional** - no Chant feature depends on them.

### Available Hooks

| Hook | Purpose | Chant Use |
|------|---------|-----------|
| `pre-commit` | Validate before commit | Lint spec files |
| `commit-msg` | Validate commit message | Enforce `chant(id): msg` format |
| `post-commit` | After commit completes | Update spec status |
| `pre-push` | Before push to remote | Warn about incomplete specs |
| `post-merge` | After merge/pull | Rebuild index |

### Hook Implementations

#### pre-commit

```bash
#!/bin/sh
# .git/hooks/pre-commit

staged_specs=$(git diff --cached --name-only -- '.chant/specs/*.md')

if [ -n "$staged_specs" ]; then
    # Validate all specs (staged files are a subset)
    chant lint
    if [ $? -ne 0 ]; then
        echo "Spec validation failed. Fix errors or use --no-verify"
        exit 1
    fi
fi
```

> **Note:** A `--files` flag for targeted validation is under consideration.

#### commit-msg

```bash
#!/bin/sh
# .git/hooks/commit-msg

msg=$(cat "$1")

if echo "$msg" | grep -qE '^chant\([a-z0-9-]+\):'; then
    spec_id=$(echo "$msg" | sed -E 's/^chant\(([a-z0-9-]+)\):.*/\1/')
    if ! chant show "$spec_id" >/dev/null 2>&1; then
        echo "Warning: Spec $spec_id not found"
    fi
fi

exit 0
```

#### post-commit

```bash
#!/bin/sh
# .git/hooks/post-commit

# Post-commit hooks can log commits for auditing.
# Chant automatically records commits during `chant work` execution,
# so manual commit tracking is typically not needed.

msg=$(git log -1 --format=%s)

if echo "$msg" | grep -qE '^chant\([a-z0-9-]+\):'; then
    spec_id=$(echo "$msg" | sed -E 's/^chant\(([a-z0-9-]+)\):.*/\1/')
    echo "Committed work for spec: $spec_id"
fi
```

#### pre-push

```bash
#!/bin/sh
# .git/hooks/pre-push

branch=$(git rev-parse --abbrev-ref HEAD)

if echo "$branch" | grep -qE '^chant/'; then
    spec_id=$(echo "$branch" | sed 's/^chant\///')
    # Check if spec exists and warn if pushing incomplete work
    if chant show "$spec_id" >/dev/null 2>&1; then
        echo "Pushing spec branch: $spec_id"
    else
        echo "Warning: Spec $spec_id not found"
    fi
fi

exit 0
```

### What Hooks Don't Do

Hooks are convenience, not enforcement:

| Feature | With Hooks | Without Hooks |
|---------|------------|---------------|
| Spec validation | Automatic on commit | `chant lint` manually |
| Commit recording | Automatic via `chant work` | Recorded during work execution |
| Status updates | Automatic | `chant finalize` manually |

### Team Setup

For teams, commit hook scripts to the repository and use a hook manager:

```bash
# Example with Lefthook (manual setup)
# Create lefthook.yml with chant-lint commands
# Team members run: lefthook install
```

### Skipping Hooks

```bash
git commit --no-verify -m "wip: checkpoint"
git push --no-verify
```

---

## Custom Merge Driver

Chant includes a custom git merge driver that automatically resolves frontmatter conflicts in spec files.

### What It Does

When merging spec branches back to main, frontmatter conflicts commonly occur. The merge driver:

- Intelligently merges status, completed_at, and model fields
- Preserves implementation content
- Prefers "completed" status over "in_progress"
- Merges lists (commits, labels, target_files) with deduplication

### Installation

1. Add to `.gitattributes` in your repository root:
   ```
   .chant/specs/*.md merge=chant-spec
   ```

2. Configure git:
   ```bash
   git config merge.chant-spec.driver "chant merge-driver %O %A %B"
   git config merge.chant-spec.name "Chant spec merge driver"
   ```

### Verification

```bash
git config --get merge.chant-spec.driver
grep chant-spec .gitattributes
```
