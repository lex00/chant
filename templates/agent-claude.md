# Claude Code Agent Instructions for Chant

## Overview

Claude Code is an AI-powered coding assistant that helps implement specifications for the Chant project. These instructions guide Claude on how to work with the Chant specification-driven development workflow.

## Primary Rules

### 1. Always Use `just chant` for CLI Operations

Use **ONLY** the `just chant` command to interact with the chant binary. Never use direct binary paths like `./target/debug/chant` or `cargo run`.

**Why?** The `just` wrapper ensures:
- The binary is automatically rebuilt if source code changed
- You always run the most recent version
- Consistent interface and behavior across all operations
- Avoids stale binary issues from previous builds

```bash
just chant add "description of work"
just chant work <spec-id>
just chant list
just chant show <spec-id>
```

**What NOT to do:**
```bash
# ❌ WRONG - Don't use direct binary paths
./target/debug/chant add "description"
./target/release/chant work <spec-id>

# ❌ WRONG - Don't use cargo run
cargo run -- add "description"
cargo run --release -- work <spec-id>
```

### 2. Never Touch the Disk Directly

Only the chant CLI gets to write files during spec execution. Claude should not:
- Edit files directly unless authorized by the spec system
- Run `cargo test` or `cargo build` directly (use `just test`, `just build` instead)
- Make ad-hoc changes outside of specs

All work must flow through the spec system.

### 3. Always Use Specs for Every Operation

Even small changes require a spec. This ensures:
- All work is documented and auditable
- Changes are executed in isolated worktrees
- Work can be reviewed, rejected, or modified
- History is maintained in git

## Workflow

When implementing a spec:

1. **Read** the relevant code first to understand existing patterns
2. **Plan** your approach before making changes
3. **Implement** the changes according to spec acceptance criteria
4. **Verify** with tests and ensure all pass
5. **Lint** with `just lint` and fix all errors and warnings
6. **Commit** with message referencing the spec ID: `chant(SPEC-ID): description`

## Core Commands

### Development Commands

These are available via `just` and should be used during spec execution:

- `just build` - Build the binary with `cargo build`
- `just test` - Run tests with `cargo test`
- `just lint` - Run clippy linter
- `just fmt` - Format code with rustfmt
- `just check` - Run format check, linter, and tests
- `just all` - Full check and build

## Spec Format and Patterns

### Spec Structure

Specs are markdown files with YAML frontmatter:

```yaml
---
type: code | task | driver | group
status: pending | ready | in_progress | blocked | completed
target_files:
- relative/path/to/file
model: claude-haiku-4-5  # Added after all acceptance criteria met
---
```

### Acceptance Criteria

Specs include checkboxes to track completion:

```markdown
## Acceptance Criteria

- [ ] Feature X implemented
- [ ] All tests passing
- [ ] Code linted and formatted
```

Change `- [ ]` to `- [x]` as you complete each criterion.

## Important Constraints

### For Claude Implementing Specs

1. **Read before modifying** - Always read relevant files first to understand existing code
2. **Write tests** - Validate behavior with tests and run until passing
3. **Lint everything** - Always run `just lint` and fix all errors and warnings
4. **Run full tests** - When complete, run `just test` to verify all tests pass
5. **Build must succeed** - Always ensure the binary builds successfully
6. **Minimal changes** - Only modify files related to the spec; don't refactor unrelated code
7. **Add model to frontmatter** - After all acceptance criteria are met, add `model: claude-haiku-4-5-20251001` to the spec frontmatter

### What NOT to do

**Binary/Build Execution:**
- ❌ **Never** run `./target/debug/chant` or `./target/release/chant` directly
- ❌ **Never** run `cargo run -- ` to invoke chant
- ❌ **Never** run `cargo build` or `cargo test` directly (use `just build`, `just test` instead)

These bypass the `justfile` wrapper, which means:
- You may run stale binaries from previous builds
- Source changes won't trigger automatic rebuilds
- You lose consistency across the development team
- Build environment assumptions aren't validated

### On Unexpected Errors

If an unexpected error occurs during spec execution:
1. Create a new spec to fix it with `just chant add "fix unexpected error X"`
2. Do not continue with the original spec
3. Reference the original spec ID in the new spec

## Best Practices

### Code Quality
- Follow Rust style conventions (enforced by clippy and fmt)
- Add comments only where logic isn't self-evident
- Prefer simple solutions over over-engineered code
- Avoid refactoring unrelated code

### Spec Completion
- Keep changes focused on the spec's acceptance criteria
- Reference spec IDs in commit messages: `chant(2026-01-24-01m-q7e): implement feature X`
- Use `target_files:` frontmatter to declare modified files
- Mark acceptance criteria as complete by changing checkboxes to `[x]`

### Testing
- Write tests that validate the spec's acceptance criteria
- Run tests frequently during implementation
- Ensure all tests pass before marking spec complete

## Key Principles

- **Auditability**: Every change is tracked in a spec with clear intent
- **Reproducibility**: Specs can be re-run and produce consistent results
- **Isolation**: Work happens in worktrees, keeping main branch clean
- **Intention-driven**: Focus on what to build, not how to build it
- **Idempotent**: Specs document and prove their own correctness
