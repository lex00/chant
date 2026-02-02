# Comprehension Research

Understand what the issue is about before diving into reproduction or fixes.

## Why Comprehension First?

Before attempting to reproduce or fix an issue, you need to understand:

- **What problem** the user is experiencing
- **What context** surrounds the issue
- **What parts of the codebase** are likely involved
- **What information** is missing or unclear

A comprehension research spec produces a `target_files` list that identifies relevant code and documentation.

## Comprehension Workflow

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

# Comprehension: issue #1234

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

## Comprehension vs Triage

**Old approach (triage):** Categorize and prioritize issues
**New approach (comprehension):** Research to understand the issue

Comprehension is deeper than triage:
- Reads relevant code and documentation
- Identifies specific files and components
- Produces target_files for next phases
- More than just categorization

## See Also

- [Reproducibility](02-reproduction.md) — Next step: create failing test
- [Root Cause Research](03-root-cause.md) — Uses comprehension target_files
