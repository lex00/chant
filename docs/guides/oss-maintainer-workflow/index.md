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
Issue Report    Triage      Reproduction     Research        Implementation    Review
    │             │              │               │                  │             │
    ▼             ▼              ▼               ▼                  ▼             ▼
┌─────────┐  ┌─────────┐   ┌──────────┐   ┌───────────┐    ┌────────────┐   ┌─────────┐
│ GitHub  │  │ Assess  │   │ Failing  │   │ Root Cause│    │  Informed  │   │ Verify  │
│  Issue  │─▶│ Quality │─▶ │  Test    │─▶ │  Analysis │─▶  │    Fix     │─▶ │ & Ship  │
│         │  │         │   │          │   │           │    │            │   │         │
└─────────┘  └─────────┘   └──────────┘   └───────────┘    └────────────┘   └─────────┘
                │               │              │                 │
                ▼               ▼              ▼                 ▼
            triage.md     reproduce.md   research.md       implement.md
              spec            spec           spec              spec
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
| [Triage](01-triage.md) | `task` | Assessment + questions | Filter and categorize incoming issues |
| [Reproduction](02-reproduction.md) | `task` | Failing test | Confirm and isolate the bug |
| [Research](03-research.md) | `task` | RCA document | Deep investigation of root cause |
| [Implementation](04-implementation.md) | `code` | Working fix | Informed fix based on research |
| [Review](05-review.md) | `task` | Approval decision | Independent validation |
| [Documentation](06-documentation.md) | `documentation` | Updated docs | User-facing documentation |
| [Release](07-release.md) | `task` | Release notes | User-friendly changelog |

## Quick Start

Here's a minimal example of the full workflow:

```bash
# 1. Triage the issue
chant add "Triage issue #1234: Data loss on concurrent writes" --type task
# Edit spec, then:
chant work <triage-spec-id> --prompt triage

# 2. Create reproduction case
chant add "Reproduce issue #1234" --type task
# Edit spec to add: informed_by: [<triage-spec-id>]
chant work <repro-spec-id> --prompt reproduce

# 3. Research root cause
chant add "Research root cause: issue #1234" --type task
# Edit spec to add: informed_by: [<repro-spec-id>, relevant-docs, source-files]
chant work <research-spec-id> --prompt research

# 4. Implement the fix
chant add "Fix issue #1234: Use locking for concurrent writes" --type code
# Edit spec to add: informed_by: [<research-spec-id>]
chant work <impl-spec-id> --prompt implement

# 5. Independent review
chant add "Review fix for issue #1234" --type task
# Edit spec to add: informed_by: [<repro-spec-id>, <impl-spec-id>]
chant work <review-spec-id> --prompt review
chant approve <impl-spec-id> --by "maintainer-name"

# 6. Update documentation (if needed)
chant drift  # Check for documentation drift
chant add "Document concurrent write locking API" --type documentation
# Edit spec to add: informed_by: [<impl-spec-id>]
chant work <doc-spec-id> --prompt document

# 7. Merge all completed specs
chant merge --all
```

## Custom Prompts

This workflow uses custom prompts for each stage. See [Prompt Guide](../prompts.md) for general prompt concepts, then install these prompts:

| Prompt | Stage | Purpose |
|--------|-------|---------|
| `triage` | Triage | Assess and categorize issues |
| `reproduce` | Reproduction | Create minimal failing tests |
| `research` | Research | Deep root cause investigation |
| `implement` | Implementation | Informed fix development |
| `review` | Review | Independent validation |
| `document` | Documentation | User-facing documentation |

Copy the prompt files from the [prompts section](#installing-prompts) to `.chant/prompts/` in your project.

## Guide Pages

1. **[Issue Triage](01-triage.md)** — Assess incoming issues before deep work
2. **[Reproduction Case](02-reproduction.md)** — Create minimal failing tests
3. **[Root Cause Analysis](03-research.md)** — Deep investigation workflow (core concept)
4. **[Implementation](04-implementation.md)** — Informed fix development (core concept)
5. **[Validation & Review](05-review.md)** — Independent verification and approval
6. **[Documentation](06-documentation.md)** — Documentation specs with drift detection
7. **[Release Coordination](07-release.md)** — Aggregate fixes into release notes
8. **[Advanced Patterns](08-advanced.md)** — Security, breaking changes, parallel work
9. **[Complete Walkthrough](09-example.md)** — Real-world example from start to finish

## Installing Prompts

Create these files in `.chant/prompts/`:

- [`triage.md`](../../prompts/triage.md) — Issue triage prompt
- [`reproduce.md`](../../prompts/reproduce.md) — Reproduction case prompt
- [`research.md`](../../prompts/research.md) — Root cause analysis prompt
- [`implement.md`](../../prompts/implement.md) — Implementation prompt
- [`review.md`](../../prompts/review.md) — Review prompt
- [`document.md`](../../prompts/document.md) — Documentation prompt

See [Advanced Patterns](08-advanced.md) for prompt customization based on issue type.

## Prerequisites

- Familiarity with [core concepts](../../concepts/specs.md)
- Understanding of [spec types](../../concepts/spec-types.md)
- Basic knowledge of [prompts](../../concepts/prompts.md)

## See Also

- [Research Workflows Guide](../research.md) — General research workflow concepts
- [Approval Workflow](../approval-workflow.md) — Approval gates and review
- [Recovery & Resume](../recovery.md) — Handling failed specs
