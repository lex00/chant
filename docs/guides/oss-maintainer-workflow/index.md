# Open Source Maintainer Workflow

Command examples and output are illustrative -- your exact output will differ.

## The Scenario

You maintain `kvstore`, a Rust CLI tool for key-value storage. A user files issue #1234: concurrent writes to the same key silently lose data. Two CLI processes writing simultaneously should both succeed, but one write vanishes without an error.

This is the kind of issue that tempts you to grep for the write path and add a mutex. But a hasty fix without proper investigation leads to incomplete solutions, regressions, and poor documentation for future maintainers. Instead, you'll use chant to work through the issue systematically -- understanding before acting.

## The Research-First Approach

The workflow moves through six phases, each producing a spec that feeds the next:

```
Comprehension --> Reproduction --> Root Cause --> Impact Map --> Fork Fix --> Upstream PR
   (research)       (task)        (research)    (research)     (code)     (human gate)
```

Each phase is a separate spec. Research specs produce documents that inform later phases. The chain creates an auditable trail from issue report to merged fix.

### Why separate phases?

A single "fix the bug" spec would leave no record of what was investigated, what was ruled out, or why this approach was chosen over alternatives. Separate specs mean:

- **Auditability.** When someone asks "why was this fixed with locking instead of CAS?", the root cause spec explains the reasoning.
- **Resumability.** If the agent fails during root cause analysis, you reset that one spec -- not the entire investigation.
- **Collaboration.** One maintainer can do comprehension, another can pick up root cause, each with full context.

### When to skip phases

For trivial bugs -- typos, obvious one-liners, clear documentation errors -- go straight to implementation:

```bash
$ chant add "Fix typo in README storage section"
Created spec: 2026-02-08-001-abc
$ chant work 001
```

If comprehension reveals the report is not a bug or won't be fixed, document the finding and stop. Not every issue needs all six phases.

## The Investigation

The rest of this guide follows the kvstore concurrent write bug through each phase:

0. **[Setup](00-setup.md)** -- Configure silent mode for working on a shared repo
1. **[Comprehension](01-comprehension.md)** -- Understand what the issue is about
2. **[Reproduction](02-reproduction.md)** -- Create a failing test that proves the bug
3. **[Root Cause](03-root-cause.md)** -- Find out why data is lost
4. **[Impact Map](04-impact-map.md)** -- Discover what else is affected
5. **[Fork Fix](05-fork-fix.md)** -- Implement the fix and create a staging PR
6. **[Upstream PR](06-upstream-pr.md)** -- Human reviews and submits upstream
7. **[Advanced Patterns](08-advanced.md)** -- Single-spec mode, pausing, takeover

## How Specs Connect

Research specs pass knowledge forward through `informed_by` and `target_files`:

```yaml
# Comprehension produces a research document
target_files:
  - .chant/research/issue-1234-comprehension.md

# Root cause reads the comprehension output
informed_by:
  - .chant/research/issue-1234-comprehension.md
target_files:
  - .chant/research/issue-1234-root-cause.md

# Implementation reads all research
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
```

Each spec is a self-contained unit of work with its own acceptance criteria, but the `informed_by` chain ensures nothing is lost between phases.

## See Also

- [Lifecycle Walkthrough](../lifecycle-walkthrough.md) -- Core spec lifecycle concepts
- [Recovery & Resume](../recovery.md) -- Handling failed specs
