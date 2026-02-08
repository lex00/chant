# Recovery & Resume

## Approach

Failures happen. Chant detects failures, preserves work, makes recovery explicit, and never loses data (git tracks everything).

See [philosophy](../getting-started/philosophy.md) for chant's broader design principles.

## Quick Reference

```bash
chant recover --check     # Check for recovery-needed specs
chant recover             # Interactive recovery
chant resume 001          # Resume specific spec
chant retry 001           # Retry failed spec
chant unlock 001 --force  # Force unlock (dangerous)
chant verify --all        # Verify all completed specs
chant push --all          # Push unpushed completions
```

## Failure Scenarios

### Agent Crash Mid-Work

```bash
$ chant work 001
Warning: Stale lock detected

Lock info:
  PID: 12345 (not running)
  Started: 2026-01-22 10:00:00 (2 hours ago)

Options:
  [R] Resume - keep existing work, continue
  [C] Clean - discard, start fresh
  [A] Abort - do nothing
```

Detection: PID file exists but process is gone.

### Machine Reboot

```bash
$ chant list --summary
Warning: Found 2 specs with stale locks

$ chant recover
Spec 001:
  Clone: .chant/.clones/001
  Uncommitted changes: 3 files
  [R] Resume  [D] Discard  [S] Skip
```

Detection: On startup, scan for stale locks.

### Network Failure During Push

```bash
$ chant verify 001
Spec 001: NOT PUSHED

Local commit: abc123
Remote branch: does not exist

Fix: chant push 001
```

### Git Conflict on Merge

```bash
$ chant merge 001
Error: Content conflicts detected

Files with conflicts:
  - src/api/handler.go

Next steps:
  1. Resolve manually: git merge --continue
  2. Auto-rebase: chant merge 001 --rebase --auto
  3. Abort: git merge --abort
```

Conflict types: fast-forward, content, tree.

### Agent Failure (Non-Crash)

```yaml
# Spec frontmatter
---
status: failed
error: "Tests failed: 2 assertions"
failed_at: 2026-01-22T12:30:00Z
attempts: 1
---
```

Recovery: `chant retry 001`

## State Diagram

```
pending ──→ in_progress ──┬──→ completed
     ↑                    ├──→ failed ────→ in_progress (retry)
     │                    └──→ crashed ───→ in_progress (resume)
     └────────────────────────────────────────────────────┘
```

## Rollback / Undo

```bash
# Undo completed spec (creates revert commit)
chant undo 001

# Undo specific files
chant undo 001 --files src/auth/middleware.go

# Undo multiple specs (reverse order)
chant undo 001 002 003
```

Prefer **revert** (safe, history preserved) over **reset** (dangerous, requires force push).

## Checkpoints

Enable incremental commits for long-running specs:

```markdown
# In prompt
Commit frequently:
  git commit -m "chant(001): checkpoint - auth middleware done"
```

```yaml
# Spec tracks checkpoints
---
checkpoints:
  - at: 2026-01-22T10:15:00Z
    commit: abc123
    message: "checkpoint - auth middleware done"
---
```

Resume from checkpoint:
```bash
$ chant resume 001
Found 2 checkpoints. Resume from latest? [y/N]
```

## Configuration

```yaml
# config.md
recovery:
  stale_lock_timeout: 4h

  post_work_checks:
    untracked_files: warn
    unstaged_changes: error
    unpushed_commits: warn

checkpoints:
  enabled: true
  squash: true              # Squash into single commit on completion

conflicts:
  prevention:
    warn_if_modified: 1h    # Warn if target files recently changed
    auto_rebase: true       # Rebase before completing
```

## Best Practices

1. **Small specs** - Easier to recover, less work lost
2. **Frequent commits** - Agent should commit incrementally
3. **Target files** - Declare expected files to detect missing work
4. **Checkpoints** - Enable for long specs
