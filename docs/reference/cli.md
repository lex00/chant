# CLI Reference

## Spec Management

```bash
chant add "Fix authentication bug"    # Create spec
chant list                            # List all specs
chant list --ready                    # List ready specs
chant show 2026-01-22-001-x7m         # Show spec details
chant edit 2026-01-22-001-x7m         # Open in editor
```

## Execution

```bash
chant work 2026-01-22-001-x7m         # Execute single spec
chant work 2026-01-22-001-x7m --prompt tdd  # Execute with specific prompt
chant work 2026-01-22-001-x7m --parallel  # Execute all ready members
chant split 2026-01-22-001-x7m           # Split into group members
```

## Search

```bash
chant search "auth"                   # Search archive
chant search "label:feature"          # Search by label
```

## Status

```bash
chant status                          # Overview
chant ready                           # Show ready specs
```

## DAG Visualization

```bash
chant dag                             # ASCII dependency graph
chant dag --format dot                # Export as Graphviz DOT
chant dag --format mermaid            # Export as Mermaid diagram
chant dag --format json               # Export as JSON

chant dag --spec 001                  # Show DAG rooted at spec
chant dag --spec 001 --depth 2        # Limit depth
chant dag --label auth                # Filter by label
```

**ASCII output:**

```
$ chant dag
001 ─┬─▶ 002 ───▶ 004
     └─▶ 003 ─┬─▶ 005
              └─▶ 006

Legend: ○ pending  ◐ in_progress  ● completed  ✗ failed  ◇ waiting
```

**DOT export (for Graphviz):**

```bash
$ chant dag --format dot > deps.dot
$ dot -Tpng deps.dot -o deps.png
```

**Mermaid export (for docs):**

```bash
$ chant dag --format mermaid
graph LR
    001[Add auth] --> 002[Add login]
    001 --> 003[Add register]
    002 --> 004[Add middleware]
    003 --> 005[Add tests]
    003 --> 006[Add docs]
```

## Daemon (Scale)

```bash
chant daemon start                    # Start daemon
chant daemon start --background       # Start in background
chant daemon start --metrics-port 9090  # With Prometheus metrics
chant daemon stop                     # Stop daemon
chant daemon status                   # Check if running
```

Daemon provides: persistent index, lock table, queue, metrics.
CLI auto-connects to daemon if running, falls back to direct mode.

## Queue (Daemon Required)

```bash
chant queue next                      # Get next ready spec
chant queue next --project auth       # Filter by project
chant queue stats                     # Queue depth, wait times
```

Used by orchestrators and worker mode.

## Lock (Optional)

```bash
chant lock list                       # Show all locks
chant lock acquire <id>               # Acquire lock (scripting)
chant lock release <id>               # Release lock
chant lock status <id>                # Check lock status
```

Without daemon: PID files. With daemon: in-memory table.

## Agent Worker (Scale)

```bash
chant agent worker                    # Start worker mode
chant agent worker --project auth     # Only work on auth specs
chant agent worker --once             # Single spec then exit
```

Worker mode: poll queue → acquire lock → execute → release → repeat.

## Execution Flow

```
chant work 2026-01-22-001-x7m
       │
       ▼
┌──────────────────────────────────────┐
│  1. Load spec from 2026-01-22-001-x7m.md │
│  2. Check if ready (deps satisfied)  │
│  3. Resolve prompt (spec → config)   │
│  4. Load prompt from prompts/{name}.md │
└──────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│  5. Create branch (if enabled)       │
│  6. Build message (prompt + spec)    │
│  7. Spawn agent with prompt + spec   │
│  8. Stream output                    │
└──────────────────────────────────────┘
       │
       ▼
   ┌───┴───┐
   │       │
success  failure
   │       │
   ▼       ▼
┌────────┐ ┌────────┐
│complete│ │ failed │
│spec    │ │ spec   │
└────────┘ └────────┘
       │
       ▼
┌──────────────────────────────────────┐
│  9. Update frontmatter (commit hash) │
│  10. Create PR (if enabled)          │
│  11. Check if driver complete        │
└──────────────────────────────────────┘
```
