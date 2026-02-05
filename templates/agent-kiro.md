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

### 2. Use `just` for All Development

All development commands use the `just` wrapper:

```bash
just build     # Build the project
just test      # Run tests
just lint      # Run linter (clippy)
just fmt       # Format code
just check     # Full check: format, lint, test
just all       # Full check and build
```

**Never run these directly:**
```bash
cargo build
cargo test
cargo run
./target/debug/chant
./target/release/chant
```

Why? The `just` wrapper ensures:
- Automatic rebuilds when source changes
- Always running the latest binary
- Consistent behavior across the team

### 3. Environment Variables

Configure these for optimal Kiro integration:

```bash
# Rust tooling
export RUST_LOG=info
```

### 4. Code Quality Standards

Every implementation must:

1. **Pass all tests**: `just test` must succeed
2. **Pass linting**: `just lint` must have zero errors
3. **Be formatted**: `just fmt` applied to all changes
4. **Be minimal**: Only modify files in the spec
5. **Have tests**: Write tests validating acceptance criteria

## Workflow with Kiro

### Step 1: Understand the Spec

```bash
just chant show <spec-id>
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

### Step 2a: Parallel Execution (if working with multiple specs)

For multiple related specs, execute them in parallel:
```bash
just chant work --parallel
```

Then merge with automatic conflict resolution:
```bash
just chant merge --all --rebase --auto
```

### Step 3: Validate

```bash
just test      # Run all tests
just lint      # Check linting
just fmt       # Format code
just check     # All of above
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
- Follow existing Rust patterns
- Write focused, minimal implementations
- Use `just` commands exclusively
- Validate with tests before committing
- Add comments only where logic is unclear

### DON'T

- Edit files outside the spec system
- Make unrelated improvements or refactoring
- Skip tests or linting
- Run cargo commands directly
- Commit without running `just check`
- Add unnecessary dependencies or features

## Key Commands Reference

| Purpose | Command |
|---------|---------|
| View spec | `just chant show <spec-id>` |
| List all specs | `just chant list` |
| Search specs | `just chant search` |
| Execute spec | `just chant work <spec-id>` |
| Execute parallel | `just chant work --parallel` |
| Merge specs | `just chant merge --all --rebase --auto` |
| Run tests | `just test` |
| Check code quality | `just lint` |
| Auto-format code | `just fmt` |
| Full validation | `just check` |
| Build binary | `just build` |
| Show status | `just chant status` |
| Export specs | `just chant export` |

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
2. Using `just` for all commands
3. Writing tests and validating quality
4. Committing with spec references
5. Keeping changes minimal and focused
