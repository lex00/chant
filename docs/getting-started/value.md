# Value Proposition

## The Problem

Specifications don't drive themselves:

- Docs describe intent but can't enforce it
- Tickets track work but forget after closure
- Code comments explain but don't verify
- AI agents execute but don't persist

**The result:** Intent exists for a moment, then decays.

## Self-Driving Specs

Specs that:

- **Execute** — Agent invocation
- **Verify** — Continuous checking
- **Detect drift** — When reality diverges
- **Replay** — Restore intent automatically

See [how it works →](../concepts/autonomy.md)

## The Deeper Problem

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

## What Chant Provides

- **Markdown-native** — Human-readable, any editor
- **Spec tracking** — With dependencies
- **Agent execution** — Built-in
- **Git-native** — Branches, commits, PRs
- **Simple CLI** — Minimal ceremony

## Core Value

### Markdown IS the UI

Open `.chant/specs/` in any editor.

- Git diffs show exactly what changed
- PRs include spec changes alongside code
- Search with `grep`

**Value**: Zero friction to view, edit, understand.

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
