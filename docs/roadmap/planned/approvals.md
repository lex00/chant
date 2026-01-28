# Approvals

> **Status: Not Implemented** ❌
>
> This feature is documented as a planned workflow enhancement.
> It is not currently implemented. See [Roadmap](../roadmap.md) - Phase 8 "Not yet implemented".

## Three Types of Approval

| Type | When | Question Answered |
|------|------|-------------------|
| **Spec Approval** | Before work | "Is this the right thing to build?" |
| **Code Approval** | After work | "Is this implementation acceptable?" |
| **Completion Approval** | After merge | "Are acceptance criteria met?" |

All three are optional. All three work locally without external systems.

## Spec Approval

Approve the spec before agent execution.

```yaml
---
status: pending
approval:
  required: true
  status: pending      # pending, approved, rejected
  approved_by: null
  approved_at: null
---

# Add authentication

## Acceptance Criteria
- [ ] JWT tokens work
- [ ] 401 on invalid token
```

### Workflow

```bash
# Create spec that needs approval
chant add "Add authentication" --needs-approval

# Someone reviews the spec
chant approve 001 --by alice
# Or reject with reason
chant reject 001 --by bob --reason "Scope too large, split first"

# Now work can start
chant work 001
```

### Without Approval

```bash
chant work 001
Error: Spec requires approval

Approval: pending
Use 'chant approve 001' or 'chant work 001 --skip-approval'
```

### Configuration

```yaml
# config.md
defaults:
  approval:
    spec: false         # Require spec approval by default
    code: false         # Require code review by default
    completion: false   # Require completion sign-off by default
```

## Code Approval (Review)

Review the implementation after agent completes.

```yaml
---
status: completed
branch: chant/2026-01-22-001-x7m
commit: a1b2c3d4
review:
  required: true
  status: pending      # pending, approved, changes_requested, rejected
  reviews: []
---
```

### Embedded Reviews

Reviews live in the spec file:

```markdown
---
status: completed
review:
  status: approved
  reviews:
    - by: alice
      at: 2026-01-22T15:30:00Z
      verdict: approved
    - by: bob
      at: 2026-01-22T16:00:00Z
      verdict: approved
---

# Add authentication

[spec content...]

## Reviews

### alice - 2026-01-22 15:30 - APPROVED

Looks good. Clean implementation.

- [x] Error handling is solid
- [x] Tests cover edge cases

### bob - 2026-01-22 16:00 - APPROVED

LGTM. One minor suggestion for future:

- Consider rate limiting on login endpoint (not blocking)
```

### Review Commands

```bash
# Request review
chant review request 001

# Add review
chant review 001 --by alice --approve
chant review 001 --by bob --approve --comment "LGTM"
chant review 001 --by carol --request-changes --comment "See inline"

# Check status
chant review status 001
```

### Using Prompts for Review

The `code-review` prompt can automate initial review:

```bash
chant work 001 --prompt code-review
```

Agent reviews the code and appends findings to spec file.

## Completion Approval

Verify acceptance criteria are actually met.

```yaml
---
status: completed
review:
  status: approved
completion:
  required: true
  status: pending
  verified_by: null
  verified_at: null
---
```

### Workflow

```bash
# After code is merged
chant complete 001 --verify

# Someone checks acceptance criteria
chant verify 001 --by alice
# Marks each criterion as verified

# Or reject
chant verify 001 --by bob --failed --reason "JWT not working in prod"
```

### In Spec File

```markdown
## Acceptance Criteria

- [x] JWT tokens work <!-- verified: alice, 2026-01-22 -->
- [x] 401 on invalid token <!-- verified: alice, 2026-01-22 -->

## Completion

Verified by: alice
Verified at: 2026-01-22T17:00:00Z
```

## Approval States

```
Spec Approval:     pending → approved → [work starts]
                          ↘ rejected

Code Approval:     pending → approved → [can merge]
                          ↘ changes_requested → [fix] → pending
                          ↘ rejected

Completion:        pending → verified → [done]
                          ↘ failed → [reopen spec]
```

## Who Can Approve

### Default (Anyone)

```yaml
approval:
  required: true
  # Anyone can approve
```

### Specific People

```yaml
approval:
  required: true
  approvers: [alice, bob]  # Only these people
```

### Role-Based

```yaml
approval:
  required: true
  approvers:
    - role: tech-lead
    - role: security  # For security-tagged specs
```

### Count Required

```yaml
approval:
  required: true
  min_approvals: 2  # Need 2 approvals
```

## No External System Required

Everything works locally:

```bash
# Solo dev with self-review checkpoint
chant add "Risky refactor" --needs-approval
# Think about it...
chant approve 001 --by me
chant work 001
# Review my own work...
chant review 001 --by me --approve
```

Git history tracks all approvals. Markdown is human-readable.

## Summary

| Approval | Stored In | Commands |
|----------|-----------|----------|
| Spec | `approval:` frontmatter | `chant approve/reject` |
| Code | `review:` frontmatter + body | `chant review` |
| Completion | `completion:` frontmatter | `chant verify` |

All optional. All local. All in markdown.
