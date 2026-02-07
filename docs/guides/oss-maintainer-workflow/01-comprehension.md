# Comprehension Research

Understand what the issue is about before diving into reproduction or fixes.

## Why Comprehension First?

Before attempting to reproduce or fix an issue, you need to understand:

- **What problem** the user is experiencing
- **What context** surrounds the issue
- **What parts of the codebase** are likely involved
- **What information** is missing or unclear

A comprehension research spec produces a `target_files` list that identifies relevant code and documentation.

## Exit Criteria

Comprehension is complete when you can answer these questions:

### What You Should Know

- [ ] **Observable symptom** — What specific behavior is the user reporting? (e.g., "data loss during concurrent writes", not "writes don't work")
- [ ] **Affected code areas** — Which components, modules, or files are likely involved?
- [ ] **Prior work** — What related PRs, issues, or documentation exist? Have similar issues been reported or fixed before?
- [ ] **Missing information** — What details are absent from the issue report that will be needed for reproduction?

### What You Should NOT Know Yet

At the end of Comprehension, you should **NOT** have:

- A root cause hypothesis (save that for Root Cause stage)
- A specific fix in mind
- Line-level understanding of the bug

**Comprehension is about mapping the territory, not diagnosing the problem.** You're identifying where to look, not what's broken.

## Comprehension Workflow

Comprehension involves four sub-activities:

1. **Read issue thread** — Understand the user's report, context, and any discussion
2. **Review prior PRs/attempts** — Find related issues, PRs, or past fixes
3. **Read affected code** — Skim the modules and files that appear relevant
4. **Document observable symptoms** — Capture what is happening (not why)

```
GitHub Issue    Comprehension Spec    Understanding      Target Files
     │                 │                    │                  │
     ▼                 ▼                    ▼                  ▼
┌─────────┐      ┌───────────┐        ┌──────────┐      ┌───────────┐
│ Issue   │      │ Research  │        │ What is  │      │ informed  │
│ #1234   │─────▶│ what it   │───────▶│ affected │─────▶│ by: files │
│         │      │ is about  │        │          │      │ for repro │
└─────────┘      └───────────┘        └──────────┘      └───────────┘
```

## Creating a Comprehension Spec

```bash
chant add "Comprehension: issue #1234" --type research
```

Edit the spec:

```yaml
---
type: research
status: ready
labels:
  - comprehension
  - issue-1234
informed_by:
  - https://github.com/yourproject/issues/1234
target_files:
  - .chant/research/issue-1234-comprehension.md
---

# Phase 1: Comprehension - Understand Issue #1234

## Context

A user has reported issue #1234. Before jumping to solutions, we need to understand what they're actually experiencing.

## Issue Summary

[Paste or summarize the issue report here]

## Research Questions

- [ ] What specific behavior is the user reporting?
- [ ] What components are likely involved?
- [ ] What relevant code paths exist?
- [ ] What documentation or design docs are relevant?
- [ ] What information is missing from the report?

## Acceptance Criteria

- [ ] Issue type identified (bug, feature request, configuration, docs)
- [ ] Affected components identified
- [ ] Relevant source files listed in target_files
- [ ] Relevant documentation identified
- [ ] Missing information documented
```

## Comprehension Output

The spec produces a comprehension document with `target_files`:

```markdown
# Comprehension: Issue #1234

**Date:** 2026-02-02
**Analyzed by:** chant agent

## Issue Type

**Bug** - User reports data loss during concurrent write operations

## Summary

User experiences data loss when multiple CLI processes write to the same
key simultaneously. One write appears to be silently lost.

## Affected Components

| Component | Files | Relevance |
|-----------|-------|-----------|
| Storage write path | `src/storage/store.rs` | Primary write logic |
| Concurrency handling | `src/storage/concurrent.rs` | Locking mechanisms |
| CLI write command | `src/cli/write.rs` | Entry point for writes |

## Target Files for Next Phase

The following files should be examined during reproduction and root cause analysis:

- `src/storage/store.rs` - Core write implementation
- `src/storage/concurrent.rs` - Concurrency primitives
- `tests/storage/*.rs` - Existing storage tests
- `docs/architecture/storage.md` - Design documentation

## Missing Information

- [ ] Exact OS and version
- [ ] Specific sequence of CLI commands
- [ ] Whether data is completely lost or partially corrupted
- [ ] Error messages (if any)

## Recommendation

**Proceed to reproducibility phase** with focus on concurrent write scenarios.
Use target files listed above to inform reproduction test creation.
```

## Using target_files

The `target_files` output from comprehension feeds into the reproduction phase:

```yaml
# In reproduction spec:
informed_by:
  - .chant/specs/2026-02-02-001-abc.md  # Comprehension spec
  - .chant/research/issue-1234-comprehension.md  # Comprehension output
  - src/storage/store.rs  # From target_files
  - src/storage/concurrent.rs  # From target_files
```

## Decision Point: Collapsing Research Stages

After Comprehension, evaluate whether to proceed with separate research stages or collapse them:

### Keep All Stages Separate (Default)

Use separate Reproducibility and Root Cause stages when:
- **Genuine unknown** — Root cause is unclear and requires investigation
- **Complex reproduction** — Multiple ways to reproduce or unclear reproduction steps
- **Research-heavy** — Significant investigation needed after confirming reproducibility

### Collapse Stages 2-3 (Known Fix Path)

Combine Reproducibility and Root Cause into a single "Research" stage when:
- **Root cause is already apparent** — The fix is known from Comprehension
- **Simple validation needed** — Just need to confirm the obvious cause
- **Well-understood problem** — Issue matches known patterns or previous fixes

**Example of collapsing:**
```bash
# Instead of separate Reproducibility + Root Cause specs:
chant add "Research: Validate and confirm fix for #1234" --type research
# This single spec both confirms reproduction AND validates the known fix approach
```

**When in doubt:** Keep stages separate. It's easier to have thorough research than to redo incomplete research.

## Comprehension vs Triage

**Old approach (triage):** Categorize and prioritize issues
**New approach (comprehension):** Research to understand the issue

Comprehension is deeper than triage:
- Reads relevant code and documentation
- Identifies specific files and components
- Produces target_files for next phases
- More than just categorization

## Decomposition Gate

**When an issue contains multiple distinct bugs**, Comprehension may reveal that what appeared to be one issue is actually several independent problems requiring separate fixes.

### When to Decompose

Decompose when Comprehension reveals:

- **Multiple distinct root causes** — The symptoms point to unrelated bugs in different subsystems
- **Separable failures** — Each bug can be fixed independently without blocking the others
- **Umbrella issues** — The reporter grouped multiple problems into one issue for convenience

**Example:** Issue #38109 "Data loss in concurrent scenarios" turns out to contain:
1. Race condition in storage layer (locking bug)
2. Missing validation in CLI input (separate issue)
3. Incorrect error handling in retry logic (separate issue)
4. Documentation gap about concurrency guarantees (non-code fix)
5. Test coverage gap exposing all of the above

### When NOT to Decompose

Do NOT decompose when:

- **Single root cause, multiple symptoms** — Different error messages or failures that stem from the same underlying bug
- **Tightly coupled changes** — Fixes that must be implemented together to avoid breaking behavior
- **Sequential dependencies** — One fix is a prerequisite for another

**Example:** "Writes fail in three different scenarios" where all three failures are caused by the same incorrect lock scope — this is ONE bug with multiple symptoms, not three bugs.

### How to Decompose

When decomposition is needed:

1. **Create separate spec chains** for each distinct bug:
   ```bash
   # Bug 1: Race condition
   chant add "Comprehension: #38109 race condition" --type research
   chant add "Root cause: #38109 race condition" --type research
   chant add "Fix #38109: Add proper locking" --type code

   # Bug 2: Missing validation
   chant add "Comprehension: #38109 input validation" --type research
   chant add "Fix #38109: Add CLI input validation" --type code

   # Bug 3: Error handling
   chant add "Comprehension: #38109 retry logic" --type research
   chant add "Fix #38109: Fix retry error handling" --type code
   ```

2. **Pick one bug to pursue first** based on:
   - Severity and user impact
   - Clarity of the issue (some may need more investigation)
   - Dependencies (fix foundational issues before dependent ones)

3. **Document the others** in issue comments or separate tracking issues:
   ```markdown
   ## Decomposition Result

   Issue #38109 contains 5 distinct problems:

   1. Race condition (High priority) — Pursuing via specs 001-abc, 002-def
   2. Input validation (Medium) — Tracked in #38110
   3. Retry error handling (Medium) — Tracked in #38111
   4. Documentation (Low) — Tracked in #38112
   5. Test coverage (Ongoing) — Addressed as each bug is fixed
   ```

**Exit from Comprehension:** If decomposition is needed, complete the Comprehension spec, document the decomposition decision, and create follow-up specs for each distinct bug. Proceed with one bug at a time through the full workflow.

## See Also

- [Reproducibility](02-reproduction.md) — Next step: create failing test
- [Root Cause Research](03-root-cause.md) — Uses comprehension target_files
