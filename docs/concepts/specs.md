# Specs

## Spec Types at a Glance

| Type | Use For | Example |
|------|---------|---------|
| `code` | Features, bugs, refactoring | Implement JWT auth |
| `task` | Manual work, prompts, config | Create documentation prompt |
| `driver` | Coordinate multiple specs | Auth system (with .1, .2, .3 members) |
| `group` | Alias for driver | Same as driver |
| `documentation` | Generate docs from code | Document auth module |
| `research` | Analysis, synthesis | Analyze survey data |

```yaml
# Code spec - implement something
---
type: code
target_files: [src/auth.rs]
---

# Task spec - manual/config work
---
type: task
target_files: [.chant/prompts/doc.md]
---

# Driver spec - coordinates members
---
type: driver
---
# (has 001.1.md, 001.2.md members)

# Documentation spec - docs from code
---
type: documentation
tracks: [src/auth/*.rs]
target_files: [docs/auth.md]
---

# Research spec - analysis/synthesis
---
type: research
origin: [data/metrics.csv]
target_files: [analysis/report.md]
---
```

See [spec-types.md](spec-types.md) for detailed documentation of each type.

## Unified Model

No separate "epic" type. Specs can be split into groups. A spec with group members is a driver.

```
.chant/specs/
  2026-01-22-001-x7m.md       ← Driver (has members)
  2026-01-22-001-x7m.1.md     ← Member
  2026-01-22-001-x7m.2.md     ← Member
  2026-01-22-001-x7m.2.1.md   ← Nested group member
```

## Filename is the ID

No `id` field in frontmatter. The filename (without `.md`) is the identifier.

See [ids.md](ids.md) for format details.

## Frontmatter Schema

```yaml
---
# No id field - filename is the ID

# Type (determines behavior)
type: code                     # code | documentation | research

# State
status: pending                # pending | in_progress | completed | failed
depends_on:                    # Spec IDs that must complete first
  - 2026-01-22-001-x7m

# Organization
labels: [auth, feature]        # Free-form tags
target_files:                  # Files this spec creates/modifies
  - src/auth/middleware.go

# Context (reference docs for any type)
context:                       # Docs injected as background for agent
  - docs/api-design.md

# Type-specific fields (see spec-types.md)
tracks:                        # documentation: source code to monitor
informed_by:                   # research: materials to synthesize
origin:                        # research: input data (triggers drift)
schedule: weekly               # research: recurring execution

# Git (populated on completion)
branch: chant/2026-01-22-002-q2n
commit: a1b2c3d4
pr: https://github.com/...
completed_at: 2026-01-22T15:30:00Z
model: claude-opus-4-5            # AI model that executed the spec

# Execution
prompt: standard               # Optional, defaults to config
decisions: document            # autonomous | document | pause | fail

# Verification
last_verified: 2026-01-22T15:00:00Z  # Timestamp of last verification
verification_status: passed   # passed | partial | failed (after verify)
verification_failures:        # List of failed acceptance criteria
  - "Criterion description"

# Replay
replayed_at: 2026-01-22T16:00:00Z    # Timestamp of last replay
replay_count: 1               # Number of times replayed
original_completed_at: 2026-01-15T14:30:00Z  # Preserved from first completion
---
```

See [spec-types.md](spec-types.md) for field usage by type.

## Spec States

```
waiting → pending → in_progress → completed
                  ↘             ↘ failed
                   blocked
                   cancelled
```

- **waiting**: Has triggers that are not yet satisfied (see [triggers.md](triggers.md)) *(Planned)*
- **pending**: Ready to execute (no triggers, or all triggers satisfied)
- **in_progress**: Agent currently executing
- **completed**: Work done, committed
- **failed**: Execution failed, needs attention
- **blocked**: Spec has unmet dependencies or is waiting for something
- **cancelled**: Work was cancelled before completion

> **Note**: The `waiting` state is planned but not yet implemented. Currently supported: `pending`, `in_progress`, `completed`, `failed`, `blocked`, and `cancelled`.

## Drift Detection

Documentation and research specs declare their input files. When these change after completion, drift is detected.

```yaml
# Documentation: tracks source code
---
type: documentation
tracks:
  - src/auth/*.rs
target_files:
  - docs/auth.md
---

# Research: origin data + informed_by materials
---
type: research
origin:
  - data/metrics.csv
informed_by:
  - docs/methodology.md
target_files:
  - analysis/report.md
---
```

### Drift by Type

| Type | Field | Drifts When |
|------|-------|-------------|
| `code` | — | Acceptance criteria fail |
| `documentation` | `tracks:` | Tracked source code changes |
| `research` | `origin:`, `informed_by:` | Input files change |

### Checking for Drift

Use `chant verify` to re-check acceptance criteria and detect drift:

```bash
$ chant verify 001
Verifying spec 001: Add rate limiting

Checking acceptance criteria...
  ✓ Rate limiter middleware exists
  ✓ Returns 429 with Retry-After header
  ✓ Tests verify rate limiting works

Spec 001: VERIFIED
```

For documentation and research specs, use `chant drift` to detect input file changes:

```bash
$ chant drift
⚠ Drifted Specs (inputs changed)
  2026-01-24-005-abc (documentation)
    src/api/handler.rs (modified: 2026-01-25)

$ chant replay 005  # Re-run to update documentation
```

See [autonomy.md](autonomy.md) for more on drift and replay.

## Readiness

A spec is **ready** when:
1. Status is `pending`
2. All `depends_on` specs are `completed`
3. No group members exist OR all members are `completed`

## Spec Groups

Determined by filename, not frontmatter:

```
2026-01-22-001-x7m.md      ← Driver
2026-01-22-001-x7m.1.md    ← Member (driver = 2026-01-22-001-x7m)
```

No `driver` field needed. The `.N` suffix establishes group membership.

A driver with incomplete members cannot be marked complete. See [groups.md](groups.md).

## Spec Cancellation

Soft-delete a spec by marking it cancelled. The spec file is preserved but excluded from execution.

### Cancelling a Spec

```bash
$ chant cancel 001                # Cancel with confirmation
$ chant cancel 001 --yes          # Skip confirmation
$ chant cancel 001 --dry-run      # Preview what would be cancelled
$ chant cancel 001 --force        # Force cancellation (skip safety checks)
```

**Safety Checks:**
- Cannot cancel specs that are in-progress or failed (unless `--force`)
- Cannot cancel member specs (cancel the driver instead)
- Cannot cancel already-cancelled specs
- Warns if other specs depend on this spec (unless `--force`)

### What Happens When Cancelled

1. Spec status changed to `Cancelled` in frontmatter
2. File is preserved in `.chant/specs/`
3. Cancelled specs excluded from `chant list` and `chant work`
4. Can still view with `chant show` or `chant list --status cancelled`
5. All git history preserved

### Cancelled State

```yaml
---
status: cancelled
---
```

### Difference from Delete

- `cancel`: Changes status to Cancelled, preserves files and history
- `delete`: Removes spec file, logs, and worktree artifacts

### Re-opening Cancelled Specs

To resume work on a cancelled spec, manually edit the status back to pending:

```bash
# Edit the spec file and change status: cancelled to status: pending
chant work 001  # Resume execution
```

## Spec Amendments

Specs are **append-only by default**. Prefer:
- Cancel and create new spec
- Add member specs for new requirements
- Create follow-up spec

### Editing Before Work (Planned)

> **Status: Planned** - The `chant edit` command is on the roadmap but not yet implemented. For now, edit spec files directly in your text editor.

Freely edit pending specs:

```bash
$ chant edit 001
# Opens spec file in $EDITOR
```

Or edit directly - it's just a markdown file.

### Editing During/After Work (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

If spec is `in_progress` or `completed`:

```bash
$ chant edit 001
Warning: Spec has work in progress.
Editing may cause inconsistency.

Options:
  [D] Edit description only (safe)
  [F] Full edit (may break history)
  [A] Abort

Choice: D
# Opens editor with description section only
```

**Safe edits** (always allowed):
- Description clarification
- Labels
- Notes

**Risky edits** (warning):
- Target files (may not match actual work)
- Dependencies (may invalidate completion)

### Amendment Log

Track changes to spec after creation:

```yaml
---
status: completed
amendments:
  - at: 2026-01-22T14:00:00Z
    by: alex
    field: description
    reason: "Clarified scope"
---
```

### Splitting Specs

If a spec grows too large, use `chant split` to break it into member specs:

```bash
$ chant split 001
Creating member specs from 001...

Found sections:
  1. "Implement auth middleware"
  2. "Add JWT validation"
  3. "Write tests"

Create as members? [y/N]

Created:
  001.1 - Implement auth middleware
  001.2 - Add JWT validation
  001.3 - Write tests

Driver 001 status: pending (waiting for members)
```

## Acceptance Criteria Validation

Chant validates that all acceptance criteria checkboxes are checked before marking a spec complete. This validation happens after the agent exits.

```markdown
## Acceptance Criteria

- [x] Implement login endpoint
- [ ] Add rate limiting        <- Unchecked!
- [x] Write tests
```

If unchecked boxes exist, chant shows a warning and fails:

```
⚠ Found 1 unchecked acceptance criterion.
Use --force to skip this validation.
error: Cannot complete spec with 1 unchecked acceptance criteria
```

The spec is marked as `failed` until all criteria are checked.

### Skipping Validation

Use `--force` to complete despite unchecked boxes:

```bash
chant work 001 --force
```

### Best Practice

Agents should check off criteria as they complete each item:
- Change `- [ ]` to `- [x]` in the spec file
- This creates a clear record of completion

## Agent Output

After successful completion, chant appends the agent's output to the spec file. This creates an audit trail of agent work.

### Format

The output is appended as a new section with timestamp:

```markdown
## Agent Output

2026-01-24T15:30:00Z

\`\`\`
Done! I've implemented the authentication middleware.

Summary:
- Added JWT validation in src/auth/middleware.go
- Added tests in src/auth/middleware_test.go
- All 5 tests pass
\`\`\`
```

### Multiple Runs

Each replay with `--force` appends a new output section:

```markdown
## Agent Output

2026-01-24T15:30:00Z

\`\`\`
[first run output]
\`\`\`

## Agent Output

2026-01-24T16:00:00Z

\`\`\`
[replay output - agent detected implementation exists]
\`\`\`
```

This allows tracking how the agent behaved across multiple executions.

### Truncation

Outputs longer than 5000 characters are truncated with a note indicating the truncation. This prevents spec files from growing excessively large while still capturing the essential information about what the agent accomplished.

## Model Tagging

When a spec completes, chant records the AI model used in the frontmatter:

```yaml
---
status: completed
commit: abc1234
model: claude-opus-4-5
---
```

### How Model is Detected

The model is detected from environment variables at execution time. Chant checks these variables in order:

1. `CHANT_MODEL` - chant-specific override
2. `ANTHROPIC_MODEL` - standard Anthropic environment variable

The first non-empty value found is recorded. If neither is set, the `model` field is omitted from the frontmatter.

### Possible Values

The `model` field contains whatever value is in the environment variable. Common values include:

- `claude-opus-4-5` - Claude Opus 4.5
- `claude-sonnet-4` - Claude Sonnet 4
- `claude-haiku-3-5` - Claude Haiku 3.5

The value is recorded as-is without validation, so it may also contain version suffixes or custom identifiers depending on your setup.

### Use Cases

- **Cost tracking**: See which models completed which specs to understand costs
- **Debugging**: Identify model-specific behavior differences when issues arise
- **Auditing**: Know which AI version produced each change for compliance or review
