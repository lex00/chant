# Prompt Guide

This guide covers prompt authoring, examples, and advanced techniques.

## Why Prompts Matter

Prompts are the core of Chant. Everything else is infrastructure.

```
Good prompt + mediocre spec = Good result
Bad prompt + perfect spec = Bad result
```

## Anatomy of a Prompt

```markdown
<!-- .chant/prompts/standard.md -->
---
name: standard
purpose: Default prompt for general tasks
---

# Role

You are a senior developer implementing a spec.

# Context

{{spec.description}}

# Instructions

1. Read relevant code first
2. Make minimal changes
3. Verify before committing

# Constraints

- Don't modify unrelated files
- Don't add dependencies without asking
- Keep changes focused

# Output

Commit your changes with message: chant({{spec.id}}): {{spec.title}}
```

## The Five Sections

Every effective prompt has these sections:

### 1. Role

Who is the agent? This sets behavior and expertise level.

```markdown
# Role

You are a senior developer with 10 years of experience.
You write clean, tested, maintainable code.
You prefer simplicity over cleverness.
```

### 2. Context

What does the agent need to know? Injected from spec.

```markdown
# Context

## Spec
{{spec.description}}

## Target Files
{{spec.target_files}}
```

### 3. Instructions

What should the agent do? Step-by-step process.

```markdown
# Instructions

## Phase 1: Understand
1. Read the target files
2. Understand the existing patterns

## Phase 2: Implement
3. Make changes
4. Follow existing code style

## Phase 3: Verify
5. Run tests
6. Check for lint errors

## Phase 4: Commit
7. Commit: git commit -m "chant({{spec.id}}): {{spec.title}}"
```

### 4. Constraints

What should the agent NOT do? Boundaries and guardrails.

```markdown
# Constraints

- Only modify files related to the spec
- Don't refactor unrelated code
- Don't add new dependencies without approval
- Don't commit secrets
```

### 5. Output

What does "done" look like?

```markdown
# Output

When complete:
1. All acceptance criteria met
2. Tests passing
3. Changes committed with proper message format

If you cannot complete:
1. Commit any partial progress
2. Document what's blocking
3. Exit with error
```

## Template Variables

Prompts use `{{variable}}` syntax:

| Variable | Description |
|----------|-------------|
| `{{project.name}}` | Project name |
| `{{spec.id}}` | Spec identifier |
| `{{spec.title}}` | First heading |
| `{{spec.description}}` | Full spec body |
| `{{spec.target_files}}` | Target files list |
| `{{worktree.path}}` | Worktree path (parallel) |
| `{{worktree.branch}}` | Branch name (parallel) |

## Prompt Inheritance

Prompts can extend other prompts:

```yaml
---
name: tdd
extends: standard
---

{{> parent}}

## TDD Requirements

Write tests before implementation.
```

The `{{> parent}}` marker indicates where the parent prompt content should be injected.

---

## Built-in Prompts

### standard.md

The default. Balanced approach for most tasks.

```markdown
---
name: standard
purpose: Default execution prompt
---

# Execute Spec

You are implementing a spec for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Instructions

1. **Read** the relevant code first
2. **Plan** your approach before coding
3. **Implement** the changes
4. **Verify** each acceptance criterion
5. **Commit** with message: `chant({{spec.id}}): <description>`

## Constraints

- Only modify files related to this spec
- Follow existing code patterns
- Do not refactor unrelated code
```

### minimal.md

Quick fixes, less ceremony.

```markdown
---
name: minimal
purpose: Quick fixes with minimal overhead
---

# Quick Fix

{{spec.description}}

Make the change. Commit as `chant({{spec.id}}): <description>`.

Keep it simple. Don't over-engineer.
```

### split.md

Break down large specs into members.

```markdown
---
name: split
purpose: Break down specs into smaller pieces
---

# Split Spec

You are breaking down a spec into smaller, executable pieces.

## Driver Spec

**{{spec.title}}**

{{spec.description}}

## Instructions

1. Analyze the spec scope
2. Identify independent pieces of work
3. Create member specs that are:
   - Small enough to complete in one session
   - Clear acceptance criteria
   - Specific target files
```

### tdd

Test-driven development workflow.

```markdown
---
name: tdd
extends: standard
---

{{> parent}}

## TDD Cycle

### 1. RED - Write Failing Test
Write a test that fails because the feature doesn't exist.

### 2. GREEN - Make It Pass
Write minimum code to make the test pass.

### 3. REFACTOR - Clean Up
Improve code quality while keeping tests passing.
```

---

## Prompt Patterns

### The Loop

The fundamental execution pattern:

```markdown
Repeat until done:
1. **Read** - Understand current state
2. **Plan** - Decide next action
3. **Change** - Make one change
4. **Verify** - Check it works
5. **Commit** - Save progress
```

### Fail Fast

```markdown
If you encounter a blocker:
1. Stop immediately
2. Commit any safe partial work
3. Document the blocker
4. Mark spec as failed

Don't guess. Don't work around. Fail clearly.
```

### Minimal Change

```markdown
Make the smallest change that satisfies the spec.

- Don't refactor adjacent code
- Don't fix unrelated issues
- Don't add "nice to have" features
```

---

## Common Mistakes

### Too Vague

```markdown
# Bad
Do the spec well.

# Good
1. Read the target files
2. Implement the change
3. Write tests
4. Run existing tests
5. Commit with format: chant(ID): description
```

### Too Rigid

```markdown
# Bad
Always use exactly 4 spaces for indentation.

# Good
Follow the existing code style in the file.
```

### No Failure Path

```markdown
# Bad
Complete the spec.

# Good
Complete the spec.

If you cannot complete:
1. Commit partial progress
2. Document what's blocking
3. Exit with non-zero status
```

### Scope Creep Invitation

```markdown
# Bad
Improve the code quality while you're there.

# Good
Only modify code directly related to the spec.
Note other improvements as potential follow-up specs.
```

---

## Prompt Library Organization

```
.chant/prompts/
├── standard.md          # Default for most tasks
├── tdd.md               # Test-driven development
├── security.md          # Security-sensitive changes
├── docs.md              # Documentation tasks
└── domain/
    ├── api.md           # API-specific
    └── frontend.md      # UI changes
```

---

## Checklist: Is Your Prompt Ready?

- [ ] **Role** defined
- [ ] **Context** injection (spec body, target files)
- [ ] **Instructions** numbered and phased
- [ ] **Verification** step before completion
- [ ] **Constraints** on scope and behavior
- [ ] **Failure handling** documented
- [ ] **Commit format** specified
