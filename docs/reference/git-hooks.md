# Git Hooks

## Philosophy

Git hooks enhance the Chant workflow. They are **optional** - no Chant feature depends on them.

```
With hooks:    Automated validation, status updates, can block
With daemon:   Same automation, cannot block, no setup
Without both:  Manual validation, explicit commands, still works
```

## Daemon vs Hooks

The daemon provides most hook functionality automatically. See [daemon.md](../scale/daemon.md#git-integration).

| Want to... | Use |
|------------|-----|
| Auto-record commits | Daemon (automatic) or hooks |
| Rebuild index on merge | Daemon (automatic) or hooks |
| Validate on save | Daemon (automatic) |
| **Block bad commits** | Hooks only |
| **Block pushes** | Hooks only |

**Use hooks when you need to block operations.** Otherwise, daemon handles it.

```yaml
# If you have daemon, you probably don't need hooks
scale:
  daemon:
    git_watch:
      enabled: true   # Handles most hook functionality
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

Rebuild index after changes:

```bash
#!/bin/sh
# .git/hooks/post-merge (or post-checkout)

# Only if daemon is running
if chant daemon status >/dev/null 2>&1; then
    chant daemon reindex
fi
```

## Hook Managers

Chant generates configs for popular hook managers.

### Husky (Node.js)

```bash
chant hooks generate --manager husky
```

Creates `.husky/` directory:

```
.husky/
  pre-commit
  commit-msg
  post-commit
```

And updates `package.json`:

```json
{
  "scripts": {
    "prepare": "husky install"
  }
}
```

### Lefthook (Go)

```bash
chant hooks generate --manager lefthook
```

Creates `lefthook.yml`:

```yaml
pre-commit:
  commands:
    chant-lint:
      glob: ".chant/specs/*.md"
      run: chant lint --files {staged_files}

commit-msg:
  commands:
    chant-verify:
      run: |
        msg=$(cat {1})
        if echo "$msg" | grep -qE '^chant\([a-z0-9-]+\):'; then
          spec_id=$(echo "$msg" | sed -E 's/^chant\(([a-z0-9-]+)\):.*/\1/')
          chant show "$spec_id" >/dev/null 2>&1 || echo "Warning: Spec not found"
        fi

post-commit:
  commands:
    chant-record:
      run: |
        msg=$(git log -1 --format=%s)
        if echo "$msg" | grep -qE '^chant\([a-z0-9-]+\):'; then
          spec_id=$(echo "$msg" | sed -E 's/^chant\(([a-z0-9-]+)\):.*/\1/')
          commit=$(git rev-parse HEAD)
          chant update "$spec_id" --commit "$commit" 2>/dev/null || true
        fi
```

### pre-commit (Python)

```bash
chant hooks generate --manager pre-commit
```

Creates `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: chant-lint
        name: Chant spec validation
        entry: chant lint --files
        language: system
        files: '^\.chant/specs/.*\.md$'
        pass_filenames: true
```

### Native Git Hooks

```bash
chant hooks generate --manager native
```

Creates `.git/hooks/` scripts directly (not recommended for teams - not version controlled).

### Overcommit (Ruby)

```bash
chant hooks generate --manager overcommit
```

Creates `.overcommit.yml`:

```yaml
PreCommit:
  ChantLint:
    enabled: true
    command: ['chant', 'lint', '--files']
    include: '.chant/specs/*.md'

CommitMsg:
  ChantVerify:
    enabled: true
    command: ['chant', 'hooks', 'verify-commit-msg']
```

## CLI Commands

```bash
# Generate hook configs
chant hooks generate --manager husky
chant hooks generate --manager lefthook
chant hooks generate --manager pre-commit
chant hooks generate --manager native

# Install hooks (runs manager's install)
chant hooks install

# Remove hooks
chant hooks remove

# Run hook manually (for testing)
chant hooks run pre-commit
chant hooks run commit-msg "chant(001): fix bug"

# List available hooks
chant hooks list
```

## Configuration

```yaml
# config.md
hooks:
  manager: lefthook        # husky, lefthook, pre-commit, native, none

  pre_commit:
    enabled: true
    lint: true             # Validate spec files

  commit_msg:
    enabled: true
    verify_task: true      # Warn if spec ID not found

  post_commit:
    enabled: true
    record_commit: true    # Update spec with commit hash

  pre_push:
    enabled: false         # Off by default (can be annoying)
    warn_incomplete: true
```

## What Hooks Don't Do

Hooks are convenience, not enforcement:

| Feature | With Hooks | Without Hooks |
|---------|------------|---------------|
| Spec validation | Automatic on commit | `chant lint` manually |
| Commit recording | Automatic | `chant update --commit` manually |
| Status updates | Automatic | Explicit state changes |

**No Chant feature requires hooks.** Everything works with explicit commands.

## Team Setup

For teams, commit the hook manager config:

```bash
# Generate and commit
chant hooks generate --manager lefthook
git add lefthook.yml
git commit -m "chore: add chant git hooks"

# Team members run once
lefthook install   # or: npm run prepare (husky)
```

## Custom Hooks

Add your own alongside chant hooks:

```yaml
# lefthook.yml
pre-commit:
  commands:
    chant-lint:
      # ... chant's hook

    your-linter:
      run: your-custom-linter {staged_files}
```

## Skipping Hooks

When you need to bypass:

```bash
git commit --no-verify -m "wip: checkpoint"
git push --no-verify
```

Hooks should help, not block. Warning > error for most cases.

## Summary: When to Use What

| Setup | Best For |
|-------|----------|
| **Daemon only** | Solo dev, small teams. Automatic, no setup, warns but doesn't block. |
| **Hooks only** | CI enforcement, strict teams. Blocks bad commits/pushes. Requires setup per machine. |
| **Both** | Enterprise. Daemon for convenience, hooks for enforcement. |
| **Neither** | Quick projects. Manual `chant lint`, explicit commands. |

```
Daemon running?
  │
  ├─ Yes → Git integration automatic
  │         └─ Want to BLOCK? → Also install hooks
  │
  └─ No → Install hooks for automation
           └─ Or just use manual commands
```
