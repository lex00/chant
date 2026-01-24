# Prompt Examples

## Built-in Prompts

These ship with `chant init`.

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

## Acceptance Criteria

{{#each spec.acceptance}}
- [ ] {{this}}
{{/each}}

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
- Do not add features beyond what's specified
```

### minimal.md

Quick fixes, small changes. Less ceremony.

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

Split a large spec into group members.

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
   - Independent (can run in parallel if no dependencies)
   - Clear acceptance criteria
   - Specific target files

## Output Format

Create member spec files:

```
.chant/specs/{{spec.id}}.1.md
.chant/specs/{{spec.id}}.2.md
...
```

Each member:

```markdown
---
status: pending
{{#if has_dependency}}
depends_on:
  - {{dependency}}
{{/if}}
---

# [Specific subtask title]

## Context
[Why this piece exists]

## Acceptance Criteria
- [ ] [Specific, verifiable criterion]

## Target Files
- [file1]
- [file2]
```

Do not implement. Only create the spec breakdown.
```

---

## Community Prompts

Install with `chant prompt add <name>`.

### tdd (Test-Driven Development)

```markdown
---
name: tdd
purpose: Test-first development workflow
version: 1.0.0
author: chant-prompts
tags: [workflow, testing]
models:
  recommended: [large-model, capable-model]
---

# Test-Driven Development

You are implementing a spec using TDD.

## Your Spec

{{spec.body}}

## TDD Cycle

### 1. RED - Write Failing Test

First, write a test that:
- Describes the expected behavior
- Fails because the feature doesn't exist
- Is minimal and focused

```
# Run tests - should FAIL
{{project.test_command}}
```

Commit: `chant({{spec.id}}): red - add failing test for <feature>`

### 2. GREEN - Make It Pass

Write the minimum code to make the test pass:
- No extra features
- No premature optimization
- Just make the test green

```
# Run tests - should PASS
{{project.test_command}}
```

Commit: `chant({{spec.id}}): green - implement <feature>`

### 3. REFACTOR - Clean Up

Now improve the code:
- Remove duplication
- Improve naming
- Simplify logic
- Keep tests passing

```
# Run tests - should still PASS
{{project.test_command}}
```

Commit: `chant({{spec.id}}): refactor - clean up <feature>`

## Repeat

If more functionality needed, repeat RED → GREEN → REFACTOR.

## Final Check

- [ ] All tests pass
- [ ] No skipped tests
- [ ] Coverage maintained or improved
```

### security-review

```markdown
---
name: security-review
purpose: Security-focused code review
version: 2.1.0
author: chant-prompts
tags: [security, review]
models:
  recommended: [high-capability]
  note: Needs strong reasoning for security analysis
---

# Security Review

You are reviewing code for security vulnerabilities.

## Scope

{{spec.body}}

## Checklist

### Input Validation
- [ ] All user input validated
- [ ] Input length limits enforced
- [ ] Special characters handled
- [ ] File uploads validated (type, size)

### Authentication & Authorization
- [ ] Authentication required where needed
- [ ] Authorization checks on resources
- [ ] Session handling secure
- [ ] Password handling follows best practices

### Data Protection
- [ ] Sensitive data encrypted at rest
- [ ] Sensitive data encrypted in transit
- [ ] PII handled appropriately
- [ ] Secrets not hardcoded

### Injection Prevention
- [ ] SQL injection prevented (parameterized queries)
- [ ] XSS prevented (output encoding)
- [ ] Command injection prevented
- [ ] Path traversal prevented

### Error Handling
- [ ] Errors don't leak sensitive info
- [ ] Stack traces not exposed to users
- [ ] Logging doesn't include secrets

## Output

Create a security report in the spec file:

```markdown
## Security Review Results

### Critical Issues
[List any critical vulnerabilities]

### High Priority
[List high priority issues]

### Medium Priority
[List medium priority issues]

### Low Priority / Recommendations
[List suggestions]

### Passed Checks
[List what looks good]
```

If critical issues found, mark spec as failed with details.
```

### documentation

```markdown
---
name: documentation
purpose: Generate or update documentation
version: 1.2.0
author: chant-prompts
tags: [docs, writing]
---

# Documentation Spec

You are writing or updating documentation.

## Scope

{{spec.body}}

## Guidelines

### Voice & Tone
- Clear and concise
- Active voice preferred
- Second person ("you") for instructions
- Present tense

### Structure
- Start with the "why" before the "how"
- Use headings liberally
- Keep paragraphs short (3-4 sentences)
- Use lists for steps or options

### Code Examples
- Every concept needs an example
- Examples should be copy-pasteable
- Show output where helpful
- Use realistic (not foo/bar) names

### Completeness
- [ ] All features documented
- [ ] Prerequisites listed
- [ ] Edge cases covered
- [ ] Troubleshooting section if applicable

## Format

Use the existing documentation style in this project.

{{#if project.doc_style}}
Style guide: {{project.doc_style}}
{{/if}}
```

### code-review

```markdown
---
name: code-review
purpose: Review code changes for quality
version: 1.0.0
author: chant-prompts
tags: [review, quality]
---

# Code Review

You are reviewing code for quality and correctness.

## Changes to Review

{{spec.body}}

## Review Criteria

### Correctness
- Does it do what it's supposed to?
- Are edge cases handled?
- Are there obvious bugs?

### Design
- Is the approach appropriate?
- Is it over-engineered or under-engineered?
- Does it follow existing patterns in the codebase?

### Readability
- Is the code self-documenting?
- Are names clear and consistent?
- Is the complexity appropriate?

### Testing
- Are there tests?
- Do tests cover the important cases?
- Are tests readable and maintainable?

### Performance
- Any obvious performance issues?
- Appropriate data structures used?
- N+1 queries? Unnecessary loops?

## Output

Add review comments to the spec:

```markdown
## Code Review Results

### Must Fix
[Blocking issues that must be addressed]

### Should Fix
[Important issues to address]

### Consider
[Suggestions for improvement]

### Praise
[What's done well - be specific]

**Verdict**: [APPROVE / REQUEST_CHANGES / NEEDS_DISCUSSION]
```
```

---

## Language & Framework Prompts

### rust-idioms

```markdown
---
name: rust-idioms
purpose: Rust-specific coding patterns
version: 1.0.0
author: community
tags: [rust, language]
extends: standard
---

# Rust Development

{{> standard}}

## Rust-Specific Guidelines

### Error Handling
- Use `Result<T, E>` for recoverable errors
- Use `?` operator for propagation
- Create custom error types for libraries
- Use `thiserror` for error derives
- Use `anyhow` for applications

### Ownership & Borrowing
- Prefer borrowing over cloning
- Use `&str` over `String` in function parameters
- Return owned types from constructors
- Use `Cow<str>` when ownership is conditional

### Patterns
- Use `Option` combinators (`map`, `and_then`, `unwrap_or`)
- Prefer iterators over manual loops
- Use `derive` macros generously
- Implement `From`/`Into` for conversions

### Testing
- Use `#[cfg(test)]` modules
- Use `assert_eq!` with descriptive messages
- Test error cases, not just happy path
- Use `proptest` for property-based testing

### Performance
- Profile before optimizing
- Use `#[inline]` sparingly
- Prefer stack allocation
- Use `Arc` only when needed

```bash
# Before committing
cargo fmt
cargo clippy -- -D warnings
cargo test
```
```

### react-components

```markdown
---
name: react-components
purpose: React component development
version: 2.0.0
author: community
tags: [react, frontend, javascript]
extends: standard
---

# React Component Development

{{> standard}}

## React Guidelines

### Component Structure
```tsx
// 1. Imports (external, then internal)
// 2. Types/interfaces
// 3. Component
// 4. Styles (if co-located)
```

### Patterns
- Functional components with hooks
- Props interface named `{Component}Props`
- Destructure props in parameters
- Use `React.FC` sparingly (prefer explicit return types)

### State Management
- `useState` for local state
- `useReducer` for complex state
- Context for cross-cutting concerns
- Avoid prop drilling beyond 2 levels

### Hooks
- Custom hooks start with `use`
- Extract complex logic to custom hooks
- Follow rules of hooks
- Use `useMemo`/`useCallback` when needed (not always)

### Testing
- React Testing Library over Enzyme
- Test behavior, not implementation
- Use `screen.getByRole` over `getByTestId`
- Mock at network boundary, not components

### Accessibility
- Semantic HTML elements
- ARIA labels where needed
- Keyboard navigation works
- Color contrast sufficient

```bash
# Before committing
npm run lint
npm run typecheck
npm run test
```
```

### django-views

```markdown
---
name: django-views
purpose: Django view development
version: 1.0.0
author: community
tags: [django, python, backend]
extends: standard
---

# Django Development

{{> standard}}

## Django Guidelines

### Views
- Class-based views for CRUD
- Function views for simple cases
- Use `get_object_or_404`
- Always validate user permissions

### Models
- Explicit `related_name` on ForeignKey
- Use `Meta` class for ordering, indexes
- Custom managers for common queries
- `__str__` on every model

### Forms
- ModelForm when possible
- Clean methods for validation
- `clean_<field>` for field-specific
- `clean()` for cross-field

### Security
- `@login_required` or `LoginRequiredMixin`
- Check object permissions, not just auth
- Use ORM, never raw SQL with user input
- CSRF tokens on all forms

### Testing
- `TestCase` for database tests
- `SimpleTestCase` for no-db tests
- Factory Boy for test data
- Test views with `Client`

```bash
# Before committing
python manage.py check
python manage.py test
black .
flake8
```
```

---

## Extending Prompts

### Basic Extension

```markdown
---
name: my-tdd
extends: tdd
---

# Additional Instructions

{{> tdd}}

## Team Conventions

Also follow our specific conventions:

- Use table-driven tests
- Mock external services with `testify/mock`
- Minimum 80% coverage on new code
- Integration tests in `_integration_test.go` files
```

### Override Section

```markdown
---
name: strict-security
extends: security-review
---

# Security Review (Strict Mode)

{{> security-review}}

## Additional Requirements

This is a financial application. Extra scrutiny required:

### Compliance
- [ ] PCI-DSS requirements met
- [ ] Audit logging for all transactions
- [ ] Data retention policies followed

### Zero Trust
- [ ] All inputs treated as malicious
- [ ] Internal APIs also authenticated
- [ ] Least privilege enforced

**Any security issue is blocking. No exceptions.**
```

### Composition (Multiple Parents)

```markdown
---
name: secure-tdd
extends: [tdd, security-review]
---

# Secure Test-Driven Development

Combine TDD with security awareness.

## Workflow

1. Write security-focused test (RED)
2. Implement securely (GREEN)
3. Review for vulnerabilities (SECURITY)
4. Refactor (REFACTOR)

{{> tdd}}

---

After each GREEN phase, run security checks:

{{> security-review}}
```

---

## Packing & Sharing

### Create Your Prompt

```markdown
# .chant/prompts/my-workflow.md
---
name: my-workflow
purpose: Our team's standard workflow
version: 1.0.0
author: yourname
tags: [workflow, team]
license: MIT
---

# Our Workflow

[Your prompt content...]
```

### Pack for Distribution

```bash
# Create distributable package
chant prompt pack my-workflow

# Creates: my-workflow-1.0.0.tar.gz
# Contents:
#   - my-workflow.md (the prompt)
#   - README.md (auto-generated usage)
#   - examples/ (if present)
```

### Publish

```bash
# To GitHub (recommended)
mkdir -p prompts && cp my-workflow.md prompts/
git add prompts/
git commit -m "Add my-workflow prompt"
git push

# Others install with:
chant prompt add --from github:you/repo/prompts/my-workflow.md

# To official registry (requires approval)
chant prompt publish my-workflow
```

### Version Updates

```bash
# Bump version
chant prompt version my-workflow --bump minor

# Publish update
git add .chant/prompts/my-workflow.md
git commit -m "chore: bump my-workflow to 1.1.0"
git push
```

---

## Prompt Repository Structure

For sharing multiple prompts:

```
my-prompts/
├── README.md
├── prompts/
│   ├── workflow-a.md
│   ├── workflow-b.md
│   └── security/
│       ├── audit.md
│       └── review.md
├── examples/
│   ├── workflow-a-example.md
│   └── workflow-b-example.md
└── LICENSE
```

### README Template

```markdown
# My Prompt Collection

Prompts for [use case].

## Installation

```bash
chant prompt add --from github:you/my-prompts/prompts/workflow-a.md
```

## Prompts

| Prompt | Purpose |
|--------|---------|
| workflow-a | Description |
| workflow-b | Description |
| security/audit | Description |

## Requirements

- Chant >= 0.5.0
- Recommended models: Claude 3 Opus, GPT-4

## Contributing

[Instructions]
```

---

## Quick Reference

### Using Community Prompts

```bash
# Install
chant prompt add tdd
chant prompt add security-review
chant prompt add --from github:user/repo/prompt.md

# Use
chant work 001 --prompt tdd
chant work 001 --prompt security-review

# List installed
chant prompt list

# Update
chant prompt update tdd
```

### Creating Your Own

```bash
# Create from template
chant prompt new my-prompt

# Or manually create .chant/prompts/my-prompt.md

# Test locally
chant work 001 --prompt my-prompt

# Share
git push  # others use --from github:...
```

### Extending

```yaml
---
extends: community-prompt
---
# Your additions...
{{> community-prompt}}
# More additions...
```
