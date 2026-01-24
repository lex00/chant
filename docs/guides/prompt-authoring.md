# Prompt Authoring Guide

## Why This Matters

Prompts are the core of Chant. Everything else is infrastructure.

```
Good prompt + mediocre spec = Good result
Bad prompt + perfect spec = Bad result
```

This guide teaches you to write prompts that produce reliable, autonomous agent behavior.

## Anatomy of a Prompt

```markdown
<!-- .chant/prompts/standard.md -->
---
name: standard
description: Default prompt for general tasks
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

Every effective prompt has these sections (explicitly or implicitly):

### 1. Role

Who is the agent? This sets behavior and expertise level.

```markdown
# Role

You are a senior developer with 10 years of experience.
You write clean, tested, maintainable code.
You prefer simplicity over cleverness.
```

**Tips:**
- Be specific about expertise level
- Include values (simplicity, testing, etc.)
- Avoid generic "you are an AI assistant"

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

## Acceptance Criteria
{{#if spec.criteria}}
{{spec.criteria}}
{{/if}}
```

**Tips:**
- Inject spec body, not just title
- Include target files if specified
- Include acceptance criteria for verification

### 3. Instructions

What should the agent do? Step-by-step process.

```markdown
# Instructions

## Phase 1: Understand
1. Read the target files
2. Understand the existing patterns
3. Identify what needs to change

## Phase 2: Plan
4. Decide on approach
5. List files you'll modify

## Phase 3: Implement
6. Make changes
7. Follow existing code style
8. Add tests if appropriate

## Phase 4: Verify
9. Run tests: {{test_command}}
10. Check for lint errors
11. Review your changes

## Phase 5: Commit
12. Stage changes: git add <files>
13. Commit: git commit -m "chant({{spec.id}}): {{spec.title}}"
```

**Tips:**
- Number the steps
- Group into phases
- Be explicit about verification
- Include the exact commit format

### 4. Constraints

What should the agent NOT do? Boundaries and guardrails.

```markdown
# Constraints

## Scope
- Only modify files related to the spec
- Don't refactor unrelated code
- Don't "improve" things not asked for

## Dependencies
- Don't add new dependencies without explicit approval
- Don't upgrade existing dependencies

## Style
- Follow existing code patterns
- Don't change formatting of untouched code
- Match the project's naming conventions

## Safety
- Don't delete files unless explicitly asked
- Don't modify .env or credentials
- Don't commit secrets
```

**Tips:**
- Be explicit about scope limits
- Call out common mistakes
- Include safety constraints

### 5. Output

What does "done" look like? Expected deliverable.

```markdown
# Output

When complete:
1. All acceptance criteria met
2. Tests passing
3. No lint errors
4. Changes committed with proper message format

Commit message format:
  chant({{spec.id}}): <brief description>

If you cannot complete the spec:
1. Commit any partial progress
2. Document what's blocking in the spec file
3. Exit with error
```

**Tips:**
- Define success criteria
- Specify commit message format
- Explain what to do on failure

## Prompt Patterns

### Pattern: The Loop

The fundamental execution pattern:

```markdown
# Instructions

Repeat until done:
1. **Read** - Understand current state
2. **Plan** - Decide next action
3. **Change** - Make one change
4. **Verify** - Check it works
5. **Commit** - Save progress

Don't make multiple changes without verification.
```

### Pattern: Checkpoint Commits

For long specs:

```markdown
# Commit Strategy

Commit frequently:
- After each logical unit of work
- Before risky changes
- Every 10-15 minutes of work

Checkpoint commits:
  git commit -m "chant({{spec.id}}): checkpoint - <what's done>"

Final commit will be squashed if configured.
```

### Pattern: Fail Fast

For reliability:

```markdown
# Error Handling

If you encounter a blocker:
1. Stop immediately
2. Commit any safe partial work
3. Document the blocker:
   - What you tried
   - What failed
   - What you think is needed
4. Mark spec as failed

Don't guess. Don't work around. Fail clearly.
```

### Pattern: Minimal Change

For safety:

```markdown
# Change Philosophy

Make the smallest change that satisfies the spec.

- Don't refactor adjacent code
- Don't fix unrelated issues (create new specs)
- Don't add "nice to have" features
- Don't change formatting of untouched lines

If you see something worth fixing, note it but don't fix it.
```

### Pattern: Test First

For TDD workflows:

```markdown
# Test-Driven Development

1. Write failing test for the requirement
2. Run test, confirm it fails
3. Implement minimum code to pass
4. Run test, confirm it passes
5. Refactor if needed
6. Repeat for next requirement

Never implement without a failing test first.
```

### Pattern: Self-Verification

For autonomous execution:

```markdown
# Before Completing

Run this checklist:
- [ ] All acceptance criteria met
- [ ] Tests pass: {{test_command}}
- [ ] No lint errors: {{lint_command}}
- [ ] No untracked files: git status
- [ ] Commit message follows format
- [ ] No TODOs left in code (unless intentional)

If any check fails, fix before completing.
```

## Writing for Autonomy

Autonomous prompts need extra clarity because there's no human to ask.

### Be Explicit About Decisions

```markdown
# Decision Making

When facing ambiguity:

## Code Style
- Follow existing patterns in the file
- If no pattern exists, use project defaults
- If no project defaults, use language idioms

## Approach Selection
- Prefer the simpler approach
- Prefer fewer dependencies
- Prefer explicit over clever

## Edge Cases
- Handle obvious edge cases (null, empty, etc.)
- Add TODO comments for non-obvious edge cases
- Don't gold-plate edge case handling
```

### Define "Done" Precisely

```markdown
# Definition of Done

A spec is complete when:
1. All acceptance criteria checkboxes can be checked
2. Tests pass (existing + new)
3. No lint/type errors
4. Code is committed
5. No untracked files

A spec is NOT complete if:
- "It works for the happy path"
- "It just needs a few more tests"
- "It compiles"
```

### Handle Unknowns

```markdown
# When Stuck

If you don't know how to proceed:

1. **Missing information**: Check if spec description has it.
   If not, make reasonable assumption and document it.

2. **Technical blocker**: Try 2-3 approaches.
   If all fail, document what you tried and fail the spec.

3. **Scope unclear**: Implement the minimal interpretation.
   Note what you excluded and why.

Don't spin. Try, document, move on.
```

## Prompt Variables

### Defining Variables

```yaml
---
name: my-prompt
variables:
  - name: test_command
    description: Command to run tests
    default: "npm test"

  - name: lint_command
    description: Command to run linter
    default: "npm run lint"

  - name: language
    description: Primary programming language
    required: true
---
```

### Using Variables

```markdown
# Verification

Run tests: {{test_command}}
Run linter: {{lint_command}}

{{#if language == "go"}}
Also run: go vet ./...
{{/if}}
```

### Spec-Provided Variables

Specs can override prompt variables:

```yaml
# Spec frontmatter
---
status: pending
prompt: my-prompt
prompt_vars:
  test_command: "pytest"
  lint_command: "ruff check ."
---
```

## Prompt Inheritance

### Extending Prompts

```yaml
---
name: tdd
extends: standard
---

# Additional Instructions

{{> parent}}

## TDD Requirements

Write tests before implementation.
```

### Override vs Extend

```markdown
<!-- Extend: add to parent -->
{{> parent}}
Plus these additional instructions...

<!-- Override: replace parent section -->
# Instructions
Completely different instructions...
```

### Inheritance Chain

```
base
  └── standard
        ├── tdd
        ├── security
        └── refactor
```

## Testing Prompts

### Dry Run

```bash
# See assembled prompt without executing
chant work 001 --prompt my-prompt --dry-run
```

### Prompt Preview

```bash
# Preview prompt with spec variables filled
chant prompt preview my-prompt --spec 001
```

### Prompt Validation

```bash
# Check prompt for common issues
chant prompt lint my-prompt

Issues found:
  ⚠ No verification step
  ⚠ No failure handling
  💡 Consider adding acceptance criteria check
```

### A/B Testing

```bash
# Compare prompt effectiveness
chant work --search "label:test" --prompt standard
chant work --search "label:test" --prompt improved

chant report compare --prompts standard,improved
```

## Common Mistakes

### Too Vague

```markdown
# Bad
Do the spec well.

# Good
1. Read the target files
2. Implement the change described in the spec
3. Write tests for new functionality
4. Run existing tests to check for regressions
5. Commit with message format: chant(ID): description
```

### Too Rigid

```markdown
# Bad
Always use exactly 4 spaces for indentation.
Always add JSDoc comments to every function.
Always create a test file named X_test.go.

# Good
Follow the existing code style in the file.
Add documentation for public APIs.
Add tests following the project's testing conventions.
```

### No Failure Path

```markdown
# Bad
Complete the spec.

# Good
Complete the spec.

If you cannot complete:
1. Commit any partial progress with "checkpoint" message
2. Document what's blocking
3. Exit with non-zero status
```

### Assuming Context

```markdown
# Bad
Use the standard testing framework.

# Good
Run tests with: {{test_command}}
If no test command is configured, look for:
- package.json scripts (npm test)
- Makefile targets (make test)
- Common patterns (pytest, go test, cargo test)
```

### Scope Creep Invitation

```markdown
# Bad
Improve the code quality while you're there.

# Good
Only modify code directly related to the spec.
Note other improvements as potential follow-up specs.
Don't fix unrelated issues in this spec.
```

## Prompt Evolution

### Level 1: Basic

```markdown
Do the spec. Commit when done.
```

### Level 2: Structured

```markdown
# Spec
{{spec.body}}

# Instructions
1. Read relevant code
2. Make changes
3. Run tests
4. Commit

# Output
Commit message: chant({{spec.id}}): {{spec.title}}
```

### Level 3: Robust

```markdown
# Role
You are a senior developer.

# Context
{{spec.body}}

# Instructions
1. Read target files
2. Understand existing patterns
3. Plan minimal changes
4. Implement
5. Verify: tests, lint, type check
6. Commit with proper message

# Constraints
- Minimal changes only
- Follow existing patterns
- No new dependencies

# On Failure
Document what's blocking, commit partial work, exit with error.
```

### Level 4: Autonomous

All of Level 3, plus:
- Decision framework for ambiguity
- Checkpoint strategy
- Self-verification checklist
- Clear definition of done
- Edge case handling guidance

## Prompt Library Design

### Organization

```
.chant/prompts/
├── base.md              # Foundation (rarely used directly)
├── standard.md          # Default for most tasks
├── tdd.md               # Test-driven development
├── security.md          # Security-sensitive changes
├── refactor.md          # Refactoring tasks
├── docs.md              # Documentation tasks
├── review.md            # Code review tasks
└── domain/
    ├── api.md           # API-specific
    ├── database.md      # Database changes
    └── frontend.md      # UI changes
```

### Choosing Prompts

```yaml
# config.md
prompts:
  default: standard

  by_label:
    security: security
    refactor: refactor
    docs: docs
    tdd: tdd

  by_path:
    "src/api/**": domain/api
    "src/db/**": domain/database
    "src/ui/**": domain/frontend
```

## Checklist: Is Your Prompt Ready?

- [ ] **Role** defined (who is the agent?)
- [ ] **Context** injection (spec body, target files, criteria)
- [ ] **Instructions** numbered and phased
- [ ] **Verification** step before completion
- [ ] **Constraints** on scope and behavior
- [ ] **Failure handling** documented
- [ ] **Commit format** specified
- [ ] **Variables** have sensible defaults
- [ ] **Tested** with dry-run on sample tasks
- [ ] **Linted** for common issues

## Reference: Built-in Prompts

| Prompt | Use Case | Key Features |
|--------|----------|--------------|
| `base` | Foundation | Minimal, for extending |
| `standard` | General tasks | Balanced constraints |
| `tdd` | Test-driven | Test-first workflow |
| `security` | Sensitive code | Extra verification |
| `refactor` | Code cleanup | No behavior changes |
| `docs` | Documentation | Markdown focus |
| `review` | Code review | Read-only analysis |
| `autonomous` | Unattended | Self-sufficient decisions |
| `split` | Spec breakdown | Creates member specs |

See [prompt-examples.md](prompt-examples.md) for full examples.
