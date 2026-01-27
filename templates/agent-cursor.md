# Cursor IDE Agent Rules for Chant

## Overview

This configuration provides Cursor IDE with instructions for working with Chant specification-driven development. Cursor is an AI-powered code editor that can accelerate development when properly configured.

## Primary Rules

### 1. Understand the Chant Workflow

Chant is a specification-driven development tool where:
- Work is defined in specs with clear acceptance criteria
- Specs are executed in isolated worktrees
- Changes flow through the spec system, not ad-hoc edits
- All work is auditable and reproducible

### 2. Use `just` Commands

Use the `just` wrapper for all development commands:

```bash
just build     # Build the project
just test      # Run tests
just lint      # Run linter (clippy)
just fmt       # Format code
just check     # Full check: format, lint, test
just all       # Full check and build
```

**Never run:**
```bash
cargo build
cargo test
cargo run
./target/debug/chant
```

### 3. Work Within Specs

- Always read the spec's acceptance criteria first
- Implement only what the spec requires
- Mark checkboxes as complete: `- [x]` when done
- Reference spec IDs in commit messages: `chant(SPEC-ID): description`

### 4. Test-Driven Approach

- Write tests that validate acceptance criteria
- Run `just test` frequently during development
- Ensure all tests pass before considering a spec complete

## Workspace Configuration

### Recommended Settings

Add to `.cursor/settings.json` or project settings:

```json
{
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer",
  "[rust]": {
    "editor.formatOnSave": true
  },
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### Code Quality

Cursor should enforce:
- Rust formatting with `rustfmt`
- Linting with `clippy`
- Type checking with `rust-analyzer`

## Implementation Constraints

### DO

- Read code before modifying
- Follow existing patterns and conventions
- Write focused, minimal changes
- Use the `just` commands exclusively
- Run tests and linter before committing

### DON'T

- Edit files ad-hoc outside of specs
- Run cargo commands directly
- Make unrelated code improvements
- Skip tests or linting
- Commit without running checks

## Integration with Chant

### Before Starting

1. Review the spec: `just chant show <spec-id>`
2. Check acceptance criteria
3. Identify target files

### During Implementation

1. Use Cursor's AI assistance to understand code
2. Write tests alongside implementation
3. Use `just test` to validate changes
4. Use `just lint` to check code quality
5. Use `just fmt` to format code

### After Completion

1. Mark acceptance criteria checkboxes: `- [x]`
2. Run `just all` for final validation
3. Create commit: `git commit -m "chant(SPEC-ID): description"`

## Parallel Development with Merge Workflow

When working with multiple parallel specs:

1. Execute specs in parallel: `just chant work --parallel`
2. Merge with conflict auto-resolution: `just chant merge --all --rebase --auto`

The `--rebase` and `--auto` flags:
- Rebase each branch sequentially onto main
- Auto-resolve conflicts using AI agent
- Create cleaner commit history for interdependent work

## Key Commands

| Action | Command |
|--------|---------|
| Show spec | `just chant show <spec-id>` |
| List specs | `just chant list` |
| Search specs | `just chant search` |
| Execute spec | `just chant work <spec-id>` |
| Execute in parallel | `just chant work --parallel` |
| Merge specs | `just chant merge --all --rebase --auto` |
| Run tests | `just test` |
| Check code | `just lint` |
| Format code | `just fmt` |
| Full check | `just check` |
| Show status | `just chant status` |
| Export specs | `just chant export` |

## Notes

- Cursor should respect the Chant workflow and spec system
- Focus on clear, maintainable code over clever solutions
- Always verify changes with tests before committing
- Keep PRs aligned with spec acceptance criteria
