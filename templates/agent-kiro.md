# Kiro Agent Rules for Chant

## Overview

Kiro is an AI-powered coding assistant. This configuration provides Kiro with instructions for working within Chant's specification-driven development workflow.

## Essential Rules for Chant

### 1. The Spec-First Approach

All work in this project follows Chant specifications:

- Specs define work intentions with clear acceptance criteria
- Specs are executed in isolated git worktrees
- Changes are auditable and reproducible
- Follow the spec, not assumptions

**Process:**
1. Read the spec and its acceptance criteria
2. Implement ONLY what the spec requires
3. Mark checkboxes when complete: `- [x]`
4. Reference spec ID in commits: `chant(SPEC-ID): description`

### 2. Use Project Build Tools

Use the project's build tools and commands as appropriate for the language and framework:

- Run tests using the project's test runner
- Use the project's linter or formatter if configured
- Follow any build or development scripts defined in the project

Common patterns:
- Node.js: `npm test`, `npm run lint`, `npm run build`
- Python: `pytest`, `pylint`, `black`
- Ruby: `rake test`, `rubocop`
- Go: `go test`, `go vet`, `gofmt`
- Rust: `cargo test`, `cargo clippy`, `cargo fmt`

### 3. Code Quality Standards

Every implementation must:

1. **Pass all tests**: Run the project's test suite
2. **Pass linting**: Use the project's linter if available
3. **Be formatted**: Apply the project's formatter if available
4. **Be minimal**: Only modify files in the spec
5. **Have tests**: Write tests validating acceptance criteria where applicable

## Workflow with Kiro

### Step 1: Understand the Spec

View the spec using the chant CLI:

```bash
chant show <spec-id>
```

Review:
- Spec description
- Acceptance criteria (the checkboxes)
- Target files listed
- Any constraints mentioned

### Step 2: Plan and Implement

1. Read relevant source files first
2. Use Kiro to understand code patterns
3. Implement changes following spec requirements
4. Write tests alongside implementation

### Step 3: Validate

Run the project's quality checks:

```bash
# Run tests (use your project's test command)
npm test          # Node.js example
pytest            # Python example
cargo test        # Rust example

# Run linter (if available)
npm run lint      # Node.js example
pylint .          # Python example
cargo clippy      # Rust example

# Format code (if available)
npm run format    # Node.js example
black .           # Python example
cargo fmt         # Rust example
```

Fix any issues before proceeding.

### Step 4: Update Spec and Commit

1. Mark completion in spec: `- [x]` for each criterion
2. Commit with spec reference:
   ```bash
   git commit -m "chant(SPEC-ID): brief description of what was implemented"
   ```

## Development Constraints

### DO

- Read code before suggesting changes
- Follow existing patterns and conventions in the codebase
- Write focused, minimal implementations
- Use the project's build tools and commands
- Validate with tests before committing
- Add comments only where logic is unclear

### DON'T

- Edit files outside the spec system
- Make unrelated improvements or refactoring
- Skip tests or linting
- Commit without validating your changes
- Add unnecessary dependencies or features

## Key Chant Commands Reference

| Purpose | Command |
|---------|---------|
| View spec | `chant show <spec-id>` |
| List all specs | `chant list` |
| Search specs | `chant search` |
| Show status | `chant status` |
| Create new spec | `chant add "<description>"` |

## Common Patterns in Chant

### Acceptance Criteria Format

Specs use checkboxes to track progress:

```markdown
## Acceptance Criteria

- [ ] Feature implemented
- [ ] Tests passing
- [ ] Code linted and formatted
```

### Commit Message Format

Always reference the spec ID:

```
chant(2026-01-25-00g-g7o): Add --agent flag to init command
```

### Target Files

Specs declare which files they modify:

```yaml
target_files:
- src/main.rs
- src/templates.rs
```

## Summary

Work with Kiro by:
1. Following specs strictly
2. Using the project's build tools and commands
3. Writing tests and validating quality
4. Committing with spec references
5. Keeping changes minimal and focused
