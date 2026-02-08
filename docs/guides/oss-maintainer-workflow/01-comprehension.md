# Phase 1: Comprehension

Before reproducing the bug or guessing at fixes, you need to understand what the user is actually experiencing. Comprehension is about mapping the territory -- identifying where to look, not diagnosing what's broken.

## Starting the Investigation

You create a research spec for the comprehension phase:

```bash
$ chant add "Comprehension: issue #1234 data loss on concurrent writes"
Created spec: 2026-02-08-001-r4x
```

You edit the spec to set it up as a research task. The `target_files` field tells the agent where to write its findings:

```yaml
---
type: research
labels: [comprehension, issue-1234]
informed_by:
  - https://github.com/yourproject/kvstore/issues/1234
target_files:
  - .chant/research/issue-1234-comprehension.md
---
```

Then you run it:

```bash
$ chant work 001
Working 001-r4x: Comprehension: issue #1234 data loss on concurrent writes
> Agent working in worktree /tmp/chant-001-r4x
...
Completed in 1m 30s
```

## What the Agent Produces

The agent reads the issue thread, skims the relevant source files, checks for prior related issues, and writes a comprehension document:

```markdown
# Comprehension: Issue #1234

## Issue Type
Bug - User reports data loss during concurrent write operations

## Observable Symptom
When two CLI processes write to the same key simultaneously,
one write is silently lost. No error messages. The key contains
the value from one writer, but the other writer's data vanishes.

## Affected Components
| Component | Files | Relevance |
|-----------|-------|-----------|
| Storage write path | src/storage/store.rs | Primary write logic |
| Concurrency handling | src/storage/concurrent.rs | Lock mechanisms |
| CLI write command | src/cli/write.rs | Entry point |

## Missing Information
- Exact sequence of CLI commands that triggers it
- Whether data is completely lost or partially corrupted
- Frequency (every time? intermittent?)
```

At this point you know *what* to investigate, but not *why* it happens. That distinction matters. Comprehension gives you the map; root cause analysis is the expedition.

## The Decomposition Gate

Sometimes comprehension reveals that a single issue report contains multiple distinct bugs. The user filed one issue about "data loss in concurrent scenarios," but the agent's review finds three separate problems: a race condition in the storage layer, missing input validation in the CLI, and incorrect retry logic.

If these are truly independent bugs with different root causes, you decompose:

```bash
$ chant add "Comprehension: #1234 race condition in storage"
Created spec: 2026-02-08-002-abc

$ chant add "Comprehension: #1234 CLI input validation"
Created spec: 2026-02-08-003-def
```

Each gets its own investigation chain. You pursue one at a time, starting with the most severe.

If the symptoms look different but stem from the same root cause -- say, three write failure modes all caused by the same missing lock -- that's one bug with multiple symptoms. Don't decompose; keep it as a single investigation.

## When to Stop Early

Comprehension may reveal there's nothing to fix:

- **Not a bug.** The reported behavior is working as designed. Document the finding, close the issue with an explanation.
- **Can't action.** The fix would require breaking changes that conflict with stability guarantees. Document the trade-off, suggest a workaround.

In either case, the comprehension spec still has value as a record of what was investigated and why the decision was made.

## Collapsing Later Phases

After comprehension, if the root cause is already obvious -- say, the agent found an unlocked read-modify-write cycle in plain sight -- you can combine reproduction and root cause into a single research spec. But when in doubt, keep phases separate. Thorough research is easier to build on than incomplete research.

**Next:** [Reproduction](02-reproduction.md)
