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

Consistent with [chant's philosophy](../getting-started/philosophy.md): **markdown IS the UI**.

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
chant --quiet work 2026-01-22-001-x7m
chant -q work 2026-01-22-001-x7m
# No terminal output, exit code only
# Spec file still updated
```

The `--quiet` / `-q` flag is a global flag that can be used with any command to suppress non-essential output.

## Verbosity

```bash
chant work 2026-01-22-001-x7m           # Normal
chant work 2026-01-22-001-x7m -v        # Verbose (more detail)
chant work 2026-01-22-001-x7m -vv       # Debug (agent prompts visible)
```

## Progress Bars

### Chain Execution

`chant work --chain` displays a progress bar tracking completion across the chain:

```bash
$ chant work --chain

→ Starting chain execution (5 specs)...

⠋ [=====>---------------------------------] 2/5 Working on 2026-01-22-002-x7n
```

The progress bar shows:
- Current position / total specs
- Current spec being worked on
- Visual progress indicator

Progress updates as each spec completes, providing real-time visibility into chain execution.

### Parallel Execution

`chant work --parallel` shows a multi-progress display with:
- Overall completion across all specs
- Individual progress bars for each worker (optional, when verbose)

```bash
$ chant work --parallel 3

→ Starting parallel execution...
  • agent-1: 3 specs
  • agent-2: 2 specs
  • agent-3: 2 specs

⠋ [===============>----------------------] 4/7 specs completed
```

The main progress bar tracks overall completion, updating as workers finish their assigned specs.

### Display Details

Progress bars use the `indicatif` library with:
- Spinner animation (⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏)
- Bar width: 40 characters
- Progress chars: `=>`
- Colors: cyan/blue bar, green spinner

Progress bars are automatically hidden in:
- `--quiet` mode
- Non-TTY environments
- `--json` output mode

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
