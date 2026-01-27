# Recovery & Resume

## Philosophy

Failures happen. Chant should:
1. Detect failures automatically
2. Preserve work where possible
3. Make recovery explicit and safe
4. Never lose data (git tracks everything)

## Failure Scenarios

### 1. Agent Crash Mid-Work

**What happens:**
- Agent process dies unexpectedly
- Spec status still `in_progress`
- Lock file exists but process is gone
- Uncommitted changes in clone/worktree

**Detection:**
```rust
fn is_stale_lock(lock: &LockFile) -> bool {
    if lock.hostname != current_hostname() {
        // Different machine - check time
        lock.started_at < now() - Duration::hours(4)
    } else {
        // Same machine - check if PID exists
        !process_exists(lock.pid)
    }
}
```

**Recovery:**
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

Choice: R

Resuming spec 001...
```

**Resume behavior:**
- Checks clone/worktree for uncommitted changes
- If changes exist, asks to keep or discard
- Acquires new lock
- Restarts agent with existing state

### 2. Machine Crash / Reboot

**What happens:**
- Entire machine goes down
- All locks become stale
- Clones/worktrees may have uncommitted work

**Detection:**
- On startup, scan for stale locks
- Check each lock's PID

**Recovery:**
```bash
$ chant status
Warning: Found 2 specs with stale locks

  001 - locked by PID 12345 (not running)
  002 - locked by PID 12346 (not running)

Run 'chant recover' to handle these.

$ chant recover
Spec 001:
  Clone: .chant/.clones/001
  Uncommitted changes: 3 files
  Last commit: "chant(001): partial implementation"

  [R] Resume  [D] Discard  [S] Skip

Spec 002:
  Clone: .chant/.clones/002
  No uncommitted changes
  Status: in_progress (no commits)

  [R] Restart  [D] Discard  [S] Skip
```

### 3. Network Failure During Push

**What happens:**
- Agent completed work
- Committed locally
- Push to remote failed
- Spec marked complete but branch not pushed

**Detection:**
```rust
fn verify_spec_completion(spec: &Spec) -> VerifyResult {
    if spec.status != "completed" {
        return VerifyResult::NotComplete;
    }

    // Check if branch exists on remote
    let branch = format!("chant/{}", spec.id);
    if !remote_branch_exists(&branch) {
        return VerifyResult::NotPushed;
    }

    // Check if commit matches
    if spec.commit != remote_head(&branch) {
        return VerifyResult::CommitMismatch;
    }

    VerifyResult::Ok
}
```

**Recovery:**
```bash
$ chant verify 001
Spec 001: NOT PUSHED

Local commit: abc123
Remote branch: does not exist

Fix:
  cd .chant/.clones/001
  git push origin chant/001

Or: chant push 001
```

### 4. Daemon Crash

**What happens:**
- Daemon process dies
- Workers lose connection
- Queue state in memory is lost

**Recovery:**
- Queue is derived from files - rebuilds on daemon restart
- Workers retry connection
- In-progress tasks continue (they have locks)

```bash
$ chant daemon start
Rebuilding queue from spec files...
Found 42 ready specs
Found 3 in-progress specs (locks held)
Daemon ready.
```

**No data loss** - daemon state is optimization, files are truth.

### 5. Git Conflict on Merge

**What happens:**
- Agent completed spec on branch
- Main branch changed while agent worked
- Merge has conflicts

**Detection:**
```bash
$ chant merge 001
Merge conflict in src/api/handler.go

Conflicting changes:
  - main: lines 42-50 (added validation)
  - chant/001: lines 42-55 (added auth check)
```

**Recovery options:**

```bash
# Option 1: Auto-resolve with agent (recommended)
$ chant merge 001 --rebase --auto
# Agent resolves conflicts automatically

# Option 2: Rebase without auto-resolve
$ chant merge 001 --rebase
# If conflicts: rebase is aborted, spec skipped
# Manually resolve, then retry

# Option 3: Resume failed spec and re-run
$ chant resume 001 --work
# Resets to pending and re-executes on current main

# Option 4: Manual resolution
$ git checkout chant/001
$ git rebase main
# ... resolve conflicts ...
$ git checkout main
$ git merge --ff-only chant/001
```

### 6. Agent Failure (Non-Crash)

**What happens:**
- Agent runs but fails to complete spec
- Tests fail, or agent gives up
- Spec marked `failed`

**Detection:**
- Agent exits with error
- Spec status set to `failed`
- Error captured in spec file

```yaml
---
status: failed
error: "Tests failed: 2 assertions"
failed_at: 2026-01-22T12:30:00Z
attempts: 1
---
```

**Recovery:**
```bash
$ chant retry 001
# Creates new attempt
# Keeps history of previous attempt

$ chant show 001
Status: in_progress
Attempt: 2
Previous attempts:
  1. failed - Tests failed (2026-01-22 12:30)
```

### 7. Uncommitted New Files

**What happens:**
- Agent created new files but didn't add them
- Commit doesn't include new files
- Spec appears complete but work is missing

**Prevention:**
```markdown
# In prompt
Before committing:
- Run `git status` to check for untracked files
- Add all new files: `git add <new-files>`
- Verify with `git status` - nothing untracked
```

**Detection:**
```rust
fn post_work_check(clone_path: &Path) -> Vec<Warning> {
    let mut warnings = vec![];

    // Check for untracked files
    let untracked = git_untracked_files(clone_path);
    if !untracked.is_empty() {
        warnings.push(Warning::UncommittedFiles(untracked));
    }

    // Check for unstaged changes
    let unstaged = git_unstaged_changes(clone_path);
    if !unstaged.is_empty() {
        warnings.push(Warning::UnstagedChanges(unstaged));
    }

    warnings
}
```

**Recovery:**
```bash
$ chant complete 001
Warning: Uncommitted files in clone:
  - src/api/new_handler.go (new file)
  - src/api/types.go (modified)

[A] Add and amend commit
[I] Ignore (mark complete anyway)
[C] Cancel

Choice: A
Adding files and amending commit...
Done.
```

### 8. Wrong Branch Base

**What happens:**
- Spec started from old main
- Main has moved significantly
- Big merge conflict expected

**Detection:**
```bash
$ chant status 001
Spec: 001
Status: in_progress
Branch: chant/001
Base: main @ abc123 (5 days ago, 47 commits behind)

Warning: Branch is significantly behind main.
Consider: chant rebase 001
```

**Recovery:**
```bash
$ chant rebase 001
Rebasing chant/001 onto current main...

Conflict in src/api/handler.go
[R] Resolve manually
[A] Abort rebase, retry spec from scratch
[S] Skip (keep old base)
```

## Resume Commands

```bash
# Check for recovery-needed tasks
chant recover --check

# Interactive recovery
chant recover

# Resume specific spec
chant resume 001

# Retry failed spec
chant retry 001

# Force unlock (dangerous)
chant unlock 001 --force

# Verify all completed tasks
chant verify --all

# Push unpushed completions
chant push --all
```

## State Diagram

```
                    chant work
                         │
                         ▼
                   ┌───────────┐
                   │  pending  │
                   └─────┬─────┘
                         │
            ┌────────────┴────────────┐
            │                         │
            ▼                         ▼
     ┌─────────────┐          ┌─────────────┐
     │ in_progress │          │   blocked   │
     └──────┬──────┘          └─────────────┘
            │
     ┌──────┴──────┬──────────────┐
     │             │              │
     ▼             ▼              ▼
┌─────────┐  ┌──────────┐  ┌───────────┐
│completed│  │  failed  │  │  crashed  │
└─────────┘  └────┬─────┘  └─────┬─────┘
                  │              │
                  │   chant      │   chant
                  │   retry      │   resume
                  │              │
                  └──────┬───────┘
                         │
                         ▼
                   ┌───────────┐
                   │in_progress│
                   └───────────┘
```

## Recovery Configuration

```yaml
# config.md
recovery:
  stale_lock_timeout: 4h      # When to consider lock stale
  auto_resume: false          # Auto-resume on stale lock
  preserve_clones: true       # Keep clones after completion
  clone_retention: 7d         # How long to keep completed clones

  post_work_checks:
    untracked_files: warn     # error | warn | ignore
    unstaged_changes: error
    unpushed_commits: warn
```

## Logging for Recovery

Spec file captures recovery-relevant info:

```yaml
---
status: failed
attempts:
  - started: 2026-01-22T10:00:00Z
    ended: 2026-01-22T10:30:00Z
    result: failed
    error: "Tests failed"
    commit: abc123

  - started: 2026-01-22T11:00:00Z
    ended: null
    result: crashed
    error: "Process killed"
    commit: def456

last_checkpoint:
  at: 2026-01-22T11:15:00Z
  files_modified: [src/api/handler.go]
  tests_passing: false
---
```

## Rollback / Undo

### Undo a Completed Spec

Spec completed but changes were wrong:

```bash
$ chant undo 001
Spec 001 completed at 2026-01-22 15:30
Commit: abc123 "chant(001): Add authentication"

Undo options:
  [R] Revert commit (creates new commit undoing changes)
  [B] Reset branch (removes commit, keeps in reflog)
  [M] Manual (show what to do)

Choice: R

Creating revert commit...
Commit def456: "Revert chant(001): Add authentication"

Spec 001 status: reverted
```

### Revert vs Reset

| Method | Effect | Safety |
|--------|--------|--------|
| Revert | New commit undoing changes | Safe (history preserved) |
| Reset | Remove commit from history | Dangerous (requires force push) |

**Recommendation**: Always use revert unless you haven't pushed yet.

### Undo Multiple Specs

```bash
$ chant undo 001 002 003
Will revert (in reverse order):
  003 - Add tests (abc123)
  002 - Add validation (def456)
  001 - Add auth (ghi789)

Continue? [y/N]
```

### Partial Undo

Undo specific files from a spec:

```bash
$ chant undo 001 --files src/auth/middleware.go
Reverting only src/auth/middleware.go from commit abc123...
Commit xyz789: "Partial revert of chant(001): middleware.go"
```

### Undo in Different Scenarios

**If branch not merged:**
```bash
$ chant undo 001
Branch chant/001 not merged to main.
Delete branch? [y/N]
```

**If PR exists:**
```bash
$ chant undo 001
PR #42 exists for this spec.
Close PR and delete branch? [y/N]
```

**If already in production:**
```bash
$ chant undo 001
Warning: Commit abc123 is in main branch.
This will create a revert commit in main.
Continue? [y/N]
```

## Partial Completion & Checkpoints

### The Problem

Agent works for 30 minutes, crashes at 90% complete. All work lost?

### Checkpointing

Agent should commit incrementally:

```markdown
# In prompt
## Commit Strategy

Commit your work frequently:
- After each logical unit of work
- Before running tests
- Before making risky changes

Use checkpoint commits:
  git commit -m "chant(001): checkpoint - auth middleware done"

Final commit will squash if configured.
```

### Checkpoint Detection

```yaml
---
status: in_progress
checkpoints:
  - at: 2026-01-22T10:15:00Z
    commit: abc123
    message: "checkpoint - auth middleware done"
  - at: 2026-01-22T10:30:00Z
    commit: def456
    message: "checkpoint - validation added"
---
```

### Recovery from Checkpoint

```bash
$ chant resume 001
Spec 001 crashed.
Found 2 checkpoints:

  1. abc123 - checkpoint - auth middleware done (10:15)
  2. def456 - checkpoint - validation added (10:30)

Resume from checkpoint 2? [y/N]

Resuming from def456...
Agent starting with existing work...
```

### Checkpoint Config

```yaml
# config.md
checkpoints:
  enabled: true
  interval: 10m              # Remind agent to checkpoint every 10m
  auto_commit: false         # Don't auto-commit (let agent decide)

  # On completion
  squash: true               # Squash checkpoints into single commit
  keep_history: false        # Don't keep checkpoint commits
```

### Squash on Completion

```bash
$ chant complete 001
Spec has 5 checkpoint commits.
Squashing into single commit...

Final commit: xyz789 "chant(001): Add authentication"
```

### Progress Tracking

Track progress even without commits:

```yaml
---
status: in_progress
progress:
  files_modified: [src/auth/middleware.go, src/auth/jwt.go]
  tests_status: 3/5 passing
  estimated_completion: 80%
  last_activity: 2026-01-22T10:45:00Z
---
```

## Conflict Resolution (Detailed)

### Conflict Types

| Type | Cause | Resolution |
|------|-------|------------|
| **Merge conflict** | Main changed while agent worked | Manual or retry |
| **Logical conflict** | Two tasks modify same logic | Human review |
| **Test conflict** | Merged code breaks tests | Fix or revert |
| **Dependency conflict** | Dependency spec was reverted | Re-run or update deps |

### Automatic Resolution

Some conflicts can be auto-resolved:

```yaml
# config.md
conflicts:
  auto_resolve:
    # Accept theirs for generated files
    patterns:
      - "*.lock"
      - "*.generated.*"
    strategy: theirs

    # Accept ours for spec-specific files
    # (spec knows better)
    spec_files: ours
```

### Resolution Strategies

```bash
$ chant merge 001
Conflict in src/api/handler.go

Strategies:
  [O] Ours - keep spec's changes
  [T] Theirs - keep main's changes
  [M] Manual - open editor to resolve
  [R] Retry - re-run spec on current main
  [A] Abort - leave spec as-is

Choice: M
Opening src/api/handler.go in $EDITOR...
```

### Retry with Context

Re-running with knowledge of conflict:

```bash
$ chant retry 001 --with-context
Retrying spec 001...

Agent will be informed:
  - Previous attempt conflicted with main
  - Conflicting file: src/api/handler.go
  - Main's changes: added rate limiting (commit xyz789)

Agent can incorporate both changes.
```

### Conflict Prevention

```yaml
# config.md
conflicts:
  prevention:
    # Warn before starting if files recently changed
    warn_if_modified: 1h

    # Lock files during work (team mode)
    advisory_locks: true

    # Rebase before completing
    auto_rebase: true
```

```bash
$ chant work 001
Warning: Target file recently modified:
  src/api/handler.go - changed 30m ago by spec 002

Start anyway? [y/N]
```

## Offline Mode

### Use Case

Working without network:
- On a plane
- Poor connectivity
- Air-gapped environments

### What Works Offline

| Feature | Offline | Notes |
|---------|---------|-------|
| Create specs | ✓ | Local files |
| Edit specs | ✓ | Local files |
| List/search | ✓ | Local index |
| Work (local LLM) | ✓ | Local provider |
| Work (cloud LLM) | ✗ | Needs network |
| Push branches | ✗ | Needs network |
| Create PRs | ✗ | Needs network |
| Sync SCM | ✗ | Needs network |

### Offline Workflow

```bash
# Create specs offline
$ chant add "Fix auth bug"
Created 001 (offline, will sync later)

# Work with local LLM
$ chant work 001 --provider ollama --model codellama
Working offline with ollama/codellama...

# Queue for sync
$ chant push 001
No network. Queued for push when online.

# Later, when online
$ chant sync
Pushing 3 queued branches...
Creating 2 queued PRs...
Syncing 5 specs from GitHub...
Done.
```

### Offline Queue

```yaml
# .chant/.offline/queue.json
{
  "pending_push": ["001", "002"],
  "pending_pr": ["001"],
  "pending_sync": true,
  "queued_at": "2026-01-22T10:00:00Z"
}
```

### Sync on Reconnect

```yaml
# config.md
offline:
  auto_sync: true           # Sync when network returns
  sync_interval: 5m         # Check for network every 5m
  notify_on_sync: true      # Notify when sync completes
```

### Conflict on Sync

Offline work may conflict with remote changes:

```bash
$ chant sync
Syncing...

Conflict: Spec 001
  Local: completed, commit abc123
  Remote: modified by alice (added requirement)

Options:
  [L] Keep local (ignore remote changes)
  [R] Keep remote (discard local work)
  [M] Merge (apply local work to updated spec)
```

### Offline Detection

```rust
fn is_offline() -> bool {
    // Try to reach the provider endpoint
    match reqwest::blocking::get(&config.provider_health_url) {
        Ok(resp) if resp.status().is_success() => false,
        _ => true,
    }
}

fn work_task(spec: &Spec) -> Result<()> {
    if is_offline() && spec.provider.requires_network() {
        // Suggest local alternative
        suggest_offline_provider()?;
    }
    // ...
}
```

## Best Practices

1. **Small specs** - Easier to recover, less work lost
2. **Frequent commits** - Agent should commit incrementally
3. **Target files** - Declare expected files to detect missing work
4. **Post-work hooks** - Verify completion before marking done
5. **Clone retention** - Keep clones briefly for manual recovery
6. **Checkpoints** - Enable checkpointing for long specs
7. **Offline provider** - Configure local provider for offline work
