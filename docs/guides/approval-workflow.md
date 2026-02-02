# Approval Workflow

## Overview

The approval workflow adds a gate before spec execution, ensuring that someone reviews and approves the spec before an agent starts working on it. This is useful for:

- Team workflows where specs need sign-off before implementation
- Solo developers who want a self-review checkpoint for risky changes
- Enforcing quality gates before agent execution

## When to Require Approval

Use `--needs-approval` when:

- **Risky changes**: Refactoring core systems, security-sensitive code, database migrations
- **Team coordination**: Multiple people working on the same codebase
- **Self-review**: Force yourself to think twice before executing a spec
- **Compliance**: Audit trail requirements for who approved what work

Skip approval for:

- Quick bug fixes with obvious solutions
- Documentation updates
- Automated or CI-driven spec execution

## Basic Workflow

### 1. Create a Spec Requiring Approval

```bash
chant add "Refactor authentication system" --needs-approval
```

The spec is created with `approval.required: true` and `approval.status: pending`.

### 2. Review and Approve

```bash
# Review the spec
chant show 001-abc

# Approve it
chant approve 001-abc --by alice
```

The spec's approval status changes to `approved`, and an entry is added to the "## Approval Discussion" section.

**What happens during approval:**

1. Validates the spec has `approval.required: true`
2. Validates the approver name against git committers (warns if not found)
3. Updates the spec's approval frontmatter:
   ```yaml
   approval:
     required: true
     status: approved
     by: alice
     at: 2026-01-28T14:30:45Z
   ```
4. Appends a timestamped entry to the "## Approval Discussion" section
5. Auto-commits with message: `chant(<spec-id>): approve spec`

### 3. Execute the Spec

```bash
chant work 001-abc
```

Work proceeds normally after approval. When a spec has `approval.required: true`, `chant work` checks the approval status:

- **Pending**: Work is blocked. You must approve the spec first or use `--skip-approval`.
- **Rejected**: Work is blocked entirely. Address the feedback and get approval first.
- **Approved**: Work proceeds normally.

```bash
$ chant work 001
Error: Spec requires approval before work can begin

  Approval status: pending

  To approve:  chant approve 001 --by <name>
  To bypass:   chant work 001 --skip-approval
```

### Alternative: Reject the Spec

```bash
chant reject 001-abc --by bob --reason "Scope too large, split into auth and session management"
```

The spec cannot be worked on until the issues are addressed and it is re-approved.

**What happens during rejection:**

1. Validates the spec has `approval.required: true`
2. Validates the rejector name against git committers (warns if not found)
3. Updates the spec's approval frontmatter:
   ```yaml
   approval:
     required: true
     status: rejected
     by: bob
     at: 2026-01-28T14:30:45Z
   ```
4. Appends the rejection reason to the "## Approval Discussion" section
5. Auto-commits with message: `chant(<spec-id>): reject spec`
6. Applies the configured rejection action (see [Rejection Handling Modes](#rejection-handling-modes))

## Approval Discussion

Every approval or rejection adds a timestamped entry to the spec's "## Approval Discussion" section:

```markdown
## Approval Discussion

**alice** - 2026-01-28 14:30 - APPROVED

**bob** - 2026-01-28 10:15 - REJECTED
Please address the security vulnerabilities before approval
```

This provides a built-in audit trail directly in the spec file, tracked by git.

### Searching Discussions

Use `--mentions` to find specs where a person participated in the discussion:

```bash
chant list --mentions alice
```

Use `--count` to get just the number:

```bash
chant list --mentions alice --count
```

## Rejection Handling Modes

When a spec is rejected, the behavior depends on the `approval.rejection_action` config:

### Manual Mode (Default)

```yaml
# .chant/config.md
approval:
  rejection_action: manual
```

The spec stays rejected. The author must:
1. Read the rejection reason
2. Edit the spec to address feedback
3. Request re-approval with `chant approve`

### Dependency Mode

```yaml
approval:
  rejection_action: dependency
```

Chant automatically:
1. Creates a new spec: "Fix rejection issues for `<spec-id>`"
2. Blocks the original spec on the new fix spec
3. Once the fix spec is completed, the original spec unblocks

This is useful when the rejection identifies prerequisite work.

### Group Mode

```yaml
approval:
  rejection_action: group
```

Chant automatically:
1. Converts the rejected spec to a driver type
2. Creates numbered member specs (`.1`, `.2`, `.3`)
3. Distributes acceptance criteria across members
4. Members execute sequentially (each depends on the previous)

This is useful when the rejection is "scope too large, split this up."

## Activity Tracking

Monitor approval activity across all specs:

```bash
# All recent activity
chant activity

# Filter by person
chant activity --by alice

# Filter by time
chant activity --since 1d

# Filter by spec
chant activity --spec 001
```

Activity types include: `CREATED`, `APPROVED`, `REJECTED`, `WORKED`, and `COMPLETED`.

## Emergency Bypass

In emergencies, bypass the approval check:

```bash
chant work 001-abc --skip-approval
```

This should be used sparingly and only when the approval process would cause unacceptable delays.

## Filtering by Approval Status

```bash
# Find specs waiting for approval
chant list --approval pending

# Find rejected specs that need attention
chant list --approval rejected

# Count approved specs
chant list --approval approved --count

# Find specs with recent activity
chant list --activity-since 2h

# Find specs created by a specific person
chant list --created-by alice
```

### Visual Indicators

When listing specs, approval-related visual indicators are displayed:

| Indicator | Meaning |
|-----------|---------|
| `[needs approval]` (yellow) | Spec requires approval and is pending |
| `[rejected]` (red) | Spec has been rejected |
| `[approved]` (green) | Spec has been approved |
| `👤 <name>` | Created by indicator |
| `↩ <time>` | Time since last activity (e.g., `2h`, `3d`) |
| `💬 <count>` | Number of comments in approval discussion |
| `✓ <name>` (green) | Approved by indicator |

**Example output:**

```
✓ 2026-01-28-001-abc [approved] Implement feature     👤 alice ↩ 1h 💬 3 ✓ bob
⚠ 2026-01-28-002-def [needs approval] Fix bug         👤 charlie ↩ 30m
✗ 2026-01-28-003-ghi [rejected] Improve performance   👤 dave ↩ 2h 💬 5
```

## Example: Team Workflow

```bash
# Developer creates spec
chant add "Add OAuth2 support" --needs-approval

# Tech lead reviews and approves
chant approve 001-abc --by tech-lead

# Agent executes
chant work 001-abc

# If rejected instead:
chant reject 001-abc --by tech-lead --reason "Need to handle token refresh"
# Developer fixes the spec and re-submits
chant approve 001-abc --by tech-lead
chant work 001-abc
```

## Example: Solo Developer Self-Review

```bash
# Create spec with approval gate as a thinking checkpoint
chant add "Migrate database schema" --needs-approval

# Review your own spec after some time
chant show 001-abc
# Think about edge cases, rollback plan, etc.

# Approve when satisfied
chant approve 001-abc --by me
chant work 001-abc
```

## Agent-Assisted Work Approval

When AI agents (Claude, GPT, Copilot, Gemini, etc.) assist with code changes, chant can automatically require human approval before those changes can be merged. This provides a safety checkpoint for agent-written code.

### How It Works

1. **Detection**: During finalization, chant scans commit messages for agent co-authorship signatures (e.g., `Co-Authored-By: Claude`)
2. **Auto-requirement**: If detected and enabled, `approval.required` is automatically set to `true`
3. **Gate enforcement**: The spec cannot be merged until approved

### Configuration

Enable automatic approval requirements for agent work in your project or global config:

```yaml
# .chant/config.md or ~/.config/chant/config.md
---
approval:
  require_approval_for_agent_work: true
---
```

### Detected Agents

Chant detects co-authorship from these AI assistants:
- Claude (Anthropic)
- GPT/ChatGPT (OpenAI)
- Copilot (GitHub)
- Gemini (Google)
- Other common AI coding assistants

### Workflow Example

```bash
# Agent executes spec, creates commits with Co-Authored-By
chant work spec-id
# → Agent completes work
# → Commits include: Co-Authored-By: Claude <noreply@anthropic.com>

# During finalization, agent co-authorship detected
# → approval.required automatically set to true
# Output: "⚠ Agent co-authorship detected. Approval required before merge."

# Reviewer must approve
chant approve spec-id --by reviewer-name

# Now merge can proceed
chant merge spec-id
```

### Why Use This?

- **Safety**: Agent-written code may have subtle bugs or security issues
- **Review checkpoint**: Ensures a human reviews all agent changes before deployment
- **Audit trail**: Documents who reviewed and approved agent-assisted work
- **Team policy**: Enforce code review for all AI-generated code

### Emergency Bypass

In urgent situations, use `--skip-approval` to bypass the approval check:

```bash
chant work spec-id --skip-approval
```

This should be used sparingly and documented.
