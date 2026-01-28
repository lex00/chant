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

```bash
# Manual cleanup
chant cleanup clones --older-than 7d
chant cleanup clones --all
```

### Log Rotation

```yaml
lifecycle:
  logs:
    rotate: daily              # daily | weekly | size
    retain: 30                 # Keep 30 rotations
    compress: true
    max_size: 100M             # Per file (if rotate: size)
```

### Index Rebuild (Planned)

> **Status: Planned** - Index management commands are on the roadmap but not yet implemented.

Index is derived, can always be rebuilt:

```bash
chant index rebuild            # Rebuild from spec files
chant index clear              # Delete index (rebuilds on next query)
```

## Cleanup Commands (Planned)

> **Status: Planned** - Cleanup commands are on the roadmap but not yet implemented.

```bash
# Show what would be cleaned
chant cleanup --dry-run

# Clean old clones
chant cleanup clones

# Clean old logs
chant cleanup logs

# Archive old specs (if configured)
chant cleanup archive

# Full cleanup
chant cleanup --all

# Scheduled cleanup (cron)
0 0 * * * cd /repo && chant cleanup --all --quiet
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
│  Clone created: .chant/.clones/001/                         │
│  Lock created:  .chant/.locks/001.pid                       │
│  Index updated: .chant/.store/tantivy/                      │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ completion
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Cleanup                                  │
│                                                              │
│  Lock released (immediate)                                   │
│  Clone retained (7d default)                                 │
│  Spec updated (status: completed)                            │
│  Branch pushed (if pr: true)                                 │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ after retention period
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                     Long-term                                │
│                                                              │
│  Spec: stays in git history forever                          │
│  Clone: deleted after retention                              │
│  Logs: rotated, old logs deleted                             │
│  Branch: deleted after merge (optional)                      │
└─────────────────────────────────────────────────────────────┘
```

## Git Branch Lifecycle

```yaml
lifecycle:
  branches:
    delete_after_merge: true   # Delete chant/* branches after merge
    retain_unmerged: 30d       # Keep unmerged branches for 30d
```

```bash
# Clean merged branches
chant cleanup branches --merged

# List stale branches
chant cleanup branches --dry-run
```

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

## Export (Planned)

> **Status: Planned** - Export commands are on the roadmap but not yet implemented.

### Export Specs

```bash
# Export to JSON
chant export --format json > specs.json

# Export to CSV
chant export --format csv > specs.csv

# Export specific project
chant export --project auth --format json

# Export with filters
chant export --status completed --after 2026-01-01
```

### Export Format

```json
{
  "exported_at": "2026-01-22T15:00:00Z",
  "specs": [
    {
      "id": "2026-01-22-001-x7m",
      "status": "completed",
      "title": "Add authentication",
      "created_at": "2026-01-22T10:00:00Z",
      "completed_at": "2026-01-22T12:00:00Z",
      "commit": "abc123",
      "branch": "chant/2026-01-22-001-x7m",
      "body": "..."
    }
  ]
}
```

## Backup

### Git IS the Backup

Spec files are git-tracked. Normal git backup applies:
- Push to remote
- Mirror to backup location
- Standard git disaster recovery

### Local State Backup (Planned)

> **Status: Planned** - Backup/restore commands are on the roadmap but not yet implemented.

For local state (not in git):

```bash
# Backup local state
chant backup --output backup.tar.gz

# Includes:
#   - .chant/.store/ (index)
#   - .chant/logs/
#   - .chant/.clones/ (optionally)

# Restore
chant restore backup.tar.gz
```

**Note**: Local state can be rebuilt. Backup is convenience, not necessity.

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
