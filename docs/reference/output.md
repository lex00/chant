# Output & Progress

## Live Markdown Updates

During execution, the spec file updates in real-time:

```yaml
# 2026-01-22-001-x7m.md
---
status: in_progress
started_at: 2026-01-22T15:30:00Z
progress: |
  [15:30:01] Reading src/auth/middleware.go
  [15:30:03] Found 3 relevant files
  [15:30:05] Planning approach...
  [15:30:15] Implementing JWT validation
  [15:30:45] Running tests...
---

# Add authentication

...
```

## Why Markdown?

Consistent with philosophy: **markdown IS the UI**.

- Watch with `tail -f .chant/specs/2026-01-22-001-x7m.md`
- View in any editor with auto-reload
- Git diff shows exactly what happened
- No separate log files

## Progress Field

The `progress` field is a multi-line string:

```yaml
progress: |
  [HH:MM:SS] Message
  [HH:MM:SS] Message
  ...
```

Appended to as work proceeds. Cleared on completion.

## Terminal Output

`chant work` also streams to terminal:

```bash
$ chant work 2026-01-22-001-x7m
[15:30:01] Reading src/auth/middleware.go
[15:30:03] Found 3 relevant files
[15:30:05] Planning approach...
[15:30:15] Implementing JWT validation
[15:30:45] Running tests...
[15:31:02] ✓ Complete

Commit: a1b2c3d4
```

Both terminal and file get same updates.

## Watch Command

For background execution:

```bash
chant work 2026-01-22-001-x7m --background
chant watch 2026-01-22-001-x7m   # Stream progress
```

Or watch any spec:

```bash
chant watch 2026-01-22-001-x7m
# Streams progress field updates until completion
```

## Completion

On success, progress is moved to a `log` field:

```yaml
---
status: completed
completed_at: 2026-01-22T15:31:02Z
commit: a1b2c3d4
log: |
  [15:30:01] Reading src/auth/middleware.go
  [15:30:03] Found 3 relevant files
  [15:30:05] Planning approach...
  [15:30:15] Implementing JWT validation
  [15:30:45] Running tests...
  [15:31:02] Complete
---
```

`progress` → `log` rename signals completion.

## Failure

On failure:

```yaml
---
status: failed
failed_at: 2026-01-22T15:31:02Z
error: "Test suite failed: 2 assertions"
log: |
  [15:30:01] Reading src/auth/middleware.go
  [15:30:15] Implementing JWT validation
  [15:30:45] Running tests...
  [15:31:02] ERROR: Test suite failed
---
```

`error` field captures the failure reason.

## Quiet Mode

For scripting:

```bash
chant work 2026-01-22-001-x7m --quiet
# No terminal output, exit code only
# Spec file still updated
```

## Verbosity

```bash
chant work 2026-01-22-001-x7m           # Normal
chant work 2026-01-22-001-x7m -v        # Verbose (more detail)
chant work 2026-01-22-001-x7m -vv       # Debug (agent prompts visible)
```

## Structured Output

For tooling:

```bash
chant work 2026-01-22-001-x7m --json
```

```json
{"time": "15:30:01", "event": "read", "file": "src/auth/middleware.go"}
{"time": "15:30:15", "event": "change", "file": "src/auth/middleware.go"}
{"time": "15:31:02", "event": "complete", "commit": "a1b2c3d4"}
```
