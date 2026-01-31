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

### 2. Use Standard Development Commands

Follow the project's standard development workflow:

```bash
# Run tests appropriate to your project
npm test       # For JavaScript/TypeScript projects
pytest         # For Python projects
go test ./...  # For Go projects

# Format code according to project standards
npm run format # Or project-specific formatter
black .        # For Python
go fmt ./...   # For Go

# Run linter/static analysis
npm run lint   # Or project-specific linter
pylint .       # For Python
golangci-lint  # For Go
```

Adapt these commands to your project's specific tooling and conventions.

### 3. Work Within Specs

- Always read the spec's acceptance criteria first
- Implement only what the spec requires
- Mark checkboxes as complete: `- [x]` when done
- Reference spec IDs in commit messages: `chant(SPEC-ID): description`

### 4. Test-Driven Approach

- Write tests that validate acceptance criteria
- Run your project's test suite frequently during development
- Ensure all tests pass before considering a spec complete

## Code Quality

Cursor should enforce project-specific quality standards:
- Consistent code formatting (follow project's formatter configuration)
- Static analysis and linting (use project's configured linters)
- Type checking (if applicable to the language)
- Best practices for the specific language and framework being used

## Implementation Constraints

### DO

- Read code before modifying
- Follow existing patterns and conventions
- Write focused, minimal changes
- Use the project's standard build and test commands
- Run tests and linter before committing

### DON'T

- Edit files ad-hoc outside of specs
- Make unrelated code improvements
- Skip tests or linting
- Commit without running checks

## Integration with Chant

### Before Starting

1. Review the spec: `chant show <spec-id>`
2. Check acceptance criteria
3. Identify target files

### During Implementation

1. Use Cursor's AI assistance to understand code
2. Write tests alongside implementation
3. Run your test suite to validate changes
4. Use your linter to check code quality
5. Format code according to project standards

### After Completion

1. Mark acceptance criteria checkboxes: `- [x]`
2. Run your full test suite and quality checks for final validation
3. Create commit: `git commit -m "chant(SPEC-ID): description"`

## Parallel Development with Merge Workflow

When working with multiple parallel specs:

1. Execute specs in parallel: `chant work --parallel`
2. Merge with conflict auto-resolution: `chant merge --all --rebase --auto`

The `--rebase` and `--auto` flags:
- Rebase each branch sequentially onto main
- Auto-resolve conflicts using AI agent
- Create cleaner commit history for interdependent work

## Key Commands

| Action | Command |
|--------|---------|
| Show spec | `chant show <spec-id>` |
| List specs | `chant list` |
| Search specs | `chant search` |
| Execute spec | `chant work <spec-id>` |
| Execute in parallel | `chant work --parallel` |
| Merge specs | `chant merge --all --rebase --auto` |
| Run tests | Use project-specific test command |
| Check code | Use project-specific linter |
| Format code | Use project-specific formatter |
| Show status | `chant status` |
| Export specs | `chant export` |

## MCP Integration

Chant exposes a Model Context Protocol (MCP) server that provides structured tools for spec management. When MCP is configured, prefer using these tools over shelling out to the CLI - they're faster and provide structured responses.

### Setup

MCP is automatically configured when you run `chant init --agent cursor`. The `.cursor/mcp.json` file contains:

```json
{
  "mcpServers": {
    "chant": {
      "type": "stdio",
      "command": "chant",
      "args": ["mcp"]
    }
  }
}
```

### Available Tools

**Query Tools (read-only):**
- `chant_spec_list` - List all specs, optionally filtered by status
- `chant_spec_get` - Get full details of a spec including body content
- `chant_ready` - List specs that are ready to be worked (no unmet dependencies)
- `chant_status` - Get project summary with spec counts by status
- `chant_log` - Read execution log for a spec
- `chant_search` - Search specs by title and body content
- `chant_diagnose` - Diagnose issues with a spec (checks file, log, locks, commits, criteria)

**Mutating Tools:**
- `chant_spec_update` - Update a spec's status or append output
- `chant_add` - Create a new spec with description
- `chant_finalize` - Mark a spec as completed (validates all criteria are checked)
- `chant_resume` - Reset a failed spec to pending for rework
- `chant_cancel` - Cancel a spec (sets status to cancelled)
- `chant_archive` - Move a completed spec to the archive directory

### When to Use MCP vs CLI

**Use MCP tools when:**
- Checking spec status or listing specs
- Reading spec details or logs
- Creating, updating, or finalizing specs
- You need structured JSON responses

**Use CLI (`chant`) when:**
- Executing specs with `chant work` (spawns agent process - not available via MCP)
- Running interactive commands in a terminal
- Operations that need human confirmation

## Notes

- Cursor should respect the Chant workflow and spec system
- Focus on clear, maintainable code over clever solutions
- Always verify changes with tests before committing
- Keep PRs aligned with spec acceptance criteria
