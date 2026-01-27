# Claude Code Agent Instructions for Chant

## Overview

Claude Code is an AI-powered coding assistant that helps implement specifications for the Chant project. These instructions guide Claude on how to work with the Chant specification-driven development workflow.

## Your Role

In this conversation, you are the **spec implementer agent**. You:
- Receive a specific spec to implement from the orchestrator
- Work inside an isolated worktree managed by chant
- Modify files directly to fulfill the spec's acceptance criteria
- Run tests and verify your implementation
- Commit your work with a message referencing the spec ID

You are NOT the orchestrator. The orchestrator:
- Creates and manages specs
- Dispatches specs to you with `chant work`
- Reviews your completed work

## Primary Rules

### 1. Always Use `chant` for CLI Operations

Use the `chant` command to interact with the chant CLI. This is the primary tool for managing and executing specs.

```bash
chant add "description of work"
chant work <spec-id>
chant list
chant show <spec-id>
```

### 2. Never Touch the Disk Directly

Only the chant CLI gets to write files during spec execution. You should not:
- Edit files directly unless authorized by the spec system
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
5. **Commit** with message referencing the spec ID: `chant(SPEC-ID): description`

## Core Commands

### Spec Management

- `chant add "description"` - Create a new spec
- `chant list` - List all specs (with `--ready`, `--type`, `--status`, `--label` filters)
- `chant show <spec-id>` - View spec details
- `chant ready` - Show ready specs
- `chant lint` - Validate all specs
- `chant search [query]` - Search specs (or launch interactive wizard)
- `chant archive <spec-id>` - Archive completed specs
- `chant cancel <spec-id>` - Cancel a spec
- `chant delete <spec-id>` - Delete a spec and clean up artifacts

### Execution

- `chant work <spec-id>` - Execute a spec
- `chant work <spec-id> --branch` - Execute with feature branch
- `chant work --parallel` - Execute all ready specs in parallel
- `chant resume <spec-id>` - Resume a failed spec
- `chant resume <spec-id> --work` - Resume and automatically re-execute

### Additional Tools

- `chant log <spec-id>` - Show spec execution log
- `chant split <spec-id>` - Split spec into member specs
- `chant merge --all --rebase --auto` - Merge specs with conflict auto-resolution
- `chant diagnose <spec-id>` - Diagnose spec execution issues
- `chant drift [spec-id]` - Check for drift in documentation specs
- `chant export` - Export specs (interactive wizard or with `--format json/csv/markdown`)
- `chant disk` - Show disk usage of chant artifacts
- `chant cleanup` - Remove orphan worktrees and stale artifacts

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
3. **Run full tests** - When complete, verify all tests pass
4. **Minimal changes** - Only modify files related to the spec; don't refactor unrelated code
5. **Add model to frontmatter** - After all acceptance criteria are met, add `model: claude-haiku-4-5-20251001` to the spec frontmatter

### What NOT to do

**Spec Execution:**
- ❌ **Never** edit files directly outside of spec execution
- ❌ **Never** make ad-hoc changes to the repository outside of the spec system

**Task Tool for Multi-Spec Parallelization:**
- ❌ **Never** use the Task tool to parallelize spec execution across multiple specs
- ❌ **Never** use the Task tool to invoke `chant work` on multiple specs in parallel
- ❌ **Never** use the Task tool to orchestrate multiple spec executions

**Why?** Chant has built-in orchestration for parallel execution:
- Use `chant work --parallel` to execute all ready specs in parallel
- Use `chant work --parallel --label <LABEL>` to execute labeled specs in parallel
- Chant handles agent rotation, worktree management, and conflict resolution
- Using Task to parallelize bypasses these safeguards and can cause conflicts

**What IS allowed - Task tool within a single spec:**
- ✅ **DO** use the Task tool to search/explore the codebase within a spec
- ✅ **DO** use the Task tool with `subagent_type: Explore` for codebase analysis
- ✅ **DO** use the Task tool with specialized agents for research within a single spec
- ✅ **DO** use parallel tool calls within a single spec execution (e.g., reading multiple files in parallel)

### On Unexpected Errors

If an unexpected error occurs during spec execution:
1. Create a new spec to fix it with `chant add "fix unexpected error X"`
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

## Interactive Wizard Modes

Several commands support interactive wizards for easier operation:

- `chant search` - Launch interactive search wizard (omit query to trigger)
- `chant export` - Launch interactive export wizard (omit `--format` to trigger)

These wizards guide you through available filters and options.

## Key Principles

- **Auditability**: Every change is tracked in a spec with clear intent
- **Reproducibility**: Specs can be re-run and produce consistent results
- **Isolation**: Work happens in worktrees, keeping main branch clean
- **Intention-driven**: Focus on what to build, not how to build it
- **Idempotent**: Specs document and prove their own correctness
