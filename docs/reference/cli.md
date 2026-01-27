# CLI Reference

## Initialization

Initialize chant in a new project:

```bash
chant init                                 # Interactive wizard (guided setup)
chant init --name my-project               # Direct mode with project name
chant init --name my-project --minimal     # Only create config.md (no templates)
chant init --name my-project --silent      # Keep .chant/ local-only (gitignored)
chant init --agent claude                  # Create CLAUDE.md for AI instructions
chant init --agent cursor --agent amazonq  # Create .cursorrules and Amazon Q rules
```

### Interactive Wizard Mode

When you run `chant init` without any flags, you'll be guided through setup interactively:

```
? Project name: my-project (auto-detected)
? Include prompt templates? Yes
? Keep .chant/ local only (gitignored)? No
? Initialize agent configuration?
› None
  Claude Code (CLAUDE.md)
  Cursor (.cursorrules)
  Amazon Q (.amazonq/rules.md)
  Generic (.ai-instructions)
  All of the above
```

The wizard will:
1. Auto-detect your project name from `package.json`, `Cargo.toml`, `go.mod`, or directory name
2. Ask if you want prompt templates (standard and split prompts)
3. Ask if you want silent mode (.chant/ local-only)
4. Offer to create agent configuration files (Claude Code, Cursor, Amazon Q, etc.)

### Direct Mode

Use flags to skip the wizard and initialize directly:

- `--name PROJECT`: Override detected project name
- `--minimal`: Only create config.md (skip prompt templates)
- `--silent`: Keep .chant/ local-only, not tracked in git
- `--agent PROVIDER`: Create configuration for an AI agent provider (can be specified multiple times)
- `--force`: Overwrite existing .chant/ directory

Supported agent providers: `claude`, `cursor`, `amazonq`, `generic`, `all`

## Spec Management

```bash
chant add "Fix authentication bug"    # Create spec
chant list                            # List all specs
chant list --ready                    # List ready specs
chant list --label auth               # Filter by label
chant list --label auth --label api   # Filter by multiple labels (OR)
chant list --ready --label feature    # Combine filters
chant show 2026-01-22-001-x7m         # Show spec details
```

### Edit Spec (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
chant edit 2026-01-22-001-x7m         # Open in editor
```

### Label Filtering

Filter specs by labels defined in their frontmatter:

```yaml
# In spec frontmatter
labels: [auth, feature]
```

```bash
# Filter by single label
chant list --label auth

# Filter by multiple labels (OR - shows specs matching ANY label)
chant list --label auth --label api

# Combine with --ready
chant list --ready --label feature
```

## Execution

```bash
chant work 2026-01-22-001-x7m              # Execute single spec
chant work 2026-01-22-001-x7m --prompt tdd # Execute with specific prompt
chant work 2026-01-22-001-x7m --force      # Replay a completed spec
chant work --parallel                      # Execute all ready specs in parallel
chant work --parallel --label auth         # Execute ready specs with label
chant work 001 002 003 --parallel          # Execute specific specs in parallel
```

### Split Spec

Split a spec into member specs using AI analysis:

```bash
chant split 2026-01-22-001-x7m             # Split into group members
chant split 001 --force                    # Force split even if not pending
chant split 001 --model claude-opus-4-5    # Use specific model for analysis
```

The split command analyzes the spec content and creates numbered member specs (`.1`, `.2`, etc.) that break down the work into smaller pieces.

### Replaying Completed Specs

Use `--force` to replay a spec that has already been completed:

```bash
# Replay a completed spec to verify implementation
chant work 001 --force
```

When replaying, the agent will:
1. Detect the implementation already exists
2. Verify acceptance criteria are met
3. Append a new Agent Output section to the spec
4. Not create duplicate commits (replay is idempotent)

**When to use `--force`:**
- Verification: Re-check that acceptance criteria are still satisfied
- Prompt changes: Re-run after updating the prompt template
- Testing: Validate agent behavior on known implementations
- Skip validation: Complete a spec with unchecked acceptance criteria

### Acceptance Criteria Validation

After the agent exits, chant validates that all acceptance criteria checkboxes are checked:

```
⚠ Found 1 unchecked acceptance criterion.
Use --force to skip this validation.
error: Cannot complete spec with 1 unchecked acceptance criteria
```

If unchecked boxes exist, the spec is marked as `failed`. Use `--force` to skip this validation and complete the spec anyway.

### Parallel Execution

Execute multiple ready specs concurrently:

```bash
# Execute all ready specs in parallel
chant work --parallel

# Execute specific specs in parallel (selective)
chant work 001 002 003 --parallel

# Filter by label
chant work --parallel --label auth
chant work --parallel --label feature --label urgent

# Specify prompt for all specs
chant work --parallel --prompt tdd

# Override maximum concurrent agents
chant work --parallel --max 4

# Skip cleanup prompt after execution
chant work --parallel --no-cleanup

# Force cleanup prompt even on success
chant work --parallel --cleanup
```

**Selective Parallel Execution:**

When you specify multiple spec IDs, only those specs are executed in parallel (regardless of their ready status):

```bash
# Run exactly these 4 specs in parallel
chant work 00e 00i 00j 00k --parallel

# Combine with other options
chant work 001 002 --parallel --prompt tdd --max 2
```

This is useful when you want to control exactly which specs run together, rather than running all ready specs.

**Multi-Account Support:**

Configure multiple Claude accounts in `.chant/config.md` for distributed execution:

```yaml
parallel:
  agents:
    - name: main
      command: claude
      max_concurrent: 2
    - name: alt1
      command: claude-alt1
      max_concurrent: 3
```

Example output:

```
→ Starting 5 specs in parallel...

  • main: 2 specs
  • alt1: 3 specs

[00m-khh] Working with prompt 'standard' via main
[00n-1nl] Working with prompt 'standard' via alt1
[00o-6w7] Working with prompt 'standard' via alt1

[00m-khh] ✓ Completed (commit: abc1234)
[00n-1nl] ✓ Completed (commit: def5678)
[00o-6w7] ✓ Completed (commit: ghi9012)

════════════════════════════════════════════════════════════
Parallel execution complete:
  ✓ 5 specs completed work
  ✓ 5 branches merged to main
════════════════════════════════════════════════════════════
```

**Pitfall Detection:**

After parallel execution, chant detects and reports issues:

```
→ Issues detected:
  ✗ [spec-002] API concurrency error (retryable): Error 429
  ⚠ [spec-003] Worktree not cleaned up: /path/to/worktree

→ Run chant cleanup to analyze and resolve issues.
```

## Search (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
chant search "auth"                   # Search archive
chant search "label:feature"          # Search by label
```

## Logs

View agent output logs for a spec:

```bash
chant log 001                         # Show last 50 lines of log
chant log 001 -f                      # Follow log in real-time
chant log 001 --lines 100             # Show last 100 lines
chant log 001 -n 100 -f               # Show last 100 lines and follow
```

Logs are stored in `.chant/logs/{spec-id}.log` and are created when a spec is executed with `chant work`. The log contains the full agent output including timestamp and prompt used.

**Use cases:**
- Monitor spec execution in real-time with `-f`
- Review agent output after execution
- Debug failed specs

### Real-time Log Streaming

Logs are streamed to the log file in real-time as the agent produces output, not buffered until completion. This enables monitoring spec execution as it happens:

**Terminal 1:**
```bash
chant work 001    # Agent runs, streams to stdout AND log file
```

**Terminal 2 (simultaneously):**
```bash
chant log 001 -f  # See output in real-time as agent works
```

The log file header (spec ID, timestamp, prompt name) is written before the agent starts, so `chant log -f` will begin showing content immediately.

## Status

```bash
chant status                          # Overview
chant ready                           # Show ready specs
```

## Merge

Merge completed spec branches back to main:

```bash
chant merge 001                       # Merge single spec branch
chant merge 001 002 003               # Merge multiple specs
chant merge --all                     # Merge all completed spec branches
chant merge --all --dry-run           # Preview what would be merged
chant merge --all --delete-branch     # Delete branches after merge
chant merge --all --yes               # Skip confirmation prompt
```

### Rebase Before Merge

When multiple specs run in parallel, their branches diverge from main. Use `--rebase` to rebase each branch onto current main before the fast-forward merge:

```bash
chant merge --all --rebase            # Rebase each branch before ff-merge
chant merge --all --rebase --yes      # Skip confirmation
chant merge 001 002 --rebase          # Rebase specific specs
```

### Auto-Resolve Conflicts

Use `--auto` with `--rebase` for agent-assisted conflict resolution:

```bash
chant merge --all --rebase --auto     # Auto-resolve conflicts with agent
```

When conflicts occur during rebase, chant invokes an agent with the `merge-conflict` prompt to resolve them. The agent:
1. Reads the conflicting files
2. Analyzes the conflict markers
3. Edits files to resolve conflicts
4. Stages resolved files
5. Continues the rebase

If `--auto` is not specified and conflicts occur, the rebase is aborted and the spec is skipped.

## Resume

Retry failed specs by resetting them to pending:

```bash
chant resume 001                      # Reset failed spec to pending
chant resume 001 --work               # Reset and immediately re-execute
chant resume 001 --work --prompt tdd  # Reset and re-execute with specific prompt
chant resume 001 --work --branch      # Reset and re-execute with feature branch
```

The resume command:
1. Validates the spec is in `failed` status
2. Resets status to `pending`
3. Optionally re-executes with `--work`

## Drift

Detect when documentation and research specs have stale inputs:

```bash
chant drift                           # Check all completed specs for drift
chant drift 001                       # Check specific spec
```

Drift detection checks:
- `tracks` field: Source files being documented
- `origin` field: Research spec origins
- `informed_by` field: Reference materials

A spec has "drifted" when any tracked file was modified after the spec was completed. This indicates the documentation or research may be outdated.

**Example output:**

```
⚠ Drifted Specs (inputs changed since completion)
──────────────────────────────────────────────────
● 2026-01-24-005-abc (documentation)
  Completed: 2026-01-24
  Changed files:
    - src/api/handler.rs (modified: 2026-01-25)

✓ Up-to-date Specs (no input changes)
──────────────────────────────────────────────────
● 2026-01-24-003-xyz (research)
```

## Export

Export spec data in various formats:

```bash
chant export                          # Export all specs as JSON (default)
chant export --format csv             # Export as CSV
chant export --format markdown        # Export as Markdown table
chant export --output specs.json      # Write to file instead of stdout
```

### Filtering

```bash
chant export --status completed       # Filter by status
chant export --status pending --status ready  # Multiple statuses (OR)
chant export --type code              # Filter by spec type
chant export --label feature          # Filter by label
chant export --ready                  # Only ready specs
chant export --from 2026-01-20        # Specs from date
chant export --to 2026-01-25          # Specs until date
```

### Field Selection

```bash
chant export --fields id,status,title # Select specific fields
chant export --fields all             # Include all fields
```

Default fields: `id`, `type`, `status`, `title`, `labels`, `model`, `completed_at`

## Disk

Show disk usage of chant artifacts:

```bash
chant disk                            # Show disk usage summary
```

**Example output:**

```
Chant Disk Usage

.chant/ directory breakdown:
  Specs:               92.0 KB
  Prompts:             44.0 KB
  Logs:                1.1 MB
  Archive:             1.2 MB
  Locks:               0 B
  Store:               0 B
  .chant/ Total:       2.5 MB

Worktrees in /tmp:
  Count:               25 worktrees
  Total Size:          5.8 GB

Grand Total:
  5.8 GB
```

## DAG Visualization (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

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

## Daemon (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
chant daemon start                    # Start daemon
chant daemon start --background       # Start in background
chant daemon start --metrics-port 9090  # With Prometheus metrics
chant daemon stop                     # Stop daemon
chant daemon status                   # Check if running
```

Daemon provides: persistent index, lock table, queue, metrics.
CLI auto-connects to daemon if running, falls back to direct mode.

## Queue (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
chant queue next                      # Get next ready spec
chant queue next --project auth       # Filter by project
chant queue stats                     # Queue depth, wait times
```

Used by orchestrators and worker mode. Requires daemon.

## Lock (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
chant lock list                       # Show all locks
chant lock acquire <id>               # Acquire lock (scripting)
chant lock release <id>               # Release lock
chant lock status <id>                # Check lock status
```

Without daemon: PID files. With daemon: in-memory table.

## Agent Worker (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

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
