# Data Lifecycle

## What Data Exists

| Data | Location | Tracked | Retention |
|------|----------|---------|-----------|
| Specs | `.chant/specs/*.md` | Git | Forever (git history) |
| Prompts | `.chant/prompts/*.md` | Git | Forever |
| Config | `.chant/config.md` | Git | Forever |
| Locks | `.chant/.locks/*.lock` | No | Until released |
| Index | `.chant/.store/` | No | Rebuilt on demand |
| Logs | `.chant/logs/` | Optional | Configurable |

## Spec Lifecycle

```
┌──────────┐
│ Created  │  chant add "Fix bug"
└────┬─────┘
     │
     ▼
┌──────────┐
│ Pending  │  Waiting for work or dependencies
└────┬─────────────────────┐
     │                     │
     │                     │ has unmet
     │                     │ dependencies
     │                     ▼
     │                ┌──────────┐
     │                │ Blocked  │  Waiting for depends_on
     │                └────┬─────┘
     │                     │
     │                     │ dependencies
     │                     │ complete
     │                     ▼
     │                ┌──────────┐
     │                │ Pending  │
     │                └──────────┘
     │
     │ (if --needs-approval)
     │
     ├──────────────────────────────┐
     │                              │
     │                              ▼
     │                    ┌───────────────────┐
     │                    │ Approval Required │
     │                    └────┬──────────┬───┘
     │                         │          │
     │              approved   │          │ rejected
     │                         ▼          ▼
     │                    ┌────────┐ ┌──────────┐
     │                    │Approved│ │ Rejected │
     │                    └────┬───┘ └──────────┘
     │                         │
     │◄────────────────────────┘
     │
     ▼
┌──────────┐
│In Progress│  Agent working
└────┬─────┘
     │
     ├──────────────┐
     ▼              ▼
┌──────────┐  ┌──────────┐
│Completed │  │  Failed  │
└────┬─────┘  └────┬─────┘
     │              │
     │              │ retry
     │              ▼
     │        ┌──────────┐
     │        │ Pending  │
     │        └──────────┘
     │
     ▼
┌──────────┐
│ Archived │  (optional, after retention period)
└──────────┘

Cancelled (any status)  ← chant cancel
     │
     └─────► Excluded from lists and work
```

### Approval Gate

When a spec is created with `--needs-approval`, it must be approved before work can begin. See the [Approval Workflow Guide](../guides/approval-workflow.md) for details.

## Spec Statuses

### Pending
- Spec created and ready for work
- Or failed spec that has been retried
- Can have unmet dependencies

### Blocked
- Pending spec with unmet dependencies
- Waiting for one or more specs in `depends_on` to complete
- Status automatically applied when spec is loaded if dependencies incomplete
- Excluded from `chant work` until dependencies complete
- Use `chant list --summary` to see which specs are blocked

### In Progress
- Spec is currently being executed by an agent
- Lock file created in `.chant/.locks/{spec-id}.lock` **before** status transitions to InProgress (prevents race with watch daemon)
- Lock file contains the PID of the managing process
- Lock file removed on completion or failure
- Agent writes status to `.chant-status.json` in worktree (parallel mode)

### Completed
- Spec execution completed successfully
- All acceptance criteria were checked
- Commit hash, timestamp, and model recorded in frontmatter
- Auto-finalized by `chant work` when criteria pass

### Failed
- Spec execution failed (agent error or acceptance criteria unchecked)
- Can be retried with `chant resume {spec-id} --work`
- Can be manually finalized with `chant finalize {spec-id}` if work was done
- Use `chant log {spec-id}` to view agent output

### Cancelled
- Spec soft-deleted with `chant cancel {spec-id}`
- Status changed to `Cancelled`, file preserved
- Excluded from `chant list` and `chant work`
- Can still be viewed with `chant show` or filtered with `chant list --status cancelled`
- Use `chant cancel {spec-id} --delete` if you want to permanently remove the spec file

## Dependency Blocking

Specs can declare dependencies using `depends_on`:

```markdown
---
type: code
status: pending
depends_on:
  - 2026-01-26-001-auth     # This spec depends on 001
---
```

When a spec has unmet dependencies:
1. Status automatically changes to `Blocked`
2. Spec excluded from `chant work` (can't execute)
3. Spec excluded from `chant list` (hidden by default)
4. When all dependencies complete, status reverts to `Pending`
5. Spec becomes available for work

**View blocked specs:**
```bash
chant list --status blocked          # Show all blocked specs
chant list --summary                 # See overview with block reasons
```

## Retention Policies

### Spec Files

Specs stay in `.chant/specs/` forever by default. Git history is the archive.

```yaml
# config.md
lifecycle:
  specs:
    retention: forever         # forever | duration
    # retention: 90d           # Archive after 90 days
```

### Archival (Optional)

If retention is set, completed specs can be archived:

```yaml
lifecycle:
  specs:
    retention: 90d
    archive:
      enabled: true
      location: .chant/archive/
      compress: true           # Gzip archived specs
```

```
.chant/
├── specs/                    # Active specs
│   └── 2026-01-22-001-x7m.md
└── archive/                  # Archived (gitignored or separate branch)
    └── 2025/
        └── 12/
            └── 2025-12-15-042-abc.md.gz
```

**Note**: Archival changes spec IDs (moved file). References break. Use sparingly.


## Data Flow Diagram

```
                    Human
                      │
                      │ chant add
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                     Spec File Created                        │
│                                                              │
│  .chant/specs/2026-01-22-001-x7m.md                         │
│  status: pending                                             │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ git add, git commit
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Git Repository                           │
│                                                              │
│  Permanent history of all spec changes                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ chant work
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Execution                                │
│                                                              │
│  Lock created:  .chant/.locks/{spec-id}.lock                │
│  Worktree created: /tmp/chant-{spec-id}/                    │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ completion
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Cleanup                                  │
│                                                              │
│  Lock released (immediate)                                   │
│  Worktree removed (immediate, after merge)                   │
│  Spec updated (status: completed)                            │
│  Branch merged and deleted                                   │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ long-term
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Long-term                                │
│                                                              │
│  Spec: stays in git history forever                          │
│  Logs: rotated, old logs deleted                             │
│  Branch: deleted after merge                                 │
└─────────────────────────────────────────────────────────────┘
```

## Watch Coordinator

Watch is the **unified lifecycle coordinator** for all work execution. It runs as an ephemeral daemon that handles state transitions, merging, and cleanup.

### Architecture

```
Agent (worktree)                    Watch (main)
────────────────                    ────────────
create worktree + branch
write status: "working"       →     see working → update spec: InProgress
do work, commit to branch
write status: "done"          →     see done → merge → finalize → cleanup
  (or "failed" on error)            (or handle failure)
exit
```

### Auto-Start Behavior

`chant work` commands automatically start watch if not running:

1. `chant work` checks if watch is running via PID file (`.chant/watch.pid`)
2. If not running, spawns `chant watch` as a detached background process
3. Watch writes its PID to `.chant/watch.pid` on startup
4. Watch exits gracefully after idle timeout (default: 5 minutes)

Use `--no-watch` flag to disable auto-start (useful for testing).

### PID Management

- Watch writes PID to `.chant/watch.pid` on startup
- Watch removes PID file on graceful exit (SIGINT, idle timeout, `--once`)
- `is_watch_running()` checks: PID file exists AND process alive AND is chant
- Stale PID file (process dead or wrong process) is automatically cleaned up

### Agent Status File

Agents communicate with watch via `.chant-status.json` in the worktree root:

```json
{
  "spec_id": "2026-02-03-02p-ydf",
  "status": "working",
  "updated_at": "2026-02-03T14:30:00Z",
  "error": null,
  "commits": ["abc123"]
}
```

**Status values:**
- `working` - Agent is actively working
- `done` - Agent completed successfully
- `failed` - Agent encountered an error

### Startup Recovery

On startup, watch recovers from previous crashes:

1. Scans for existing worktrees with `.chant-status.json`
2. `done` status but not merged → queued for merge
3. `working` status but stale (>1 hour) → marked failed
4. Orphaned worktrees (no status file, old) → cleaned up

### Idle Timeout

Watch automatically exits when idle (configurable):

```yaml
# config.md
watch:
  idle_timeout_minutes: 5  # Exit after 5 minutes of no activity
```

Activity is detected when:
- In-progress specs exist
- Active worktrees with agents are found
- Status file changes are observed

## Git Branch Lifecycle

```yaml
lifecycle:
  branches:
    delete_after_merge: true   # Delete chant/* branches after merge
    retain_unmerged: 30d       # Keep unmerged branches for 30d
```

Branches are cleaned up automatically when specs are completed.

## Export

Export spec data in various formats using `chant export`. See [CLI Reference](../reference/cli.md#export) for full details.

```bash
# Export to JSON
chant export --format json > specs.json

# Export to CSV
chant export --format csv > specs.csv

# Export with filters
chant export --status completed --from 2026-01-01
```

## Backup

### Git IS the Backup

Spec files are git-tracked. Normal git backup applies:
- Push to remote
- Mirror to backup location
- Standard git disaster recovery

**Note**: Local state (index, logs, clones) can be rebuilt. Git history is the permanent archive.

## Configuration Reference

```yaml
# config.md
lifecycle:
  specs:
    retention: forever         # forever | 30d | 90d | 1y
    archive:
      enabled: false
      location: .chant/archive/
      compress: true

  branches:
    delete_after_merge: true
    retain_unmerged: 30d
```

---

