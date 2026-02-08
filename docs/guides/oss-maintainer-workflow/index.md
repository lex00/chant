# Open Source Maintainer Workflow

A complete guide for open source maintainers showing how to use chant for research-driven issue resolution.

## The Challenge

Open source maintainers face a common problem: incoming issues require deep investigation before implementation. A hasty fix without proper root cause analysis often leads to:

- Incomplete solutions that miss edge cases
- Regressions in related functionality
- Wasted time on symptoms instead of causes
- Poor documentation for future maintainers

## The Research-First Approach

Chant enables a systematic workflow where understanding precedes action:

```
Issue       Comprehension   Repro    Root Cause   Impact Map  Fork Fix    Upstream PR
Report         Research                Research    Research                (Human Gate)
  │               │            │          │           │            │             │
  ▼               ▼            ▼          ▼           ▼            ▼             ▼
┌──────┐    ┌──────────┐  ┌──────┐  ┌──────────┐ ┌────────┐ ┌─────────┐  ┌──────────┐
│GitHub│    │Understand│  │Test  │  │Find Root │ │Expand  │ │Fix in   │  │Human     │
│Issue │───▶│What It   │─▶│Repro │─▶│Cause     │─▶│View    │─▶│Fork +   │─▶│Creates   │
│      │    │Is About  │  │      │  │          │ │        │ │Staging  │  │Real PR   │
└──────┘    └──────────┘  └──────┘  └──────────┘ └────────┘ │PR       │  └──────────┘
                │             │          │           │        └─────────┘
                ▼             ▼          ▼           ▼             │
           target_files  reproduce  target_files target_files informed_by
              spec         spec        spec        spec      (research specs)
```

Each stage produces a spec that informs the next, creating an auditable trail from issue to resolution.

## Key Benefits

### Auditability

Every decision is documented in specs. When someone asks "why was this fixed this way?", the research spec explains the reasoning.

### Reproducibility

Research specs capture the investigation process. Future similar issues can reference past analysis rather than starting from scratch.

### Quality

Implementation specs reference research findings, ensuring fixes address root causes rather than symptoms.

### Collaboration

Specs serve as handoff documents. One maintainer can triage, another can research, a third can implement—all with full context.

## Workflow Stages

| Stage | Spec Type | Output | Purpose |
|-------|-----------|--------|---------|
| [Comprehension](01-comprehension.md) | `research` | `target_files` + decomposition gate | State observable symptom, affected areas, prior work—no hypothesis yet. Includes gate: decompose if multiple distinct bugs found |
| [Reproducibility](02-reproduction.md) | `task` | Failing test or instructions | Confirm and isolate the bug (auto/assisted) |
| [Root Cause](03-root-cause.md) | `research` | `target_files` | Determine what needs to be fixed |
| [Impact Map](04-impact-map.md) | `research` | `target_files` | Expand view based on root cause |
| [Fork Fix](05-fork-fix.md) | `code` | Working fix + staging PR | Fix in fork, create fork-internal PR |
| [Upstream PR](06-upstream-pr.md) | `task` | Real PR | Human gate → create upstream PR |

## Quick Path for Simple Fixes

For trivial bugs (typos, obvious one-line fixes, clear documentation errors), you can skip the research phases:

```bash
# For simple fixes, go directly to implementation
chant add "Fix typo in README"
# Edit the created spec to set type: code in frontmatter
chant work <spec-id>
```

**When to use Quick Path:**
- Typos in documentation or code comments
- Obvious one-line bug fixes with no side effects
- Clear, isolated changes with minimal scope

**When NOT to use Quick Path:**
- Anything involving logic changes
- Bugs with unclear root causes
- Changes affecting multiple components

## Early Exit: "Not a Bug" or "Won't Fix"

After comprehension research, you may determine the issue should be closed without a fix:

**Not a Bug:**
```markdown
## Comprehension Outcome

**Result:** Working as designed

The reported behavior is intentional per the design doc (docs/design/storage.md).
User expected last-write-wins semantics, but the system implements first-write-wins
by design to prevent data races.

**Recommendation:** Close issue with explanation, improve documentation.
```

**Won't Fix:**
```markdown
## Comprehension Outcome

**Result:** Won't fix

The requested feature would require breaking changes to the public API and
conflicts with the project's stability guarantees. The workaround (using
manual locking) is sufficient for this use case.

**Recommendation:** Close issue, suggest workaround.
```

**When to exit early:**
- After comprehension: issue is not actionable
- After reproduction: cannot reproduce, likely user error
- After root cause: fix would be harmful (security, breaking changes)

## Getting Started: Setup for OSS Maintainers

**Before diving into the workflow,** OSS maintainers should configure silent mode to keep specs personal and local:

```bash
# Initialize chant
chant init

# Enable silent mode (keeps .chant/ local, not tracked in git)
chant silent
```

**Why this matters:** Silent mode keeps your research specs, investigation notes, and work-in-progress out of the shared repository's git history. You get the full workflow benefits while keeping the repo clean for collaborators.

**Full setup guide:** See [Setup for OSS Maintainers](00-setup.md) for complete configuration details.

---

## Quick Start

Here's a minimal example of the full workflow:

```bash
# 1. Comprehension research
chant add "Comprehension: issue #1234"
# Edit spec to set: type: research, target_files: [.chant/research/issue-1234-comprehension.md]
chant work <comprehension-spec-id>

# 2. Reproducibility
chant add "Reproduce issue #1234"
# Edit spec to set: type: task, informed_by: [<comprehension-spec-id>]
chant work <repro-spec-id>

# 3. Root cause research
chant add "Root cause: issue #1234"
# Edit spec to set: type: research, informed_by: [<comprehension-spec-id>, <repro-spec-id>],
#                   target_files: [.chant/research/issue-1234-root-cause.md]
chant work <root-cause-spec-id>

# 4. Impact map research
chant add "Impact Map: issue #1234"
# Edit spec to set: type: research, informed_by: [<root-cause-spec-id>],
#                   target_files: [.chant/research/issue-1234-impact-map.md]
chant work <impact-map-spec-id>

# 5. Fork fix with staging PR
chant add "Fix issue #1234: Use locking for concurrent writes"
# Edit spec to set: type: code, informed_by: [<root-cause-spec-id>, <impact-map-spec-id>]
chant work <impl-spec-id>
# Agent creates staging PR in fork (not upstream)

# 6. Human gate → upstream PR
# Human reviews staging PR in fork, then creates real PR to upstream
gh pr create --base upstream/main --title "Fix #1234"
```

## Workflow Summary

The six phases work together:

1. **Comprehension** → Understand the issue, produce `target_files`
2. **Reproducibility** → Confirm with test or instructions
3. **Root Cause** → Find the bug, produce `target_files`
4. **Impact Map** → Expand view, produce complete `target_files`
5. **Fork Fix** → Implement + staging PR in fork
6. **Upstream PR** → Human reviews staging PR → creates upstream PR

## Guide Pages

0. **[Setup for OSS Maintainers](00-setup.md)** — Configure silent mode and initial setup
1. **[Comprehension Research](01-comprehension.md)** — Understand what the issue is about
2. **[Reproducibility](02-reproduction.md)** — Create failing tests (auto/assisted)
3. **[Root Cause Research](03-root-cause.md)** — Determine what needs to be fixed
4. **[Impact Map Research](04-impact-map.md)** — Expand view based on root cause
5. **[Fork Fix + Staging PR](05-fork-fix.md)** — Fix in fork with fork-internal PR
6. **[Upstream PR](06-upstream-pr.md)** — Human gate before creating real PR
7. **[Advanced Patterns](08-advanced.md)** — Configuration and advanced workflows

## Key Concepts

### target_files Pattern

Research specs produce `target_files` that feed into later phases:

```yaml
# Comprehension research
target_files:
  - .chant/research/issue-1234-comprehension.md

# Root cause research uses comprehension output
informed_by:
  - .chant/research/issue-1234-comprehension.md
target_files:
  - .chant/research/issue-1234-root-cause.md

# Implementation uses all research
informed_by:
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-impact-map.md
```

### Fork-Staging Pattern

Fork-internal PRs serve as quality gates:

1. Agent implements in fork
2. Agent creates staging PR (fork branch → fork main)
3. Human reviews staging PR
4. Human creates upstream PR (fork branch → upstream main)

## Prerequisites

- Familiarity with [core concepts](../../concepts/specs.md)
- Understanding of [spec types](../../concepts/spec-types.md)
- Basic knowledge of [prompts](../../concepts/prompts.md)

## See Also

- [OSS Maintainer Workflow Example](../../../examples/oss-maintainer-workflow/) — Complete example with 6-phase bug fix
- [Research Workflows Guide](../research.md) — General research workflow concepts
- [Approval Workflow](../approval-workflow.md) — Approval gates and review
- [Recovery & Resume](../recovery.md) — Handling failed specs
