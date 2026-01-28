# Prompt Guide

This comprehensive guide covers prompt authoring, examples, and advanced techniques.

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
extends: base                    # Optional parent
variables:                       # Optional variables
  - name: test_command
    default: "npm test"
---

# Role

You are a senior developer implementing a spec.

# Context

{{spec.body}}

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
{{spec.body}}

## Target Files
{{#if spec.target_files}}
Focus on these files:
{{#each spec.target_files}}
- {{this}}
{{/each}}
{{/if}}
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
5. Run tests: {{test_command}}
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

Prompts use `{{variable}}` syntax. See [concepts/prompts.md](../concepts/prompts.md#template-variables) for the complete variable reference.

Common variables:

| Variable | Description |
|----------|-------------|
| `{{project.name}}` | Project name |
| `{{spec.id}}` | Spec identifier |
| `{{spec.title}}` | First heading |
| `{{spec.description}}` | Full spec body |
| `{{spec.target_files}}` | Target files list |
| `{{spec.acceptance}}` | Acceptance criteria |

### Custom Variables

Define custom variables in frontmatter:

```yaml
---
name: my-prompt
variables:
  - name: test_command
    default: "npm test"
  - name: language
    required: true
---
```

Specs can override:

```yaml
# Spec frontmatter
---
prompt: my-prompt
prompt_vars:
  test_command: "pytest"
---
```

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

## Prompt Inheritance

### Extending Prompts

```yaml
---
name: tdd
extends: standard
---

{{> parent}}

## TDD Requirements

Write tests before implementation.
```

### Multiple Parents

```yaml
---
name: secure-tdd
extends: [tdd, security-review]
---

{{> tdd}}

---

{{> security-review}}
```

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

{{spec.body}}

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

{{spec.body}}

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

{{spec.body}}

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
purpose: Test-first development workflow
---

# Test-Driven Development

## TDD Cycle

### 1. RED - Write Failing Test
Write a test that fails because the feature doesn't exist.

### 2. GREEN - Make It Pass
Write minimum code to make the test pass.

### 3. REFACTOR - Clean Up
Improve code quality while keeping tests passing.
```

### security-review

Security-focused code review.

```markdown
---
name: security-review
purpose: Security-focused code review
---

# Security Review

## Checklist

### Input Validation
- [ ] All user input validated
- [ ] Input length limits enforced

### Authentication & Authorization
- [ ] Authentication required where needed
- [ ] Authorization checks on resources

### Injection Prevention
- [ ] SQL injection prevented
- [ ] XSS prevented
- [ ] Command injection prevented
```

### documentation

Generate or update documentation.

```markdown
---
name: documentation
purpose: Generate or update documentation
---

# Documentation Spec

## Guidelines

- Clear and concise
- Active voice preferred
- Every concept needs an example
- Examples should be copy-pasteable
```

---

## Language-Specific Prompts

### rust-idioms

```markdown
---
name: rust-idioms
extends: standard
---

## Rust Guidelines

### Error Handling
- Use `Result<T, E>` for recoverable errors
- Use `?` operator for propagation

### Ownership
- Prefer borrowing over cloning
- Use `&str` over `String` in parameters

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
```
```

### react-components

```markdown
---
name: react-components
extends: standard
---

## React Guidelines

- Functional components with hooks
- Props interface named `{Component}Props`
- Test behavior, not implementation

```bash
npm run lint && npm run typecheck && npm run test
```
```

---

## Advanced Techniques

### Specification Discovery

Use prompts to refine vague specifications.

#### spec-critique

Reviews a draft spec and identifies gaps:

```markdown
# .chant/prompts/spec-critique.md

You are a specification reviewer. Analyze the spec and identify:

## Gaps to Address

1. **Ambiguity** - What decisions are left unstated?
2. **Scope** - Is this bounded or open-ended?
3. **Verification** - How will we know it's done?
4. **Dependencies** - What must exist first?
5. **Risks** - What could go wrong?

## Complexity Assessment

- [ ] Quick (1-2 files)
- [ ] Standard (3-5 files)
- [ ] Complex (6+ files, architectural impact)

If Complex, recommend decomposition.
```

Usage:
```bash
chant work 001 --prompt spec-critique --dry-run
```

#### spec-expand

Expands a brief idea into a full specification:

```markdown
# .chant/prompts/spec-expand.md

Given a brief idea, produce a complete spec.

## Process

1. **Understand** - What is the user trying to achieve?
2. **Research** - Read relevant existing code
3. **Specify** - Write detailed requirements
4. **Bound** - Define what's NOT in scope
5. **Verify** - Write acceptance criteria
```

### Capturing Learnings

#### learnings

Analyzes completed work and captures reusable knowledge:

```markdown
# .chant/prompts/learnings.md

After work completes, analyze what was built and capture learnings.

## What to Capture

### New Patterns
- Name the pattern
- Show a minimal example
- Explain when to use it

### Gotchas Discovered
- What went wrong initially
- What the fix was
- How to avoid it next time
```

### Retrospective Analysis

#### retro

Analyzes recent work for patterns:

```markdown
# .chant/prompts/retro.md

Analyze completed work to identify patterns and improvements.

## Analysis

### Efficiency Metrics
- **Completion rate**: completed / (completed + failed)
- **Retry rate**: specs requiring multiple attempts

### Failure Patterns
Group failures by cause: test failures, merge conflicts, timeout, scope creep

### Recommendations
1. Process improvements
2. Prompt improvements
3. Spec quality guidance
```

---

## Workflow Composition

Build workflows from composable prompts:

| Block | Purpose | Prompt |
|-------|---------|--------|
| Ideation | Capture rough ideas | `capture` |
| Specification | Refine into specs | `spec-expand`, `spec-critique` |
| Decomposition | Break into subspecs | `split` |
| Implementation | Do the work | `standard`, `tdd` |
| Review | Check quality | `review` |
| Learning | Capture insights | `learnings` |

### Composing Prompts

```yaml
# config.md
prompts:
  feature:
    compose:
      - pattern-match    # Check learnings
      - standard         # Implement
      - self-review      # Check own work
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

## Testing Prompts

### Dry Run

```bash
chant work 001 --prompt my-prompt --dry-run
```

### Prompt Preview

```bash
chant prompt preview my-prompt --spec 001
```

### Prompt Validation

```bash
chant prompt lint my-prompt
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

### Choosing Prompts by Context

```yaml
# config.md
prompts:
  default: standard

  by_label:
    security: security
    refactor: refactor
    tdd: tdd

  by_path:
    "src/api/**": domain/api
    "src/ui/**": domain/frontend
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
- [ ] **Variables** have sensible defaults
- [ ] **Tested** with dry-run
