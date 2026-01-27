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

### Create and List

```bash
chant add                                    # Interactive wizard
chant add "Fix authentication bug"           # Create spec with description
chant list                                   # List all specs
chant show 2026-01-22-001-x7m                # Show spec details
```

### Interactive Wizard for Add

When you run `chant add` without a description, you'll be guided through spec creation interactively:

```
? Spec title: Fix authentication bug
? Spec type: code
? Brief description: Add JWT token validation to API endpoints
? Acceptance criteria (one per line, end with empty line):
  - [ ] JWT validation middleware implemented
  - [ ] All tests passing
  - [ ] Code linted
?
? Target files (optional):
  - src/auth/middleware.rs
  - src/auth/tokens.rs
?
```

### List Specs

```bash
chant list                                   # List all specs
chant list --ready                           # List ready specs (shortcut for --status ready)
chant list --label auth                      # Filter by label
chant list --label auth --label api          # Multiple labels (OR logic)
chant list --ready --label feature           # Combine filters
```

#### Type Filtering

Filter specs by type:

```bash
chant list --type code                       # Code specs only
chant list --type documentation              # Documentation specs
chant list --type task                       # Task specs
chant list --type research                   # Research specs
chant list --type driver                     # Driver/group specs

# Supported types: code, task, driver, documentation, research, group
```

#### Status Filtering

Filter specs by status:

```bash
chant list --status pending                  # Pending specs
chant list --status ready                    # Ready specs (shortcut: --ready)
chant list --status in_progress              # In-progress specs
chant list --status completed                # Completed specs
chant list --status failed                   # Failed specs
chant list --status blocked                  # Blocked specs (waiting on dependencies)
chant list --status cancelled                # Cancelled specs

# Combine filters
chant list --type code --status pending      # Pending code specs
chant list --status completed --label auth   # Completed auth specs
```

#### Label Filtering

```bash
chant list --label auth                      # Specs with 'auth' label
chant list --label auth --label api          # Specs with 'auth' OR 'api' label
chant list --label feature --label urgent    # Combine multiple labels
```

### Edit Spec (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
chant edit 2026-01-22-001-x7m         # Open in editor
```

### Cancel Spec

Soft-delete a spec by marking it cancelled. The spec file is preserved but excluded from lists and execution:

```bash
chant cancel 2026-01-22-001-x7m                # Cancel a spec (confirms)
chant cancel 2026-01-22-001-x7m --yes          # Skip confirmation
chant cancel 2026-01-22-001-x7m --dry-run      # Preview changes
chant cancel 2026-01-22-001-x7m --force        # Force cancellation (skip safety checks)
```

**Safety Checks:**
- Cannot cancel specs that are in-progress or failed (unless `--force`)
- Cannot cancel member specs (cancel the driver instead)
- Cannot cancel already-cancelled specs
- Warns if other specs depend on this spec (unless `--force`)

**What Happens:**
1. Spec status changed to `Cancelled` in frontmatter
2. File is preserved in `.chant/specs/`
3. Cancelled specs excluded from `chant list` and `chant work`
4. Can still view with `chant show` or `chant list --status cancelled`
5. All git history preserved

**Difference from Delete:**
- `cancel`: Changes status to Cancelled, preserves files and history
- `delete`: Removes spec file, logs, and worktree artifacts

## Execution

```bash
chant work                                 # Interactive wizard to select specs
chant work 2026-01-22-001-x7m              # Execute single spec
chant work 2026-01-22-001-x7m --prompt tdd # Execute with specific prompt
chant work 2026-01-22-001-x7m --force      # Replay a completed spec
chant work --parallel                      # Execute all ready specs in parallel
chant work --parallel --label auth         # Execute ready specs with label
chant work 001 002 003 --parallel          # Execute specific specs in parallel
```

### Interactive Wizard for Work

When you run `chant work` without a spec ID, an interactive wizard guides you through selection:

```
? Select specs to execute:
  [x] 2026-01-26-001-abc  Fix login bug
  [ ] 2026-01-26-002-def  Add API logging
  [ ] 2026-01-26-003-ghi  Update docs
  [Select all]
? Use parallel execution? No
? Select prompt: standard (auto-detected for code)
? Create feature branch? No
```

The wizard:
1. Shows all ready specs with multi-select
2. Asks whether to use parallel execution
3. Lets you choose a prompt (defaults to spec's prompt or type-based default)
4. Asks about branch creation (if `defaults.branch` not set)
5. Executes the selected specs

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

## Lint

Validate specs for structural issues and best practices:

```bash
chant lint                            # Validate all specs
chant lint 001                        # Validate specific spec
```

### Validation Rules

Lint checks are organized into categories:

**Hard Errors** (fail validation):
- Missing title in spec
- Unknown spec IDs in `depends_on` (broken dependencies)
- Invalid YAML frontmatter

**Type-Specific Warnings:**
- `documentation`: Missing `tracks` or `target_files` fields
- `research`: Missing both `informed_by` AND `origin` fields

**Complexity Warnings:**
- More than 5 acceptance criteria
- More than 5 target files
- More than 500 words in description
- Suggests using `chant split` if too complex

**Coupling Warnings:**
- Detecting spec ID references in body (outside code blocks)
- Suggests using `depends_on` for explicit dependencies
- Skipped for drivers/groups (allowed to reference members)

**Model Waste Warnings:**
- Using expensive models (opus/sonnet) on simple specs
- Simple spec definition: ≤3 criteria, ≤2 files, ≤200 words
- Suggests using haiku for simple work

### Output

```
✓ 2026-01-26-001-abc          (all valid)
✗ 2026-01-26-002-def: Missing title
⚠ 2026-01-26-003-ghi: Spec has 8 acceptance criteria (>5)
  Consider: chant split 2026-01-26-003-ghi
⚠ 2026-01-26-004-jkl: Spec references 001-abc without depends_on
  Suggestion: Use depends_on to explicit document dependency

Validation Summary:
  Errors: 1
  Warnings: 3
```

Exit code: 0 (all valid) or 1 (errors found)

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
chant merge                           # Interactive wizard to select specs
chant merge 001                       # Merge single spec branch
chant merge 001 002 003               # Merge multiple specs
chant merge --all                     # Merge all completed spec branches
chant merge --all --dry-run           # Preview what would be merged
chant merge --all --delete-branch     # Delete branches after merge
chant merge --all --yes               # Skip confirmation prompt
```

### Interactive Wizard

When you run `chant merge` without arguments, an interactive wizard guides you through the merge process:

```
? Select specs to merge:
  [x] 2026-01-26-001-abc  Add user authentication (chant/001-abc)
  [x] 2026-01-26-002-def  Fix login bug (chant/002-def)
  [ ] 2026-01-26-003-ghi  Update API docs (chant/003-ghi)
  [Select all]
? Use rebase strategy? No
? Delete branches after merge? Yes

→ Will merge 2 spec(s):
  · chant/001-abc → main Add user authentication
  · chant/002-def → main Fix login bug
```

The wizard:
1. Loads all completed specs that have associated branches
2. Shows a multi-select list with spec ID, title, and branch name
3. Prompts for rebase strategy (default: no)
4. Prompts for branch deletion (default: yes)
5. Executes the merge with your selections

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
chant export                          # Interactive wizard
chant export --format json            # Export all specs as JSON
chant export --format csv             # Export as CSV
chant export --format markdown        # Export as Markdown table
chant export --output specs.json      # Write to file instead of stdout
```

### Interactive Wizard for Export

When you run `chant export` without format or filters, an interactive wizard guides you:

```
? Export format:
  JSON
  CSV
  Markdown
? Filter by status (select multiple):
  [x] Ready
  [ ] Completed
  [ ] Pending
  [ ] Failed
  [ ] All statuses
? Filter by type:
  (none)
  code
  task
  documentation
  driver
? Output destination:
  Print to stdout
  Save to file
? Output filename: specs.json
```

The wizard:
1. Lets you choose export format (JSON, CSV, or Markdown)
2. Allows selecting multiple status filters
3. Lets you filter by type
4. Asks where to save (stdout or file)
5. Prompts for filename if saving to file

### Direct Mode

Use flags to skip the wizard:

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

## Config Validation

Validate configuration semantically:

```bash
chant config --validate                     # Check configuration for issues
```

### Validation Checks

The `config --validate` command performs these checks:

**Agent Commands** (errors):
- Verifies each agent command exists in PATH (using `which`)
- Example: `claude`, `claude-alt1`, etc.
- Error if command not found

**Prompt Files** (errors):
- Checks `defaults.prompt` file exists at `.chant/prompts/{name}.md`
- Checks `parallel.cleanup.prompt` file exists (if cleanup enabled)
- Error if prompt file not found

**Parallel Configuration** (informational):
- Shows number of configured agents
- Shows total capacity (sum of all `max_concurrent` values)

**Recommended Fields** (warnings):
- Warns if `defaults.model` not set (will default to haiku)

### Output

```
→ Checking configuration...

Checking parallel agents...
  ✓ main (claude) - found in PATH
  ✓ alt1 (claude-alt1) - found in PATH
  ✗ alt2 (claude-alt2) - not found in PATH

Checking prompt files...
  ✓ standard (.chant/prompts/standard.md)
  ✓ parallel-cleanup (.chant/prompts/parallel-cleanup.md)

Parallel Configuration:
  Agents: 2
  Total capacity: 5 concurrent

Recommended Fields:
  ⚠ defaults.model not set (will use haiku)

✓ Configuration valid with 1 warning
```

Exit code: 0 (valid) or 1 (errors found)

## Cleanup

Remove orphan worktrees and stale artifacts from /tmp:

```bash
chant cleanup                         # Interactive - show and prompt
chant cleanup --dry-run               # Show what would be cleaned
chant cleanup --yes                   # Remove without prompting
```

**Example output:**

```
Scanning for orphan worktrees...

Found 3 orphan worktrees:
  chant-2026-01-25-01g-v2e (234 MB, 2 days)
  chant-2026-01-25-01l-c41 (512 MB, 3 days)
  chant-2026-01-24-009-8f2 (128 MB, 5 days)

Total: 874 MB

? Clean up these worktrees? [Y/n] y

Removing chant-2026-01-25-01g-v2e... done
Removing chant-2026-01-25-01l-c41... done
Removing chant-2026-01-24-009-8f2... done
Running git worktree prune... done

Cleaned up 3 worktrees, 874 MB reclaimed
```

**Use cases:**
- Recover disk space after failed or abandoned specs
- Clean up stale worktrees from interrupted executions
- Maintain clean /tmp directory on CI systems

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
