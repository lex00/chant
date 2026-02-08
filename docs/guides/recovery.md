# Recovery

Things go wrong. Agents crash, tests fail, merges conflict. This guide walks through common failure scenarios and how to recover from each.

## Scenario: Agent Fails a Spec

The most common failure. An agent runs, encounters a problem (tests fail, can't meet acceptance criteria), and the spec ends up in `failed` status:

```bash
$ chant show 001

ID:     2026-02-08-001-xyz
Status: failed
Title:  Add CSV export handler
```

Check the log to understand what went wrong:

```bash
$ chant log 001

[2026-02-08 14:32:00] Running tests...
[2026-02-08 14:32:15] ✗ 2 tests failed
[2026-02-08 14:32:15] Agent exiting with failure
```

If the problem is in the spec (unclear requirements, wrong approach), edit it first:

```bash
$ chant edit 001
```

Then reset and retry:

```bash
$ chant reset 001
Spec 001-xyz reset to pending

$ chant work 001
Working 001-xyz: Add CSV export handler
...
✓ Completed in 1m 30s (attempt 2)
```

Or do both in one step:

```bash
$ chant reset 001 --work
```

The retry counter increments each time — the agent sees it's on attempt 2 and can try a different approach.

## Scenario: Agent Killed (OOM, SIGKILL)

Sometimes the agent process gets killed by the OS (memory pressure, too many Claude processes). The spec stays `in_progress` with no agent running, and a stale worktree may be left behind.

Diagnose the state:

```bash
$ chant diagnose 001

Spec: 001-xyz
Status: in_progress
Lock: stale (PID 12345 not running)
Worktree: exists at /tmp/chant-001-xyz
  Uncommitted changes: 3 files
Branch: chant/2026-02-08-001-xyz (ahead of main by 2 commits)
```

The worktree may contain useful partial work. If you want to preserve it, check the branch manually before cleanup:

```bash
$ git -C /tmp/chant-001-xyz log --oneline main..HEAD
abc1234 Add CSV formatter
def5678 Update export module
```

Reset the spec and retry — the agent gets a fresh worktree:

```bash
$ chant reset 001 --work
```

Clean up the orphaned worktree:

```bash
$ chant cleanup
Found 1 orphan worktree:
  /tmp/chant-001-xyz (spec: 001-xyz, stale)

Remove? [y/N] y
Cleaned 1 worktree
```

## Scenario: Merge Conflict

After parallel execution, merging branches back to main can conflict if two specs touched the same files:

```bash
$ chant merge --all-completed

Merging 001-xyz... ✓
Merging 002-xyz... ✗ conflict in src/lib.rs

Resolve manually:
  cd /tmp/chant-002-xyz
  # fix conflicts
  git add src/lib.rs
  git commit
  # then retry merge
```

Or use rebase to replay on top of the first merge:

```bash
$ git -C /tmp/chant-002-xyz rebase main
# resolve conflicts
$ chant merge 002
```

The `--no-merge` flag on `chant work --parallel` skips auto-merge entirely, leaving branches for manual review:

```bash
$ chant work --parallel --no-merge
```

## Scenario: Stale State After Reboot

Your machine reboots mid-work. Lock files, worktrees, and process records are left behind. On your next session:

```bash
$ chant cleanup --dry-run
Would remove:
  Worktree: /tmp/chant-001-xyz (stale, no running process)
  Worktree: /tmp/chant-003-xyz (stale, no running process)

$ chant cleanup --yes
Cleaned 2 worktrees
```

Any specs left `in_progress` need manual reset:

```bash
$ chant list --status in_progress

ID          Type  Status       Title
001-xyz     code  in_progress  Add CSV export handler
003-xyz     code  in_progress  Fix edge case

$ chant reset 001
$ chant reset 003
```

Then re-execute:

```bash
$ chant work --chain
```

## Scenario: Wrong Approach

The agent completed work, but the approach is wrong — you want to redo it differently. Edit the spec with guidance and reset:

```bash
$ chant edit 001
# Add: "Use pessimistic locking, not optimistic. See src/lock.rs for the existing Lock module."

$ chant reset 001 --work
```

The agent sees the updated spec and takes the new direction.

## Recovery Principles

| Principle | How chant implements it |
|-----------|------------------------|
| **Never lose data** | Git tracks everything — branches, commits, worktree state |
| **Make recovery explicit** | `reset` is deliberate, not automatic |
| **Preserve partial work** | Branches survive agent crashes; inspect before cleanup |
| **Fail fast in parallel** | Chain mode stops on first failure |
| **Track attempts** | Retry counter in frontmatter lets agents adapt |

## Key Commands

| Command | When to use |
|---------|-------------|
| `chant reset <id>` | Reset a failed/stuck spec to pending |
| `chant reset <id> --work` | Reset and immediately re-execute |
| `chant cleanup` | Remove orphan worktrees and stale artifacts |
| `chant cleanup --dry-run` | Preview what would be cleaned |
| `chant diagnose <id>` | Inspect a spec's state (lock, worktree, branch) |
| `chant log <id>` | Read the agent's execution log |

## Further Reading

- [Lifecycle](../concepts/lifecycle.md) — State transitions including failed and recovery
- [CLI Reference](../reference/cli.md) — Full command documentation
