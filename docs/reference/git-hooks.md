# Git Hooks

> **Status: Partially Implemented** ⚠️
>
> Basic git hook scripts (pre-commit, commit-msg, post-commit) can be manually set up.
> The `chant hooks generate/install/remove/run/list` CLI commands are not yet implemented.
> Daemon-based git watching is also not implemented. See [Roadmap](../roadmap/roadmap.md) for future plans.

## Philosophy

Git hooks enhance the Chant workflow. They are **optional** - no Chant feature depends on them.

```
With hooks:    Automated validation, status updates, can block
Without hooks: Manual validation, explicit commands, still works
```

## Useful Hooks

| Hook | Purpose | Chant Use |
|------|---------|-----------|
| `pre-commit` | Validate before commit | Lint spec files |
| `commit-msg` | Validate commit message | Enforce `chant(id): msg` format |
| `post-commit` | After commit completes | Update spec status |
| `pre-push` | Before push to remote | Warn about incomplete specs |
| `post-merge` | After merge/pull | Rebuild index |
| `post-checkout` | After branch switch | Rebuild index |

## Hook Implementations

### pre-commit

Validate spec files before commit:

```bash
#!/bin/sh
# .git/hooks/pre-commit

# Get staged spec files
staged_specs=$(git diff --cached --name-only -- '.chant/specs/*.md')

if [ -n "$staged_specs" ]; then
    # Lint staged tasks
    echo "$staged_specs" | xargs chant lint --files
    if [ $? -ne 0 ]; then
        echo "Spec validation failed. Fix errors or use --no-verify"
        exit 1
    fi
fi
```

### commit-msg

Enforce commit message format:

```bash
#!/bin/sh
# .git/hooks/commit-msg

msg=$(cat "$1")

# Check for chant format: chant(id): message
if echo "$msg" | grep -qE '^chant\([a-z0-9-]+\):'; then
    # Extract spec ID and verify it exists
    spec_id=$(echo "$msg" | sed -E 's/^chant\(([a-z0-9-]+)\):.*/\1/')
    if ! chant show "$spec_id" >/dev/null 2>&1; then
        echo "Warning: Spec $spec_id not found"
        # Warning only, don't block
    fi
fi

# Allow non-chant commits (not everything is a spec)
exit 0
```

### post-commit

Update spec status after commit:

```bash
#!/bin/sh
# .git/hooks/post-commit

# Get commit message
msg=$(git log -1 --format=%s)

# If chant commit, record the commit hash
if echo "$msg" | grep -qE '^chant\([a-z0-9-]+\):'; then
    spec_id=$(echo "$msg" | sed -E 's/^chant\(([a-z0-9-]+)\):.*/\1/')
    commit=$(git rev-parse HEAD)

    # Update spec with commit hash (if spec exists)
    chant update "$spec_id" --commit "$commit" 2>/dev/null || true
fi
```

### pre-push

Warn about incomplete specs on branch:

```bash
#!/bin/sh
# .git/hooks/pre-push

branch=$(git rev-parse --abbrev-ref HEAD)

# Check for chant branch pattern
if echo "$branch" | grep -qE '^chant/'; then
    spec_id=$(echo "$branch" | sed 's/^chant\///')

    status=$(chant show "$spec_id" --format status 2>/dev/null)
    if [ "$status" != "completed" ]; then
        echo "Warning: Spec $spec_id is not completed (status: $status)"
        echo "Push anyway? [y/N]"
        read -r response
        if [ "$response" != "y" ]; then
            exit 1
        fi
    fi
fi

exit 0
```

### post-merge / post-checkout

Re-validate specs after changes:

```bash
#!/bin/sh
# .git/hooks/post-merge (or post-checkout)

# Re-lint specs after merge/checkout
chant lint 2>/dev/null || true
```

## Hook Manager Integration (Planned)

> **Status: Planned** - The `chant hooks generate/install/remove/run/list` CLI commands are on the roadmap but not yet implemented. For now, set up git hooks manually using the scripts in the [Hook Implementations](#hook-implementations) section above.

When implemented, chant will generate configs for popular hook managers (Husky, Lefthook, pre-commit, Overcommit) and provide CLI commands for hook management. See [Roadmap](../roadmap/roadmap.md) for details.

## What Hooks Don't Do

Hooks are convenience, not enforcement:

| Feature | With Hooks | Without Hooks |
|---------|------------|---------------|
| Spec validation | Automatic on commit | `chant lint` manually |
| Commit recording | Automatic | `chant update --commit` manually |
| Status updates | Automatic | Explicit state changes |

**No Chant feature requires hooks.** Everything works with explicit commands.

## Team Setup

For teams, commit hook scripts to the repository and use a hook manager like Lefthook or Husky:

```bash
# Example with Lefthook (manual setup)
# Create lefthook.yml with chant-lint commands
# Team members run: lefthook install
```

## Skipping Hooks

When you need to bypass:

```bash
git commit --no-verify -m "wip: checkpoint"
git push --no-verify
```

Hooks should help, not block. Warning > error for most cases.

## Summary

| Setup | Best For |
|-------|----------|
| **Hooks** | CI enforcement, strict teams. Blocks bad commits/pushes. Requires setup per machine. |
| **No hooks** | Quick projects. Manual `chant lint`, explicit commands. |

**No Chant feature requires hooks.** Everything works with explicit commands.
