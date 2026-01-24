# Philosophy

**Intent based. Spec driven. Self bootstrapping.**

## What is Chant?

Chant is a spec execution platform for AI-assisted work. Write specs in markdown. Agents execute them.

Specs can drive:
- **Code** — Implementation, configuration, infrastructure
- **Documentation** — Docs that track the code they describe
- **Research** — Analysis, synthesis, experiments

## Self Bootstrapping

Chant built itself.

The first commit was a Claude Code skill that read specs and executed them. Every feature after that was a spec the skill implemented. The binary is convenience. The model is the product.

```
skills/bootstrap/SKILL.md     # The skill that built chant
.chant/specs/                 # Specs it executed
```

The same skill works for any project. The same prompts work for chant and customers. This isn't bootstrapping as a one-time trick—it's proof that the model works.

### The Bootstrap Pattern

```
┌─────────────────────────────────────────────────────────┐
│  1. Write a spec                                        │
│     .chant/specs/001-add-feature.md                     │
├─────────────────────────────────────────────────────────┤
│  2. Execute it                                          │
│     /bootstrap 001  (skill)                             │
│     chant work 001  (binary)                            │
├─────────────────────────────────────────────────────────┤
│  3. Spec status updates, work commits                   │
│     status: completed                                   │
│     commit: abc123                                      │
└─────────────────────────────────────────────────────────┘
```

Same flow whether you use the skill or the binary. The binary adds:
- Better CLI ergonomics
- Parallel execution
- Search and indexing
- Daemon mode

But the model—specs in, work out—is identical.

### For Customers

Customers can bootstrap their projects the same way:

1. Copy the skill and prompts
2. Write specs
3. Run `/bootstrap`
4. Install binary when they want better UX

No binary required to validate the model works for their project.

## Intent-First Development

Specifications are the source of truth.

| Approach | Source of Truth | Problem |
|----------|-----------------|---------|
| Documentation-first | Docs | Rots as code changes |
| Code-first | Code | Intent buried in implementation |
| Ticket-first | Tickets | Closed and forgotten |
| **Intent-first** | Specs | Execute, verify, persist |

In intent-first development:

- Specs are executable, not just readable
- Completion means "verified", not "closed"
- Drift from intent is detected, not ignored
- Replay restores intent without manual work

The spec IS the work.

## Core Principles

### Markdown IS the UI

Specs are markdown files with YAML frontmatter. No special viewer needed.

```markdown
# .chant/specs/2026-01-22-001-x7m.md
---
status: pending
depends_on: []
---

# Add authentication

## Acceptance Criteria
- [ ] JWT tokens work
- [ ] 401 on invalid token
```

Filename is the ID.

- Edit in any editor (VS Code, vim, GitHub web UI)
- Git diffs are the changelog
- PRs are the review interface
- Search with `grep`

### Index is Optimization

The index makes queries fast. Delete and rebuild from markdown anytime.

```
.chant/
  specs/*.md      ← Source of truth (git-tracked)
  .store/         ← Derived index (gitignored)
```

### Prompts are Configuration

Agent behavior defined in markdown, not code.

```
.chant/prompts/
  standard.md     ← Default execution
  minimal.md      ← Quick fixes
  tdd.md          ← Test-driven
  split.md        ← Decompose specs
```

Customize to match your workflow.

### Prompts are Universal

The same prompts work for:
- Chant building itself
- Customers building their projects
- Code, documentation, and research specs

This universality is why self-bootstrapping works. The prompts don't know they're building chant—they just implement specs.

### Unix Philosophy

Do one thing well. Compose with standard tools.

- **Is:** Spec tracking + execution discipline
- **Is not:** Project management, visualization, reporting
- **Uses:** Git for sync, editors for viewing, grep for search
