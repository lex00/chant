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

### 3. Execute the Spec

```bash
chant work 001-abc
```

Work proceeds normally after approval.

### Alternative: Reject the Spec

```bash
chant reject 001-abc --by bob --reason "Scope too large, split into auth and session management"
```

The spec cannot be worked on until the issues are addressed and it is re-approved.

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
