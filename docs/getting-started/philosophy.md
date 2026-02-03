# Philosophy

**Intent based. Spec driven. Self bootstrapping.**

## What is Chant?

Chant is a spec execution platform for AI-assisted work. Write specs in markdown. Agents execute them.

Specs can drive:
- **Code** — Implementation, configuration, infrastructure
- **Documentation** — Docs that track the code they describe
- **Research** — Analysis, synthesis, experiments

## The Problem

Specifications don't drive themselves:

- Docs describe intent but can't enforce it
- Tickets track work but forget after closure
- Code comments explain but don't verify
- AI agents execute but don't persist

**The result:** Intent exists for a moment, then decays.

AI coding agents are powerful but stateless. Each session starts fresh:

- No memory of previous work
- No awareness of what's done vs pending
- No coordination between sessions
- No discipline enforced

Developers end up:

- Re-explaining context every session
- Manually tracking what's done
- Copy-pasting between sessions
- Getting inconsistent results

## Self-Driving Specs

Specs that:

- **Execute** — Agent invocation
- **Verify** — Continuous checking
- **Detect drift** — When reality diverges
- **Replay** — Restore intent automatically

See [how it works →](../concepts/autonomy.md)

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
- Search and filtering

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

## Core Value

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

**Value**: Zero friction to view, edit, understand.

### Index is Optimization

The index makes queries fast. Delete and rebuild from markdown anytime.

```
.chant/
  specs/*.md      ← Source of truth (git-tracked)
  .store/         ← Derived index (gitignored)
```

### Specs Drive Agents

Each spec is complete:

- Title (what)
- Description (context)
- Acceptance criteria (done when)
- Target files (where)

Agent prompt: "Implement this spec."

**Value**: Consistent, reproducible agent behavior.

### Parallel Execution

Split a spec into a group, execute members in parallel:

```bash
chant split 001
chant work 001 --parallel
```

Each agent in isolated worktree. No conflicts.

**Value**: Faster completion of complex work.

### Git-Native

- Branches per spec (optional)
- Commits tracked in frontmatter
- PRs created automatically (optional)

**Value**: Fits existing workflow.

### Crash Recovery

PID locks track running agents. Stale locks detected.

```bash
$ chant work 001
Warning: Stale lock from crashed session
Recover and continue? [y/N]
```

**Value**: Resilient to failures.

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

## Core Principles

### Unix Philosophy

Do one thing well. Compose with standard tools.

- **Is:** Spec tracking + execution discipline
- **Is not:** Project management, visualization, reporting
- **Uses:** Git for sync, editors for viewing, grep for search

## Who It's For

### Solo Developers / Small Teams

- Persistent spec tracking
- Execution discipline
- Simple tooling

### Documentation Maintainers

The problem: Docs rot as code changes.

With Chant:
- **Doc specs have origin** — `origin:` field links docs to source code
- **Drift detection** — Know when code changed since docs were written
- **Replay** — Re-run spec to update docs automatically

```yaml
---
type: documentation
origin: [src/auth/*.go]
target_files: [docs/auth.md]
---
```

### Researchers

The problem: Analysis goes stale when data changes.

With Chant:
- **Research specs have origin** — Analysis linked to data files
- **Reproducibility** — Every analysis step recorded
- **Provenance** — Know exactly what data produced what findings

```yaml
---
type: research
origin: [data/survey.csv]
target_files: [findings/analysis.md]
---
```

### Enterprise Developers (Silent Mode)

Working in rigid environments:

- Personal AI workflow without changing shared repo
- `chant init --silent` keeps `.chant/` local only
- No trace in official codebase

### Enterprise Teams

- **Derived frontmatter** — Auto-populate fields from branch/path patterns
- Integration with existing conventions

```yaml
# config.yaml
enterprise:
  derived:
    sprint:
      from: branch
      pattern: "sprint/(\\d{4}-Q\\d-W\\d)"
    team:
      from: path
      pattern: "teams/(\\w+)/"
```

**Not for:**

- Non-technical users
- Teams wanting GUI/dashboards

## What Chant Replaces

| Before | After |
|--------|-------|
| Mental tracking | `.chant/specs/` |
| Copy-paste context | Spec file IS the context |
| Manual branch management | `branch: true` in config |
| Hope for the best | Acceptance criteria + linting |
| Lost work on crash | PID locks + recovery |

## The Pitch

> Specs are markdown files. Agents execute them.
> Everything is git-tracked. Nothing is lost.
>
> `chant add "Fix the bug"` → `chant work` → done.
