# Specs

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
sources:                       # research: materials to synthesize
data:                          # research: input data triggering drift

# Git (populated on completion)
branch: chant/2026-01-22-002-q2n
commit: a1b2c3d4
pr: https://github.com/...
completed_at: 2026-01-22T15:30:00Z
model: claude-opus-4-5            # AI model that executed the spec

# Execution
prompt: standard               # Optional, defaults to config
decisions: document            # autonomous | document | pause | fail
---
```

See [spec-types.md](spec-types.md) for field usage by type.

## Spec States

```
waiting → pending → in_progress → completed
                  ↘             ↘ failed
                   cancelled
```

- **waiting**: Has triggers that are not yet satisfied (see [triggers.md](triggers.md)) *(Planned)*
- **pending**: Ready to execute (no triggers, or all triggers satisfied)
- **in_progress**: Agent currently executing
- **completed**: Work done, committed
- **failed**: Execution failed, needs attention
- **cancelled**: Work was cancelled before completion *(Planned)*

> **Note**: The `waiting` and `cancelled` states are planned but not yet implemented. Currently only `pending`, `in_progress`, `completed`, and `failed` are supported.

## Drift Detection

### The `origin:` Field

Documentation and research specs declare their source files:

```yaml
---
type: documentation
origin:
  - src/auth/*.go
  - src/api/handler.go
target_files:
  - docs/auth.md
---
```

When origin files change after spec completion → drift detected.

### Drift by Type

| Type | Drifts When |
|------|-------------|
| `code` | Acceptance criteria fail |
| `documentation` | Origin source code changes |
| `research` | Origin data files change |

### Checking for Drift (Planned)

> **Status: Planned** - The `chant verify` command is on the roadmap but not yet implemented.

```bash
$ chant verify --docs
Checking documentation specs...

doc-001: API Reference
  Origin: src/api/handler.go
  Last verified: 2026-01-22
  Code changed: 2026-01-25  ← DRIFT

  Recommendation: Re-run doc spec
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

## Spec Cancellation (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

### Cancelling a Pending Spec

```bash
$ chant cancel 001
Spec 001 cancelled.
```

Sets `status: cancelled`. Spec won't be picked up by `chant work`.

### Cancelling an In-Progress Spec

```bash
$ chant cancel 001
Spec 001 is in_progress (PID 12345)

Options:
  [G] Graceful - signal agent to stop, wait for cleanup
  [F] Force - kill immediately, may leave uncommitted work
  [A] Abort - don't cancel

Choice: G

Sending SIGTERM to agent...
Agent stopped.
Cleaning up clone...
Spec 001 cancelled.
```

**Graceful cancellation:**
1. Send SIGTERM to agent
2. Agent receives signal, commits partial work (if configured)
3. Agent exits cleanly
4. CLI releases lock, updates status

**Force cancellation:**
1. Send SIGKILL to agent
2. Clone may have uncommitted changes
3. Lock released, status set to `cancelled`
4. Warning about potential uncommitted work

### Cancelled State

```yaml
---
status: cancelled
cancelled_at: 2026-01-22T15:00:00Z
cancelled_by: alex
reason: "Requirements changed"  # Optional
partial_commit: def456          # If agent committed partial work
---
```

### Resuming Cancelled Specs (Planned)

> **Status: Planned** - This feature is on the roadmap but not yet implemented.

```bash
$ chant resume 001
Spec 001 was cancelled.
Resume as new attempt? [y/N]
```

Or re-open:

```bash
$ chant reopen 001
Spec 001 reopened (status: pending)
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

### Splitting Specs (Planned)

> **Status: Planned** - The `chant split` command is on the roadmap but not yet implemented.

If a spec grows too large:

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
