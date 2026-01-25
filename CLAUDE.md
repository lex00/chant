# Claude Code Instructions for Chant

## Overview

Chant is an Intent Driven Development tool that enables specification-driven development. Specs define work intentions, and the chant CLI executes them in isolated worktrees, ensuring reproducibility and auditability.

## Primary Rules

### 1. Always Use `just chant` for CLI Operations

Use the `just chant` command to interact with the chant binary, not `./target/debug/chant` or `cargo run`.

```bash
just chant add "description of work"
just chant work <spec-id>
just chant list
just chant show <spec-id>
```

### 2. Never Touch the Disk Directly

Only the chant CLI gets to write files during spec execution. AI agents should not:
- Edit files directly
- Run `cargo test` or `cargo build` directly
- Make ad-hoc changes outside of specs

All work must flow through the spec system.

### 3. Always Use a Spec for Every Operation

Even small changes require a spec. This ensures:
- All work is documented and auditable
- Changes are executed in isolated worktrees
- Work can be reviewed, rejected, or modified
- History is maintained in git

## Workflow

When asked to implement something:

1. **Create a spec** with `just chant add "description of the task"`
2. **Work the spec** with `just chant work <spec-id>` (or let the spec system do it)
3. **Review the result** and check acceptance criteria

The spec system handles all file modifications, testing, and git management.

## Core Commands

### Spec Management
- `just chant add "description"` - Create a new spec
- `just chant list` - List all specs
- `just chant show <spec-id>` - View spec details
- `just chant ready` - Show ready specs
- `just chant lint` - Validate all specs

### Execution
- `just chant work <spec-id>` - Execute a spec
- `just chant work <spec-id> --branch` - Execute with feature branch
- `just chant work <spec-id> --pr` - Execute and create pull request
- `just chant work --parallel` - Execute all ready specs

### Utilities
- `just chant log <spec-id>` - Show spec execution log
- `just chant status` - Project status summary
- `just chant split <spec-id>` - Split spec into members

## Development Commands

These are available via `just` and are typically run during spec execution:

- `just build` - Build the binary with `cargo build`
- `just test` - Run tests with `cargo test`
- `just lint` - Run clippy linter
- `just fmt` - Format code with rustfmt
- `just check` - Run format check, linter, and tests
- `just all` - Full check and build

## Project Structure

```
chant/
├── .chant/specs/          # Spec files (YYYY-MM-DD-XXX-abc.md)
├── src/
│   ├── main.rs           # CLI entry point and command handlers
│   ├── spec.rs           # Spec parsing and frontmatter handling
│   ├── config.rs         # Configuration management
│   ├── git.rs            # Git operations
│   ├── id.rs             # Spec ID generation
│   ├── prompt.rs         # Prompt management
│   ├── mcp.rs            # Model Context Protocol server
│   ├── worktree.rs       # Isolated worktree management
│   └── merge.rs          # Spec merge logic
├── docs/                  # MDBook documentation
├── Cargo.toml            # Rust dependencies
├── justfile              # Development commands
└── CLAUDE.md             # This file
```

## Spec Format and Patterns

### Spec Filenames
- Format: `YYYY-MM-DD-XXX-abc.md`
- Example: `2026-01-24-01m-q7e.md`

### Frontmatter
```yaml
---
type: code | task | driver | group
status: pending | ready | in_progress | blocked | completed
target_files:
- relative/path/to/file
model: claude-opus-4-5  # Added after all acceptance criteria met
---
```

### Spec Types
- **code**: Implement features, fix bugs, refactor
- **task**: Manual work, research, planning
- **driver**: Group multiple specs for coordinated execution
- **group**: Alias for driver

### Split Specs
Split specs use a `.N` suffix: `2026-01-24-01e-o0l.1`, `2026-01-24-01e-o0l.2`

### Acceptance Criteria
Use checkboxes to track completion:
```markdown
## Acceptance Criteria

- [ ] Feature X implemented
- [ ] All tests passing
- [ ] Code linted and formatted
```

## Important Constraints

### For AI Agents Working on Specs

1. **Read before modifying** - Always read relevant files first to understand existing code
2. **Write tests** - Validate behavior with tests and run until passing
3. **Lint everything** - Always run `just lint` and fix all errors and warnings
4. **Run full tests** - When complete, run `just test` to verify all tests pass
5. **Build must succeed** - Always ensure `cargo build` completes successfully
6. **Minimal changes** - Only modify files related to the spec; don't refactor unrelated code
7. **Add model to frontmatter** - After all acceptance criteria are met, add `model: claude-haiku-4-5-20251001` (or appropriate model) to the spec frontmatter

### On Unexpected Errors

If an unexpected error occurs during spec execution:
1. Create a new spec to fix it with `just chant add "fix unexpected error X"`
2. Do not continue with the original spec
3. Reference the original spec ID in the new spec

## Best Practices

### Spec Design
- Keep specs focused and single-purpose
- Write clear acceptance criteria that are verifiable
- Reference spec IDs in commit messages: `chant(2026-01-24-01m-q7e): implement feature X`
- Use `target_files:` frontmatter to declare modified files

### Testing
- Write tests that validate the spec's acceptance criteria
- Run tests frequently during implementation
- Ensure all tests pass before marking spec complete

### Code Quality
- Follow Rust style conventions (enforced by clippy and fmt)
- Add comments only where logic isn't self-evident
- Prefer simple solutions over over-engineered code

### Documentation
- Keep CLAUDE.md current as the project evolves
- Document non-obvious architectural decisions in spec descriptions
- Use git history to trace decision rationale

## Workflow Example

1. **User requests a feature**: "Add a verbose flag to the CLI"
2. **Create spec**: `just chant add "Add verbose flag to show more output"`
3. **Review the spec**: `just chant show 2026-01-24-abc-xyz`
4. **Execute**: `just chant work 2026-01-24-abc-xyz`
5. **Chant handles**:
   - Creating isolated worktree
   - Checking out correct branch
   - Running the spec through AI agent
   - Building and testing
   - Creating commit if successful
   - Cleaning up worktree
6. **Review result**: Check if acceptance criteria are met
7. **Iterate**: Create new spec if changes needed or use `--finalize` to re-run

## Key Principles

- **Auditability**: Every change is tracked in a spec with clear intent
- **Reproducibility**: Specs can be re-run and produce consistent results
- **Isolation**: Work happens in worktrees, keeping main branch clean
- **Intention-driven**: Focus on what to build, not how to build it
- **Idempotent**: Specs document and prove their own correctness
