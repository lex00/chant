# Data Lifecycle

## What Data Exists

| Data | Location | Tracked | Retention |
|------|----------|---------|-----------|
| Specs | `.chant/specs/*.md` | Git | Forever (git history) |
| Prompts | `.chant/prompts/*.md` | Git | Forever |
| Config | `.chant/config.md` | Git | Forever |
| Locks | `.chant/.locks/*.pid` | No | Until released |
| Index | `.chant/.store/` | No | Rebuilt on demand |
| Clones | `.chant/.clones/` | No | Configurable |
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
- Use `chant status` to see which specs are blocked

### In Progress
- Spec is currently being executed by an agent
- Lock file created in `.chant/.locks/{spec-id}.pid`

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
- Use `chant delete` if you want to permanently remove the spec file

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
chant status                          # See overview with block reasons
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

### Clone Cleanup

Clones from completed specs:

```yaml
lifecycle:
  clones:
    retain_completed: 7d       # Keep for 7 days after completion
    retain_failed: 30d         # Keep failed longer for debugging
    max_disk: 10G              # Clean oldest if over limit
```

Automatic cleanup runs periodically. Use `chant cleanup` for manual cleanup when needed.

### Log Rotation

```yaml
lifecycle:
  logs:
    rotate: daily              # daily | weekly | size
    retain: 30                 # Keep 30 rotations
    compress: true
    max_size: 100M             # Per file (if rotate: size)
```

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
│  Worktree created: /tmp/chant-{spec-id}/ (parallel only)    │
│  Lock created:  .chant/.locks/{spec-id}.pid                 │
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

## Git Branch Lifecycle

```yaml
lifecycle:
  branches:
    delete_after_merge: true   # Delete chant/* branches after merge
    retain_unmerged: 30d       # Keep unmerged branches for 30d
```

Branches are cleaned up automatically when specs are completed. Use `chant cleanup` for manual cleanup when needed.

## Disk Usage

```bash
$ chant disk
Disk usage:

  Specs:    12 MB (847 files)
  Prompts:  45 KB (12 files)
  Config:   2 KB
  Clones:   2.3 GB (8 clones)
  Index:    156 MB
  Logs:     89 MB

  Total:    2.5 GB

Recommendations:
  - 3 clones older than 7 days (1.2 GB)
  - Run 'chant cleanup clones' to reclaim
```

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

  clones:
    retain_completed: 7d
    retain_failed: 30d
    max_disk: 10G

  branches:
    delete_after_merge: true
    retain_unmerged: 30d

  logs:
    rotate: daily
    retain: 30
    compress: true

  index:
    rebuild_on_startup: false  # Rebuild index on daemon start
```

---

See [Planned Features](../roadmap/planned/README.md) for upcoming lifecycle enhancements including daemon mode and advanced cleanup commands.
