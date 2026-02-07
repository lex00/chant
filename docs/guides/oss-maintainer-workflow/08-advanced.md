# Advanced Patterns

## Working on Fix Branches

When working on a specific issue branch instead of main:

```yaml
# .chant/config.md
defaults:
  main_branch: "fix/issue-123"  # Target for merges
```

This lets you:
- Create specs for your fix
- Work in isolated worktrees
- Merge spec work into your fix branch (not main)

## Controlling Running Work

### Pausing Work

Stop a running agent without losing progress:

```bash
chant pause <spec-id>
```

The agent stops immediately and the spec status is set to `paused`. Use this when:
- You need to make a human decision before continuing
- The spec is blocked on external information
- You're taking a break and want to resume later

Resume with `chant work <spec-id>` or `chant resume <spec-id>`.

**Example:** You're running a research spec to evaluate libraries, but realize you need maintainer input on architectural constraints. Pause the spec, gather input, then resume.

### Taking Over Work

Pause and prepare a spec for manual continuation:

```bash
chant takeover <spec-id>
```

This command:
1. Pauses the running agent
2. Analyzes the execution log
3. Updates the spec with progress summary and next steps

Use takeover when:
- The agent is heading in the wrong direction
- You want to provide human guidance on how to proceed
- The work needs a different approach than the agent chose

**MCP integration:** The `chant_takeover` tool is available for agent-to-agent handoff scenarios.

**Example:** An implementation spec is repeatedly failing tests with the same approach. Take over, review what's been tried, and manually guide the next attempt or fix it yourself.

## Single-Spec Investigation Mode

The full OSS workflow uses six separate specs (comprehension, reproduction, root cause, impact map, fork fix, upstream PR). This provides excellent auditability and enables multi-agent handoffs, but creates overhead for a solo investigator working a single issue in one session.

For focused investigations where you want the full research rigor without the multi-spec ceremony, use a single spec with stage markers instead of separate files.

### When to Use Single-Spec Mode

**Use single-spec mode when:**
- Working solo (one human + one agent)
- Single investigation session (hours, not days)
- You want a continuous narrative with all findings in one place
- Overhead of creating/updating multiple specs feels excessive

**Use full 6-spec workflow when:**
- Multiple people will work the issue (handoffs benefit from clear boundaries)
- Investigation spans multiple days/sessions (resumption benefits from discrete units)
- Complex issues requiring parallel research branches
- You need granular dependency tracking between phases

### Single-Spec Template

Create a research spec with all stages in one document:

```yaml
---
type: research
status: pending
labels:
- oss-workflow
- investigation
target_files:
- .chant/research/issue-1234-investigation.md
---
# Investigation: issue #1234 [Short Title]

## Task

Complete full investigation of issue #1234 from comprehension through impact analysis. Document findings in `.chant/research/issue-1234-investigation.md` with stage markers.

## Acceptance Criteria

- [ ] Comprehension stage completed
- [ ] Reproduction confirmed
- [ ] Root cause identified
- [ ] Impact map analyzed
- [ ] All findings documented in target file
```

**Target file structure (`.chant/research/issue-1234-investigation.md`):**

```markdown
# Investigation: issue #1234 [Short Title]

## Stage 1: Comprehension

**Goal:** Understand what the issue is about

### Issue Summary
[Brief description of reported problem]

### Key Observations
- [Finding 1]
- [Finding 2]

### Initial Hypothesis
[What you think might be happening]

---

## Stage 2: Reproduction

**Goal:** Confirm and isolate the bug

### Reproduction Steps
1. [Step 1]
2. [Step 2]

### Expected vs Actual
- Expected: [description]
- Actual: [description]

### Test Case
[Code snippet or test that demonstrates the issue]

---

## Stage 3: Root Cause

**Goal:** Determine what needs to be fixed

### Analysis
[Deep dive into the code]

### Root Cause
[The actual bug or design flaw]

### Evidence
- [Evidence 1]
- [Evidence 2]

---

## Stage 4: Impact Map

**Goal:** Expand view based on root cause

### Affected Components
- `path/to/file.rs` - [description]
- `path/to/other.rs` - [description]

### Side Effects
[What else might be affected by a fix]

### Related Issues
[Links to similar past issues]

---

## Summary

**Root Cause:** [One sentence]

**Fix Strategy:** [High-level approach]

**Target Files for Implementation:**
- `path/to/file.rs`
- `path/to/test.rs`
```

### Usage Example

```bash
# Create single investigation spec
chant add "Investigation: issue #1234 connection leak" --type research

# Edit spec to add target_files and template
# (Use the single-spec template above)

# Work the spec - agent completes all stages in one pass
chant work <spec-id>

# When investigation is complete, create implementation spec
chant add "Fix issue #1234: Release connections properly" --type code
# Set informed_by to reference the investigation spec
chant work <impl-spec-id>
```

### Benefits vs Trade-offs

**Benefits:**
- Single document with continuous narrative
- Less overhead (one spec vs six)
- Easier to see progression and connections
- Better for straightforward issues

**Trade-offs:**
- Harder to parallelize (one agent at a time)
- Less granular resumption (can't easily restart from "just root cause")
- Informal stage boundaries (no separate acceptance criteria per stage)
- Loses some auditability (no per-stage git commits)

The single-spec mode preserves the research rigor and deliverables of the full workflow while reducing ceremony for solo investigators.
