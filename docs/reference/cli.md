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
4. Ask for your preferred model provider (Claude CLI, Ollama, OpenAI)
5. Ask for your default model (opus, sonnet, haiku, or custom)
6. Offer to create agent configuration files (Claude Code, Cursor, Amazon Q, etc.)

When Claude agent is selected, the wizard also creates `.mcp.json` for MCP server integration.

> **Tip:** The wizard is the recommended path for new users - it explains each option and prevents configuration mistakes.

### Direct Mode

Use flags to skip the wizard and initialize directly:

- `--name PROJECT`: Override detected project name
- `--minimal`: Only create config.md (skip prompt templates)
- `--silent`: Keep .chant/ local-only, not tracked in git
- `--agent PROVIDER`: Create configuration for an AI agent provider (can be specified multiple times)
- `--provider PROVIDER`: Set default model provider (`claude`, `ollama`, `openai`)
- `--model MODEL`: Set default model (`opus`, `sonnet`, `haiku`, or custom model name)
- `--force`: Overwrite existing .chant/ directory

Supported agent providers: `claude`, `cursor`, `amazonq`, `generic`, `all`

## Spec Management

### Create and List

```bash
chant add                                    # Interactive wizard
chant add "Fix authentication bug"           # Create spec with description
chant add "Risky refactor" --needs-approval  # Create spec requiring approval
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

### Needs-Approval Flag

When creating a spec with `--needs-approval`, the spec requires explicit approval before work can begin:

```bash
chant add "Add authentication" --needs-approval
```

This sets `approval.required: true` in the spec's frontmatter. The spec cannot be worked on until approved. See the [Approval Workflow Guide](../guides/approval-workflow.md) for details.

### List Specs

```bash
chant list                                   # List all specs
chant list --ready                           # List ready specs (shortcut for --status ready)
chant list --label auth                      # Filter by label
chant list --label auth --label api          # Multiple labels (OR logic)
chant list --ready --label feature           # Combine filters
```

#### Available Flags

| Flag | Description |
|------|-------------|
| `--ready` | Show only ready specs (equivalent to `--status ready`) |
| `--status STATUS` | Filter by status (pending, ready, in_progress, completed, failed, blocked, cancelled) |
| `--type TYPE` | Filter by spec type (code, task, driver, documentation, research, group) |
| `--label LABEL` | Filter by label (can be specified multiple times for OR logic) |
| `--approval STATUS` | Filter by approval status (pending, approved, rejected) |
| `--created-by NAME` | Filter by spec creator name (case-insensitive) |
| `--mentions NAME` | Filter specs mentioning a person in approval discussion |
| `--activity-since DURATION` | Filter by recent activity (e.g., "2h", "1d", "1w") |
| `--count` | Show only count of matching specs instead of listing them |
| `--global` | List specs from all configured repos in global config |
| `--repo PATH` | Filter to specific repository path (implies `--global`) |
| `--project NAME` | Filter to specific project within repository |
| `--main-only` | Skip branch resolution for in_progress specs (debug option) |

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
chant list --status blocked                  # Blocked specs (waiting on dependencies or approval)
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

#### Approval Filtering

Filter specs by approval status. See the [Approval Workflow Guide](../guides/approval-workflow.md) for details.

```bash
chant list --approval pending                # Specs awaiting approval
chant list --approval approved               # Approved specs
chant list --approval rejected               # Rejected specs
```

#### People and Activity Filtering

```bash
chant list --created-by alice                # Specs created by alice (case-insensitive)
chant list --mentions bob                    # Specs where bob is mentioned in approval discussion
chant list --activity-since 2h               # Specs modified in the last 2 hours
chant list --activity-since 1d               # Modified in the last day
chant list --activity-since 1w               # Modified in the last week
```

Duration formats: `2h` (hours), `1d` (days), `1w` (weeks), `2mo` (months).

#### Count Flag

Show only the count of matching specs instead of listing them:

```bash
chant list --count                           # Total spec count
chant list --approval pending --count        # Count of specs awaiting approval
chant list --status ready --count            # Count of ready specs
```

#### Cross-Repository Filtering

List specs across multiple repositories configured in your global config:

```bash
chant list --global                          # All specs from all configured repos
chant list --global --status ready           # Ready specs across all repos
chant list --repo /path/to/repo              # Specs from specific repository
chant list --project my-api                  # Specs from specific project
chant list --global --project frontend       # Project filter across all repos
```

When using `--global`, the output includes the repository path for each spec.

#### Flag Combinations

Combine flags for powerful filtering:

```bash
# Multi-dimensional filtering
chant list --type code --status ready --label feature
chant list --approval approved --created-by alice --activity-since 1w
chant list --global --status in_progress --project backend

# Counting filtered results
chant list --type documentation --status completed --count
chant list --global --approval pending --count

# Cross-repo queries with filters
chant list --global --status ready --type code --label urgent
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
chant work 2026-01-22-001-x7m --skip-approval  # Bypass approval check
chant work --parallel                      # Execute all ready specs in parallel
chant work --parallel --label auth         # Execute ready specs with label
chant work 001 002 003 --parallel          # Execute specific specs in parallel
```

### Approval Check

When a spec has `approval.required: true`, `chant work` checks the approval status before proceeding. Use `--skip-approval` for emergency bypasses only. See the [Approval Workflow Guide](../guides/approval-workflow.md) for details.

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

### Prep

Output cleaned spec content for agent preparation:

```bash
chant prep 2026-01-22-001-x7m              # Output spec content
chant prep 001 --clean                     # Strip agent conversation sections
```

The prep command is useful for:
- Getting the spec content ready for the agent to process
- Removing stale agent conversation sections from replayed specs (with `--clean`)
- Preparing specs for the bootstrap prompt workflow

By default, `chant prep` outputs the spec body. With the `--clean` flag, it removes any "## Agent Conversation", "## Execution Result", or similar sections that may have been added during previous runs, ensuring a clean slate for replay scenarios.

### The --force Flag

The `--force` flag for `chant work` serves two purposes:

1. **Override dependency checks** - Work on a blocked spec that has unsatisfied dependencies
2. **Skip acceptance criteria validation** - Complete a spec without all criteria checked

```bash
# Work on blocked spec (bypasses dependency checks)
chant work 001 --force

# Complete spec without all criteria checked
chant work 001 --force
```

#### Overriding Dependency Checks

When a spec is blocked due to unsatisfied dependencies, `--force` allows you to proceed anyway:

```bash
$ chant work 2026-01-22-003-abc --force
⚠ Warning: Forcing work on spec (skipping dependency checks)
  Skipping dependencies: 2026-01-22-001-x7m (in_progress)
→ Working 2026-01-22-003-abc with prompt 'standard'
```

**Use cases:**
- Dependency tracking has a bug or false positive
- You've manually verified dependencies are satisfied
- Emergency work needed despite dependency chain issues

#### Skipping Acceptance Criteria Validation

The `--force` flag also allows completing a spec even if some acceptance criteria checkboxes are not marked as complete. Use this when:
- Requirements changed after spec was created
- A criterion is no longer applicable
- You want to complete with manual verification

**Note:** For re-executing completed specs, use `chant replay` instead of `--force`.

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

### Chain Execution

Execute specs sequentially, one after another:

```bash
# Chain through all ready specs until none remain or failure
chant work --chain

# Chain through specific specs in order (ONLY these specs)
chant work --chain spec1 spec2 spec3

# Limit number of specs to chain
chant work --chain --chain-max 5

# Chain through labeled specs only
chant work --chain --label auth
```

**Two Modes of Operation:**

| Command | Behavior |
|---------|----------|
| `chant work --chain` | Chain through ALL ready specs |
| `chant work --chain spec1 spec2 spec3` | Chain through ONLY the specified specs |

**Specific ID Behavior:**

When spec IDs are provided:
- Chains through only those IDs in the order given
- Invalid spec IDs fail fast with clear error before execution starts
- Non-ready specs are skipped with warning, chain continues
- `--label` filter is ignored (IDs take precedence)
- `--chain-max` limit still applies

**Example output:**

```
→ Starting chain execution...

[1/?] Working 001-abc with prompt 'standard'
[1/?] ✓ Completed in 45s

[2/?] Working 002-def with prompt 'standard'
[2/?] ✓ Completed in 32s

[3/?] ⚠ Skipping 003-ghi: not ready (blocked by: 004-jkl)

[4/?] Working 005-mno with prompt 'standard'
[4/?] ✗ Failed: Tests did not pass

════════════════════════════════════════════════════════════
Chain execution stopped:
  ✓ 3 specs completed
  ✗ 1 spec failed: 005-mno
  ⚠ 1 spec skipped
  Total time: 2m 15s
════════════════════════════════════════════════════════════
```

**Use cases:**
- **Overnight processing**: Run all ready specs while you sleep
- **CI/CD integration**: Process specs in automated pipelines
- **Targeted batch processing**: Chain through specific specs in a defined order
- **Dependency-aware execution**: Specs become ready as their dependencies complete

## Search

Search and filter specs interactively or with direct queries:

```bash
chant search                          # Interactive wizard
chant search "auth"                   # Search by keyword
chant search "label:feature"          # Search by label
chant search "status:ready"           # Search by status
```

### Interactive Wizard

When you run `chant search` without arguments, an interactive wizard guides you through filtering:

```
? Search query (keyword, label, status, type):
auth

? Filter by status:
  [x] Pending
  [x] Ready
  [x] In Progress
  [x] Completed
  [ ] Failed
  [ ] Blocked
  [ ] Cancelled

? Filter by type:
  [x] All types
  [ ] Code
  [ ] Task
  [ ] Documentation
  [ ] Driver
  [ ] Research

Results: 15 specs matching "auth"
```

The wizard lets you:
1. Enter a search query (keyword, or `label:name`, `status:name`, `type:name`)
2. Filter results by status
3. Filter results by type
4. View and select specs from results

## Derive

Manually trigger field derivation for existing specs:

```bash
chant derive 2026-01-22-001-x7m   # Derive fields for one spec
chant derive --all                 # Derive fields for all specs
chant derive --all --dry-run       # Preview without modifying
```

### What Derivation Does

Automatically extracts and populates fields from enterprise configuration:

- Reads derivation rules from `.chant/config.md`
- Applies patterns to sources (branch, path, env, git_user)
- Validates values against enum rules (warnings only)
- Updates spec frontmatter with derived values
- Tracks which fields were derived in `derived_fields` list

### Use Cases

- **Backfill** - Add derivation rules and re-populate existing specs
- **Update values** - Re-derive if branch names or paths changed
- **Fix invalid values** - Update specs with incorrect derived values

### Examples

Add derivation rules to `.chant/config.md`:

```yaml
enterprise:
  derived:
    team:
      from: path
      pattern: "teams/(\\w+)/"
    jira_key:
      from: branch
      pattern: "([A-Z]+-\\d+)"
```

Then backfill all specs:

```bash
$ chant derive --all --dry-run
Preview: Would update 12 specs with derived fields

2026-01-22-001-x7m: team=platform, jira_key=PROJ-123
2026-01-22-002-y8n: team=frontend, jira_key=PROJ-124
...

$ chant derive --all
Derived 12 specs successfully
```

### Dry-Run Mode

Preview changes without modifying files:

```bash
$ chant derive --all --dry-run
Preview: Would update 12 specs
  • 2026-01-22-001-x7m: +team, +jira_key
  • 2026-01-22-002-y8n: +team, +jira_key
  (showing first 10 of 12)
```

Dry-run is useful for:
- Testing new derivation rules
- Validating patterns before updating
- Verifying the impact of changes

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

## Verify

Re-check acceptance criteria on completed specs to detect drift:

```bash
chant verify                          # Interactive wizard
chant verify 001                      # Verify single spec
chant verify --all                    # Verify all completed specs
chant verify --all --exit-code        # Exit with code if verification fails (CI/CD)
chant verify --all --dry-run          # Preview verification without updating
chant verify --label auth             # Verify by label
chant verify --prompt custom          # Use custom prompt
```

### Interactive Wizard

When you run `chant verify` without arguments, an interactive wizard guides you:

```
? Verify which specs:
  [x] All completed specs
  [ ] Specific spec ID(s)
  [ ] By label

? Exit with code on failure? No
? Dry run (no updates)? No

→ Verifying 12 completed specs...

✓ 2026-01-24-001-abc: Rate limiting - PASSED
✓ 2026-01-24-002-def: Auth middleware - PASSED
⚠ 2026-01-24-003-ghi: API docs - PARTIAL (1 criterion skipped)
✗ 2026-01-24-004-jkl: Logging - FAILED (2 criteria failed)

Verification Summary:
  Passed: 10
  Partial: 1
  Failed: 1
```

### What Verification Does

For each spec, the agent:
1. Reads the spec's acceptance criteria
2. Checks the current codebase against each criterion
3. Reports: ✓ PASS, ⚠ SKIP (not applicable), ✗ FAIL

### Output Stored in Frontmatter

After verification, the spec's frontmatter is updated:

```yaml
---
status: completed
last_verified: 2026-01-22T15:00:00Z
verification_status: passed          # passed | partial | failed
verification_failures:
  - "Rate limiter tests disabled"
  - "Configuration file missing"
---
```

### Exit Code for CI/CD

Use `--exit-code` to integrate with CI pipelines:

```bash
# In GitHub Actions or other CI
chant verify --all --exit-code
# Exit code: 0 (all passed), 1 (any failed)
```

### Use Cases

- **Verify Before Deploy**: Ensure acceptance criteria still hold before shipping
- **Detect Drift**: Find when reality diverges from original intent
- **Scheduled Checks**: Run nightly to catch regressions
- **Continuous Verification**: Gate deployments on spec verification

## Replay

Re-execute a completed spec's intent against the current codebase:

```bash
chant replay 001                      # Re-execute spec
chant replay 001 --dry-run            # Preview changes without executing
chant replay 001 --yes                # Skip confirmation prompts
chant replay 001 --prompt custom      # Use custom prompt
```

### What Replay Does

Replay re-executes a completed spec's intent:

```
Original (2 months ago):  Spec → Agent → Code
Replay (now):             Spec → Agent → Current Code
```

The agent:
1. Reads the original spec
2. Analyzes the current codebase
3. Re-implements the spec's intent (may differ from original implementation)
4. Creates new commits with changes

### Replay vs Retry

| | Replay | Retry |
|------|--------|-------|
| **For** | Completed specs | Failed specs |
| **Against** | Current codebase | Where it left off |
| **Purpose** | Fix drift, verify intent | Fix failure, complete work |
| **Command** | `chant replay` | `chant resume --work` |

### Use Cases

- **Fix Drift**: Re-apply intent after code changes deleted original implementation
- **Verify Intent**: Ensure spec would still succeed with current codebase
- **Update Patterns**: Re-implement using new patterns or frameworks
- **One-off Fixes**: Quickly reapply specification when needed

### Frontmatter Updates

When a spec is replayed, these fields are updated:

```yaml
---
status: completed
replayed_at: 2026-01-22T16:00:00Z
replay_count: 1
original_completed_at: 2026-01-15T14:30:00Z   # Preserved from original
---
```

## Logs

View agent output logs for a spec:

```bash
chant log 001                         # Show last 50 lines and follow (default)
chant log 001 --no-follow             # Show last 50 lines without following
chant log 001 -n 100                  # Show last 100 lines and follow
chant log 001 -n 100 --no-follow      # Show last 100 lines without following
```

Logs are stored in `.chant/logs/{spec-id}.log` and are created when a spec is executed with `chant work`. The log contains the full agent output including timestamp and prompt used.

**Use cases:**
- Monitor spec execution in real-time (follows by default)
- Review agent output after execution with `--no-follow`
- Debug failed specs

### Real-time Log Streaming

Logs are streamed to the log file in real-time as the agent produces output, not buffered until completion. This enables monitoring spec execution as it happens:

**Terminal 1:**
```bash
chant work 001    # Agent runs, streams to stdout AND log file
```

**Terminal 2 (simultaneously):**
```bash
chant log 001     # See output in real-time as agent works (follows by default)
```

The log file header (spec ID, timestamp, prompt name) is written before the agent starts, so `chant log` will begin showing content immediately.

## Status

Show project status summary with spec counts:

```bash
chant status                          # Project overview
chant status --brief                  # Compact one-line summary
chant status --json                   # JSON output format
chant status --global                 # Status across all configured repos
chant status --repo /path/to/repo     # Specific repository (implies --global)
chant status --watch                  # Watch mode - refresh every 5 seconds
chant list --ready                    # Show ready specs
```

### Flags

| Flag | Description |
|------|-------------|
| `--brief` | Compact single-line output showing key metrics |
| `--json` | Output status as JSON for programmatic parsing |
| `--global` | Show status across all configured repositories |
| `--repo <path>` | Filter to specific repository path (implies `--global`) |
| `--watch` | Watch mode - automatically refresh status every 5 seconds |

### Brief Mode

Displays a compact single-line summary:

```bash
$ chant status --brief
5 ready, 3 in-progress, 12 pending, 2 blocked, 45 completed
```

### JSON Mode

Outputs structured data for parsing:

```bash
$ chant status --json
{
  "specs": {
    "ready": 5,
    "in_progress": 3,
    "pending": 12,
    "blocked": 2,
    "completed": 45,
    "failed": 0,
    "cancelled": 1
  }
}
```

### Watch Mode

Continuously monitors project status, refreshing every 5 seconds:

```bash
$ chant status --watch
Spec Status (auto-refresh every 5s, press Ctrl+C to stop)

Ready: 5
In Progress: 3
Pending: 12
Blocked: 2
Completed: 45
Failed: 0
Cancelled: 1

[Last updated: 2026-01-28 14:30:15]
```

Press `Ctrl+C` to exit watch mode.

## Refresh

Reload all specs from disk and recalculate dependency status:

```bash
chant refresh                         # Quick summary of spec counts
chant refresh --verbose               # Show detailed ready/blocked lists
chant refresh -v                      # Short form of --verbose
```

The refresh command:
1. Reloads all specs fresh from disk (no caching)
2. Recalculates ready/blocked status based on current dependencies
3. Reports summary counts (completed, ready, in-progress, pending, blocked)
4. With `--verbose`: lists ready specs and blocked specs with their blockers

**Example output:**

```
Checking dependency status...
✓ Refreshed 142 specs
  Completed: 45
  Ready: 18
  In Progress: 3
  Pending: 52
  Blocked: 24
```

**Verbose output:**

```
Checking dependency status...
✓ Refreshed 142 specs
  Completed: 45
  Ready: 18
  In Progress: 3
  Pending: 52
  Blocked: 24

Ready specs:
  ○ 2026-01-27-00t-bfs Validate nodes.edn coverage
  ○ 2026-01-27-01a-xyz Generate AST types
  ...

Blocked specs:
  ⊗ 2026-01-27-02b-abc Implement parser (blocked by: 2026-01-27-01a-xyz (pending))
  ⊗ 2026-01-27-03c-def Add tests (blocked by: 2026-01-27-02b-abc (pending))
  ...
```

**Use cases:**
- Verify dependency status after completing specs manually
- Debug why a spec isn't showing as ready
- Get a quick overview of project spec health
- Check what's unblocked after a dependency completes

## Merge

Merge completed spec branches back to main:

```bash
chant merge                           # Interactive wizard to select specs
chant merge 001                       # Merge single spec branch
chant merge 001 002 003               # Merge multiple specs
chant merge --all                     # Merge all completed spec branches
chant merge --all-completed           # Merge completed specs with branches (post-parallel)
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

### Post-Parallel Convenience: `--all-completed`

After running `chant work --parallel`, use `--all-completed` as a convenience flag to merge all specs that:
1. Have `status: completed`
2. Have an associated branch (e.g., `chant/spec-id`)

This is perfect for post-parallel workflows where you want to merge all successfully completed work:

```bash
# After parallel execution
chant work --parallel --max 5

# Merge all completed specs that have branches
chant merge --all-completed --delete-branch --yes

# Preview what would be merged
chant merge --all-completed --dry-run
```

**`--all-completed` vs `--all`:**

| Flag | What it merges |
|------|----------------|
| `--all` | All completed specs (including those completed without branches) |
| `--all-completed` | Only completed specs that have corresponding branches |

Use `--all-completed` when you've run parallel execution and want to merge only the specs that were worked on with feature branches, ignoring specs that may have been manually completed on main.

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

Resume failed or stuck specs by resetting them to pending:

```bash
chant resume 001                      # Reset spec to pending
chant resume 001 --work               # Reset and immediately re-execute
chant resume 001 --work --prompt tdd  # Reset and re-execute with specific prompt
```

The resume command:
1. Validates the spec is in `failed` or `in_progress` status
2. Resets status to `pending`
3. Optionally re-executes with `--work`

**Use cases:**
- Retry after agent failure
- Resume specs stuck in `in_progress` (e.g., agent crashed)
- Re-attempt with different prompt or branch strategy

## Finalize

Manually finalize specs that weren't properly completed:

```bash
chant finalize 001                    # Finalize spec (records commits, timestamp, model)
```

The finalize command:
1. Validates all acceptance criteria are checked
2. Records commit SHAs from git history
3. Sets `completed_at` timestamp and `model` in frontmatter
4. Changes status to `completed`

**When to use:**
- Agent exited without calling finalize
- Spec marked failed but work was actually done
- Manual intervention needed after auto-finalize failure

**Note:** `chant work` now auto-finalizes specs when all acceptance criteria are checked. Manual finalize is only needed for recovery scenarios.

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
│  10. Check if driver complete        │
└──────────────────────────────────────┘
```

## Approve / Reject

Approve or reject specs before work begins. See the [Approval Workflow Guide](../guides/approval-workflow.md) for detailed usage.

### Approve

```bash
chant approve 001 --by alice                 # Approve spec by name
```

Updates approval status to `approved` and records approver information.

### Reject

```bash
chant reject 001 --by bob --reason "Scope too large, split first"
```

Updates approval status to `rejected`, records reason, and applies the configured rejection action (manual, dependency, or group mode).

## Activity

View recent activity across specs:

```bash
chant activity                               # Show all recent activity
chant activity --by alice                    # Filter by person
chant activity --since 2h                    # Activity in the last 2 hours
chant activity --spec 001                    # Activity for a specific spec
chant activity --by alice --since 1d         # Combine filters
```

### Activity Types

| Type | Color | Description |
|------|-------|-------------|
| `CREATED` | Cyan | Spec was created |
| `APPROVED` | Green | Spec was approved |
| `REJECTED` | Red | Spec was rejected |
| `WORKED` | Yellow | Commit with `chant(<id>):` pattern |
| `COMPLETED` | Green (bold) | Spec status changed to completed |

### Filters

- `--by <name>` - Filter by person (case-insensitive substring match)
- `--since <duration>` - Show activity from the last N time (e.g., `2h`, `1d`, `1w`, `1m`)
- `--spec <id>` - Filter by spec ID (substring match)

### Output

```
2026-01-28 14:30  alice    APPROVED    001-abc  Implement feature X
2026-01-28 12:00  bob      CREATED     002-def  Fix authentication
2026-01-27 16:45  alice    COMPLETED   003-ghi  Update docs
2026-01-27 10:15  charlie  WORKED      004-jkl  Add logging
```

Activity is deduplicated, showing the first occurrence of each (spec_id, activity_type) pair.

---

## Worktree

Inspect and manage chant worktrees used during parallel and isolated execution.

### Status

Display status information about active chant worktrees:

```bash
chant worktree status                 # Show all chant worktrees
```

**Example output:**

```
Found 3 chant worktrees:

  /tmp/chant-2026-01-27-001-abc
    Branch: chant/2026-01-27-001-abc
    HEAD:   a1b2c3d
    Spec:   2026-01-27-001-abc
    Title:  Add user authentication
    Status: in_progress
    Size:   234 MB  Age: 2h

  /tmp/chant-2026-01-27-002-def [prunable]
    Branch: chant/2026-01-27-002-def
    HEAD:   e4f5g6h
    Spec:   2026-01-27-002-def
    Title:  Fix login bug
    Status: completed
    Size:   189 MB  Age: 1d
    Reason: gitdir file points to non-existent location

  /tmp/chant-2026-01-27-003-ghi
    Branch: chant/2026-01-27-003-ghi
    HEAD:   i7j8k9l
    Spec:   2026-01-27-003-ghi
    Status: unknown (spec not found)
    Size:   156 MB  Age: 3d

⚠ 1 prunable worktree (run chant cleanup to clean up)
Total disk usage: 579 MB
```

**Output columns:**

| Field | Description |
|-------|-------------|
| Path | Absolute path to the worktree directory |
| Branch | Git branch checked out in the worktree (format: `chant/<spec-id>`) |
| HEAD | Short commit hash (7 characters) of current HEAD |
| Spec | Associated spec ID extracted from branch or path |
| Title | Spec title (if spec file exists) |
| Status | Spec status: `pending`, `ready`, `in_progress`, `completed`, `failed`, `blocked`, `cancelled` |
| Size | Disk space used by the worktree |
| Age | Time since worktree was last modified |

**Flags:**

| Flag | Meaning |
|------|---------|
| `[prunable]` (red) | Worktree can be safely removed (orphaned or corrupted) |

**Use cases:**
- Debug issues with worktree state during parallel execution
- Identify orphaned worktrees that can be cleaned up
- Monitor disk usage of active worktrees
- Check which specs have active worktrees

### Environment Variables for Agents

When agents run in worktree mode (during parallel execution), chant sets environment variables to help agents understand their execution context:

| Variable | Description | Example |
|----------|-------------|---------|
| `CHANT_WORKTREE` | Set to `1` when running in a worktree | `1` |
| `CHANT_WORKTREE_PATH` | Absolute path to the worktree directory | `/tmp/chant-2026-01-27-001-abc` |
| `CHANT_BRANCH` | Git branch name for this spec | `chant/2026-01-27-001-abc` |

These variables are only set when the agent is invoked with a working directory parameter, which indicates worktree mode. Agents can use these variables to:

- Detect they're running in an isolated environment
- Understand their changes won't affect main until merged
- Log or reference the worktree path for debugging
- Check the branch name for commit messages

**Example agent check:**

```bash
if [ -n "$CHANT_WORKTREE" ]; then
    echo "Running in worktree: $CHANT_WORKTREE_PATH"
    echo "Branch: $CHANT_BRANCH"
fi
```

---


## Watch

Automatically finalize specs when their acceptance criteria are met:

```bash
chant watch                           # Watch and finalize specs
chant watch --once                    # Run one iteration then exit
chant watch --dry-run                 # Preview actions without executing
chant watch --poll-interval 10        # Set poll interval to 10ms (overrides config)
```

### How It Works

The watch command:
1. Polls for completed specs at regular intervals
2. Checks if all acceptance criteria are marked as complete
3. Automatically finalizes specs when ready
4. Continues watching (unless `--once` is specified)

### Flags

| Flag | Description |
|------|-------------|
| `--once` | Run only one iteration then exit (useful for testing) |
| `--dry-run` | Show what would be finalized without actually doing it |
| `--poll-interval MS` | Set poll interval in milliseconds (overrides config setting) |

### Examples

**Basic usage:**

```bash
chant watch
```

Watches continuously and finalizes specs when ready.

**One-shot mode for testing:**

```bash
chant watch --once
```

Checks once and exits (useful for CI/CD integration or testing).

**Preview mode:**

```bash
chant watch --dry-run --once
```

Shows what would be finalized without making changes.

**Custom poll interval:**

```bash
chant watch --poll-interval 1000
```

Poll every 1000ms (1 second) instead of using config default.

### Use Cases

- **Continuous integration**: Monitor spec completion in CI pipelines
- **Background automation**: Run in tmux/screen for automatic finalization
- **Testing**: Use `--once` to verify watch behavior without blocking

---

## Site Generation

Generate static documentation sites from specs:

```bash
chant site init                       # Initialize theme directory
chant site init --force-overwrite     # Overwrite existing theme files
chant site build                      # Build static site
chant site build --output ./dist      # Build to custom directory
chant site serve                      # Serve site locally (default port: 3000)
chant site serve --port 8080          # Serve on custom port
chant site serve --output ./dist      # Serve from custom directory
```

### Site Init

Initialize the theme directory with customizable templates:

```bash
chant site init                       # Create theme files at .chant/site/theme
chant site init --force-overwrite     # Overwrite existing theme files
```

The `init` command:
1. Creates `.chant/site/theme/` directory
2. Copies default templates for customization:
   - `index.html` - Homepage template
   - `spec.html` - Individual spec page template
   - `styles.css` - Site stylesheet
   - Other theme assets
3. Lists created files with descriptions
4. Provides next steps for customization

**Flags:**
- `--force-overwrite` - Overwrite existing theme files without prompting

**Example output:**

```
✓ Theme initialized at .chant/site/theme

Created files:
  index.html - Homepage template
  spec.html - Individual spec page template
  styles.css - Site stylesheet

Next steps:
  1. Edit templates in .chant/site/theme
  2. Run chant site build to generate the site
  3. Run chant site serve to preview locally
```

### Site Build

Build a static site from all specs:

```bash
chant site build                      # Build to configured output directory
chant site build --output ./dist      # Build to custom directory
chant site build -o ./docs            # Short form
```

The build command:
1. Loads all specs from `.chant/specs/`
2. Uses custom theme from `.chant/site/theme/` if available
3. Falls back to embedded default theme
4. Generates static HTML files
5. Writes to output directory (default: `.chant/site/dist`)

**Flags:**
- `--output DIR`, `-o DIR` - Output directory (overrides config)

**Example output:**

```
→ Building site to .chant/site/dist
  Found 142 specs
  Using custom theme from .chant/site/theme

✓ Site built successfully
  142 specs included
  285 files written
  Output: .chant/site/dist

Next steps:
  Preview locally: chant site serve --port 3000
  Deploy: Copy .chant/site/dist to your web server
```

### Site Serve

Start a local HTTP server to preview the site:

```bash
chant site serve                      # Serve on port 3000 (default)
chant site serve --port 8080          # Serve on custom port
chant site serve -p 8080              # Short form
chant site serve --output ./dist      # Serve from custom directory
chant site serve -o ./dist            # Short form
```

The serve command:
1. Checks that the site has been built
2. Starts an HTTP server on the specified port
3. Serves static files from the output directory
4. Logs requests to stdout
5. Runs until Ctrl+C

**Flags:**
- `--port PORT`, `-p PORT` - Port to serve on (default: 3000)
- `--output DIR`, `-o DIR` - Output directory to serve (default: from config)

**Example output:**

```
→ Serving .chant/site/dist at http://127.0.0.1:3000
Press Ctrl+C to stop
  200 / .chant/site/dist/index.html
  200 /styles.css .chant/site/dist/styles.css
  200 /specs/2026-01-27-001-abc.html .chant/site/dist/specs/2026-01-27-001-abc.html
```

### Configuration

Configure site generation in `.chant/config.md`:

```yaml
site:
  output_dir: .chant/site/dist        # Output directory for build
  title: "Project Specs"              # Site title
  description: "Documentation site"   # Site description
```

**Use cases:**
- **Documentation**: Generate a browsable spec catalog for your team
- **Publishing**: Create a public documentation site from specs
- **Archiving**: Build a static snapshot of all specs
- **Review**: Share specs with stakeholders via a web interface


---

## Planned Commands

The following commands are planned but not yet implemented:

- `chant edit` - Interactive spec editing
- `chant dag` - Dependency graph visualization
- `chant daemon` - Background process for scale deployments
- `chant queue` - Queue management for worker pools
- `chant lock` - Manual lock operations
- `chant agent worker` - Worker mode for continuous execution

See [Planned Features](../roadmap/planned/README.md) for details on these upcoming capabilities.
