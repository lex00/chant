# Issue Triage

Assess incoming issues before committing to deep investigation.

## Why Triage?

Not every issue deserves immediate attention:

- **Duplicates** waste effort on already-tracked problems
- **Incomplete reports** need clarification before investigation
- **Feature requests** disguised as bugs need redirection
- **Low-severity issues** can wait while critical bugs get priority

A triage spec creates a structured assessment that guides next steps.

## Triage Workflow

```
GitHub Issue    Triage Spec       Assessment         Decision
     │              │                 │                  │
     ▼              ▼                 ▼                  ▼
┌─────────┐    ┌─────────┐      ┌──────────┐      ┌───────────┐
│ Issue   │    │ Run     │      │ Category │      │ Close     │
│ #1234   │───▶│ triage  │─────▶│ Priority │─────▶│ Defer     │
│         │    │ prompt  │      │ Questions│      │ Proceed   │
└─────────┘    └─────────┘      └──────────┘      └───────────┘
```

## Creating a Triage Spec

```bash
chant add "Triage issue #1234: Unexpected behavior on save" --type task
```

Edit the spec:

```yaml
---
type: task
status: ready
prompt: triage
labels:
  - triage
  - issue-1234
informed_by:
  - https://github.com/yourproject/issues/1234
target_files:
  - .chant/triage/issue-1234.md
---

# Triage issue #1234: Unexpected behavior on save

## Issue Summary

[Paste or summarize the issue report here]

## Acceptance Criteria

- [ ] Issue categorized (bug, feature, docs, question, duplicate)
- [ ] Priority assigned (critical, high, medium, low)
- [ ] Severity assessed (blocking, degraded, cosmetic)
- [ ] Missing information identified
- [ ] Recommendation provided (close, defer, needs-reproduction, ready-for-research)
```

## The Triage Prompt

The `triage` prompt instructs the agent to assess systematically:

```markdown
You are an open source project maintainer triaging a new issue.

Your goal is to:
1. Assess the issue quality and completeness
2. Categorize: bug, feature, documentation, question, duplicate
3. Determine priority (critical, high, medium, low) and severity
4. Identify missing information

Output:
- Clear categorization and priority assessment
- List of clarifying questions (formatted as GitHub comment)
- Recommendation: close, defer, needs-reproduction, or ready-for-research
```

See [Installing Prompts](index.md#installing-prompts) for the full prompt file.

## Triage Categories

### Bug

Confirmed or suspected incorrect behavior:

```markdown
**Category:** Bug
**Priority:** High
**Severity:** Degraded functionality

**Reasoning:** User reports data loss, which if confirmed would be serious.
The description is plausible given the concurrent write scenario.

**Recommendation:** needs-reproduction
```

### Feature Request

Enhancement disguised as a bug:

```markdown
**Category:** Feature request
**Priority:** Low
**Severity:** N/A

**Reasoning:** Current behavior matches documented design.
User wants different behavior, which is a valid feature request.

**Recommendation:** Label as "enhancement", move to feature backlog
```

### Documentation

Missing or unclear documentation:

```markdown
**Category:** Documentation
**Priority:** Medium
**Severity:** N/A

**Reasoning:** Behavior is correct, but not documented.
User confusion is valid given current docs.

**Recommendation:** Create documentation spec
```

### Question

Support request rather than issue:

```markdown
**Category:** Question
**Priority:** Low
**Severity:** N/A

**Reasoning:** User asking how to accomplish task.
No bug or missing feature indicated.

**Recommendation:** Answer in issue, close, suggest discussion forum
```

### Duplicate

Previously reported issue:

```markdown
**Category:** Duplicate of #987
**Priority:** N/A
**Severity:** N/A

**Reasoning:** Same symptoms and reproduction steps as #987.

**Recommendation:** Close as duplicate, link to original
```

## Generating Clarifying Questions

When information is missing, the triage spec produces questions formatted for GitHub:

```markdown
## Clarifying Questions

Thank you for reporting this issue! To help us investigate, could you provide:

1. **Environment details:**
   - Operating system and version?
   - Project version (run `yourproject --version`)?
   - Rust version if building from source?

2. **Reproduction steps:**
   - Minimal example that triggers the issue?
   - Does it happen every time or intermittently?

3. **Expected vs actual behavior:**
   - What did you expect to happen?
   - What actually happened?

4. **Error messages:**
   - Any error output or logs?
   - Stack trace if applicable?
```

## Triage Output

The spec produces a triage document at the `target_files` location:

```markdown
# Triage: Issue #1234

**Date:** 2026-01-29
**Triaged by:** chant agent

## Assessment

| Field | Value |
|-------|-------|
| Category | Bug |
| Priority | High |
| Severity | Degraded |
| Completeness | Partial |

## Summary

User reports data loss when saving files with concurrent write operations.
The report is missing specific reproduction steps and environment details.

## Missing Information

- [ ] OS and version
- [ ] Exact sequence of operations
- [ ] Whether data is lost or just not visible

## Clarifying Questions

[GitHub-formatted questions here]

## Recommendation

**needs-reproduction**

Once user provides missing details, create a reproduction spec to confirm
the bug before investigating root cause.

## Next Steps

1. Post clarifying questions on issue #1234
2. Wait for user response
3. When details provided, create reproduction spec:
   `chant add "Reproduce issue #1234" --type task`
```

## Priority Guidelines

| Priority | Criteria | Response Time |
|----------|----------|---------------|
| Critical | Data loss, security, complete breakage | Same day |
| High | Major feature broken, significant impact | This week |
| Medium | Minor feature broken, workaround exists | This release |
| Low | Cosmetic, edge case, enhancement | Backlog |

## Severity Guidelines

| Severity | Description |
|----------|-------------|
| Blocking | Cannot use the software at all |
| Degraded | Feature broken but software usable |
| Cosmetic | Visual issue, no functional impact |

## When to Skip Triage

Some issues don't need formal triage:

- **Obvious fixes:** Typos, simple bugs with clear solutions
- **Your own issues:** You already understand the context
- **Urgent security:** Go directly to research and fix

But even quick fixes benefit from a lightweight spec for tracking.

## Batch Triage

When multiple issues need triage, use labels:

```bash
# Create multiple triage specs
chant add "Triage issue #1234" --type task --label triage-batch-01
chant add "Triage issue #1235" --type task --label triage-batch-01
chant add "Triage issue #1236" --type task --label triage-batch-01

# Execute in parallel
chant work --parallel --label triage-batch-01
```

## See Also

- [Reproduction Case](02-reproduction.md) — Next step after "needs-reproduction"
- [Research](03-research.md) — Next step after "ready-for-research"
- [Advanced Patterns](08-advanced.md) — Security issue handling
