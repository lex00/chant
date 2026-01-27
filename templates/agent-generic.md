# Generic AI Agent Instructions for Chant

## Overview

This is a generic configuration for any AI-powered coding assistant working with Chant. Adapt these instructions to your specific tool while maintaining the core principles.

## Core Principles

### 1. Chant is Specification-Driven

- All work is defined in specs with acceptance criteria
- Specs execute in isolated, reproducible environments
- Changes are auditable and traceable
- **Follow the spec, not general best practices**

### 2. Use the Build System

All development uses the `just` wrapper:

```bash
just build     # Compile the project
just test      # Run test suite
just lint      # Check code quality
just fmt       # Format code
just check     # Run format, lint, and tests
just all       # Full validation and build
```

Direct cargo invocation should be avoided:
```bash
# DON'T do this:
cargo build
cargo test
cargo run
```

### 3. Spec-First Implementation

Every feature, fix, or change requires a spec:

1. **Read the spec first** - Understand requirements completely
2. **Check acceptance criteria** - These are your success metrics
3. **Implement only what's specified** - No scope creep
4. **Validate with tests** - Ensure the implementation works
5. **Commit with spec reference** - `chant(SPEC-ID): description`

## Development Workflow

### Before Coding

1. Review the spec: `just chant show <spec-id>`
2. Understand the acceptance criteria
3. Read relevant source files
4. Identify which files will be modified

### During Coding

1. Implement incrementally
2. Write tests alongside code
3. Run `just test` frequently
4. Follow existing code patterns
5. Keep changes minimal and focused

### Before Submitting

1. Run `just check` - must pass all checks
2. Mark spec checkboxes: `- [x]` when complete
3. Add model to spec frontmatter if required
4. Create commit: `git commit -m "chant(SPEC-ID): description"`

## Essential Rules

### DO

- Read code before making changes
- Write tests for new functionality
- Follow existing patterns and conventions
- Run `just lint` and `just test` frequently
- Keep commits focused on the spec
- Reference spec IDs in commit messages
- Mark acceptance criteria checkboxes as complete

### DON'T

- Edit files ad-hoc outside specs
- Make unrelated improvements or refactoring
- Skip tests or linting
- Use cargo commands directly
- Create large, unfocused changes
- Ignore existing code patterns

## Spec Format

### Structure

Specs are markdown files with YAML frontmatter:

```yaml
---
type: code | task | driver | group
status: pending | ready | in_progress | blocked | completed
target_files:
- relative/path/to/file
model: claude-haiku-4-5  # Added after completion
---

# Spec Title

## Description
...

## Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
```

### Acceptance Criteria

These are your completion checklist. Mark as done:

```markdown
- [x] Completed item
- [ ] Pending item
```

## Code Quality Standards

All code must meet these standards:

1. **Tests Pass**: `just test` succeeds
2. **Lint Clean**: `just lint` passes with zero errors
3. **Properly Formatted**: `just fmt` applied
4. **Minimal Scope**: Only spec-related changes
5. **Well-Commented**: Comments where logic isn't obvious

## Common Commands

| Task | Command |
|------|---------|
| View spec details | `just chant show <spec-id>` |
| List all specs | `just chant list` |
| Show ready specs | `just chant ready` |
| Search specs | `just chant search` |
| Execute spec | `just chant work <spec-id>` |
| Execute parallel | `just chant work --parallel` |
| Resume failed spec | `just chant resume <spec-id> --work` |
| Merge specs | `just chant merge --all --rebase --auto` |
| Show status | `just chant status` |
| Export specs | `just chant export` |
| Check disk usage | `just chant disk` |
| Run tests | `just test` |
| Run linter | `just lint` |
| Format code | `just fmt` |
| Full check | `just check` |
| Build | `just build` |

## Parallel Development Workflow

When executing multiple parallel specs with potential conflicts:

1. Execute all ready specs: `just chant work --parallel`
2. Merge with automatic rebase and conflict resolution:
   ```bash
   just chant merge --all --rebase --auto
   ```

This creates a clean sequential integration of parallel work.

## Interactive Wizard Modes

Some commands support interactive wizard modes (invoked without certain arguments):

- `just chant search` - Interactive spec search (omit query to launch wizard)
- `just chant export` - Interactive export format selection (omit `--format` to launch)

These wizards provide guided discovery of available options.

## Troubleshooting

### Unexpected Errors During Spec Work

If you encounter an error not covered by the spec:

1. Create a new spec: `just chant add "fix error X"`
2. Don't continue with the current spec
3. Reference the original spec ID in the new spec
4. Fix the issue in the new spec

### Tests Failing

When tests fail:

1. Read the test error carefully
2. Understand what the test validates
3. Fix the implementation or the test
4. Run `just test` until all pass
5. Continue with the spec

## Integration Guidelines

Different AI tools can be integrated with Chant:

- **Claude Code**: Use the Claude-specific instructions (CLAUDE.md)
- **Cursor IDE**: Use Cursor-specific configuration (.cursorrules)
- **Amazon Q**: Use Amazon Q-specific guidance (.amazonq/rules.md)
- **Other Tools**: Use this generic configuration

Adapt the core principles to your tool while maintaining the spec-driven workflow.

## Key Takeaways

1. **Specs define everything** - Read and follow them exactly
2. **Use `just` commands** - Never invoke cargo directly
3. **Test thoroughly** - Write and run tests
4. **Keep it simple** - Minimal, focused changes
5. **Quality first** - Always lint and format
6. **Reference specs** - Always mention spec ID in commits

Remember: Chant is about reproducible, auditable development. The spec system ensures everyone works the same way with the same results.
