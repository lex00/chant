---
name: chant
description: Manage development specs with chant — a CLI tool for orchestrating
  AI-assisted development through structured specs. Use when working with specs,
  acceptance criteria, the chant workflow, or when the user mentions chant commands.
metadata:
  author: chant
  version: "1.0"
---

## Chant Workflow

Chant organizes work into **specs** — markdown files in `.chant/specs/` that describe tasks with acceptance criteria. Specs flow through a lifecycle: `pending` → `in_progress` → `completed` (or `failed`).

### Core Commands

| Command | Purpose |
|---------|---------|
| `chant add "description"` | Create a new spec |
| `chant list` | List specs by status |
| `chant show <id>` | View spec details |
| `chant work <id>` | Execute a spec (spawns agent in worktree) |
| `chant work --chain` | Execute ready specs sequentially |
| `chant verify <id>` | Verify acceptance criteria are met |
| `chant merge <id>` | Merge completed spec branch to main |

### Spec Structure

Specs are markdown files with YAML frontmatter:

```markdown
---
title: Fix login timeout
status: pending
type: code
---

# Fix login timeout

Users report timeouts after 30 seconds on the login page.

## Acceptance Criteria

- [ ] Timeout increased to 60 seconds
- [ ] Error message shown on timeout
- [ ] Test added for timeout behavior
```

### Working with Specs

1. **Create** a spec describing the task
2. **Work** the spec — chant spawns an agent in an isolated git worktree
3. **Verify** acceptance criteria are met
4. **Merge** the spec branch back to main

### Acceptance Criteria

Each spec should have clear, checkable acceptance criteria. The agent checks these off (`- [x]`) as work completes. Chant validates all criteria are checked before finalizing.

### Commit Convention

Commits from spec work use the format:
```
chant(<spec-id>): <description>
```

### Branch Isolation

Each spec executes in its own git worktree, keeping the main branch clean. Completed work is merged back via `chant merge`.
