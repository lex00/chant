# Claude Code Instructions for Chant

## Overview

Chant is an Intent Driven Development tool that enables specification-driven development. Specs define work intentions, and the chant CLI executes them in isolated worktrees, ensuring reproducibility and auditability.

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
- `just chant add "description" --prompt <PROMPT>` - Create spec with custom prompt
- `just chant list` - List all specs (with `--ready`, `--type`, `--status`, `--label`, `--global`, `--repo`, `--project` filters)
- `just chant show <spec-id>` - View spec details (supports repo:spec-id for cross-repo specs)
- `just chant show <spec-id> --no-render` - View spec details without markdown rendering
- `just chant ready` - Show ready specs
- `just chant lint` - Validate all specs
- `just chant search [query]` - Search specs (or launch interactive wizard with no query)
  - Search flags: `--title-only`, `--body-only`, `--case-sensitive`
  - Filter flags: `--status <STATUS>`, `--type <TYPE>`, `--label <LABEL>` (multiple allowed)
  - Date filters: `--since <DATE>`, `--until <DATE>` (relative: 7d, 2w, 1m; or absolute: YYYY-MM-DD)
  - Scope flags: `--active-only`, `--archived-only`, `--global`, `--repo <REPO>`
- `just chant archive [<spec-id>]` - Archive completed specs (default: all if no ID); flags: `--dry-run`, `--older-than <N>`, `--force`, `--commit`, `--no-stage`
- `just chant cancel <spec-id>` - Cancel a spec (soft-delete); proceeds without prompting in non-TTY contexts; flags: `--force`, `--dry-run`, `--yes`
- `just chant delete <spec-id>` - Delete a spec and clean up artifacts; proceeds without prompting in non-TTY contexts; flags: `--force`, `--cascade`, `--delete-branch`, `--dry-run`, `--yes`

### Execution
- `just chant work <spec-id>` - Execute a spec
- `just chant work <spec-id> --branch` - Execute with feature branch
- `just chant work <spec-id> --branch=prefix` - Execute with custom branch prefix
- `just chant work <spec-id> --pr` - Execute and create pull request
- `just chant work <spec-id> --prompt <PROMPT>` - Execute with custom prompt
- `just chant work --parallel` - Execute all ready specs in parallel
- `just chant work --parallel --label <LABEL>` - Execute ready specs matching label(s)
- `just chant work <spec-id> --finalize` - Re-finalize an existing spec
- `just chant work <spec-id> --force` - Skip validation of unchecked acceptance criteria
- `just chant work <spec-id> --allow-no-commits` - Allow completion without matching commits (special cases only)
- `just chant work --parallel --max <N>` - Override maximum parallel agents
- `just chant work --parallel --no-cleanup` - Skip cleanup prompt after parallel execution
- `just chant work --parallel --cleanup` - Force cleanup prompt even on success
- `just chant resume <spec-id>` - Resume a failed spec (resets to pending)
- `just chant resume <spec-id> --work` - Resume and automatically re-execute
- `just chant resume <spec-id> --work --branch` - Resume with feature branch

### Autonomy Commands
- `just chant verify [<spec-id>]` - Verify acceptance criteria still pass; checks all completed specs if no ID provided
- `just chant verify --all` - Verify all completed specs
- `just chant verify --label <LABEL>` - Verify specs with matching label(s)
- `just chant verify --exit-code` - Exit with code 1 if any spec fails verification
- `just chant verify --dry-run` - Show what would be verified without making changes
- `just chant verify --prompt <PROMPT>` - Use custom prompt for verification
- `just chant replay <spec-id>` - Re-execute a completed spec
- `just chant replay <spec-id> --prompt <PROMPT>` - Re-execute with custom prompt
- `just chant replay <spec-id> --branch` - Re-execute with feature branch
- `just chant replay <spec-id> --pr` - Re-execute and create pull request
- `just chant replay <spec-id> --force` - Skip validation of unchecked acceptance criteria
- `just chant replay <spec-id> --dry-run` - Preview the replay without executing
- `just chant replay <spec-id> --yes` - Re-execute without confirmation prompt

### Utilities
- `just chant log <spec-id>` - Show spec execution log (with `-n` for line count and `--no-follow` for static output)
- `just chant status` - Project status summary
- `just chant split <spec-id>` - Split spec into member specs; auto-lints created member specs
- `just chant merge <spec-id>` - Merge spec branches back to main; proceeds without prompting in non-TTY contexts
- `just chant merge --all` - Merge all completed spec branches
- `just chant merge --dry-run` - Preview merges without executing
- `just chant merge --delete-branch` - Delete branch after successful merge
- `just chant merge --continue-on-error` - Continue even if a single spec merge fails
- `just chant merge --yes` - Skip confirmation prompt and proceed with merges
- `just chant merge --rebase` - Rebase branches before merging
- `just chant merge --rebase --auto` - Auto-resolve conflicts during rebase
- `just chant diagnose <spec-id>` - Diagnose spec execution issues
- `just chant drift [spec-id]` - Check for drift in documentation/research specs
- `just chant export` - Export specs (interactive wizard for format selection)
- `just chant export --format json` - Export specs as JSON
- `just chant export --format csv` - Export specs as CSV
- `just chant export --format markdown` - Export specs as Markdown
- `just chant export --format json --output file.json` - Export to file
- `just chant export --status <STATUS>` - Filter by status (can be specified multiple times)
- `just chant export --type <TYPE>` - Filter by type (code, task, driver, etc.)
- `just chant export --label <LABEL>` - Filter by labels (can be specified multiple times, OR logic)
- `just chant export --ready` - Only export ready specs
- `just chant export --from YYYY-MM-DD --to YYYY-MM-DD` - Filter by date range
- `just chant export --fields <FIELDS>` - Comma-separated fields to include (or 'all')
- `just chant disk` - Show disk usage of chant artifacts
- `just chant cleanup` - Remove orphan worktrees and stale artifacts (with `--dry-run`, `--yes` flags)
- `just chant config --validate` - Validate configuration

## Development Commands

These are available via `just` and are typically run during spec execution:

- `just build` - Build the binary with `cargo build`
- `just test` - Run tests with `cargo test`
- `just lint` - Run clippy linter
- `just fmt` - Format code with rustfmt
- `just check` - Run format check, linter, and tests
- `just all` - Full check and build

## Parallel Development Workflow

When working with multiple parallel specs that may have conflicts, use the merge with rebase workflow:

1. **Execute specs in parallel**: `just chant work --parallel`
2. **Merge with rebase for sequential integration**:
   ```bash
   just chant merge --all --rebase --auto
   ```
   This approach:
   - Rebases each spec's branch onto main before merging
   - Auto-resolves conflicts using the AI agent (with `--auto`)
   - Creates clean, sequential commit history
   - Better for complex features with interdependencies

**Without `--rebase`**, specs are merged directly (useful when branches don't conflict).

**Example workflow:**
```bash
# Execute three related specs in parallel
just chant work --parallel

# Check their status
just chant status

# Merge them sequentially, auto-resolving conflicts
just chant merge --all --rebase --auto

# Or preview merges first
just chant merge --all --rebase --dry-run
```

## Interactive Wizard Modes

Several commands support interactive wizard modes when invoked without arguments:

- `just chant search` - Launch interactive search wizard for filtering and finding specs
- `just chant export` - Launch interactive wizard to select export format and filters
- Many commands with `--help` will show available wizard options

Wizard modes provide guided interfaces for complex operations, making it easier to discover available filters and options.

## Releasing

Use the `just release` command to create a new release:

```bash
just release 0.2.0
```

**Prerequisites:**
- Clean working directory (no uncommitted changes)
- Version must be in `X.Y.Z` format
- Tag must not already exist

**What the release script does:**
1. Validates version format and checks for clean git state
2. Updates `version` in `Cargo.toml`
3. Builds release binary to update `Cargo.lock`
4. Commits changes with message `Release vX.Y.Z`
5. Creates annotated git tag `vX.Y.Z`
6. Pushes commit and tag to origin

**After the script completes:**
1. GitHub Actions automatically builds binaries for Linux, macOS (x86_64 + aarch64), and Windows
2. Binaries are uploaded to the GitHub release
3. Update the Homebrew formula with new SHA256 hashes from the release assets

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

## Configuration

### Agent Rotation

When executing specs in parallel, chant can distribute work across agents using different rotation strategies. Configure rotation in your chant configuration:

```yaml
parallel:
  rotation_strategy: round-robin  # Options: none, random, round-robin
```

- **none**: No rotation; use single agent for all specs
- **random**: Randomly assign specs to available agents
- **round-robin**: Distribute specs sequentially across agents (default)

This is useful for balancing load and ensuring reproducible execution patterns during parallel spec execution.

## Spec Format and Patterns

### Spec Filenames
- Format: `YYYY-MM-DD-XXX-abc.md`
- Example: `2026-01-24-01m-q7e.md`

### Frontmatter
```yaml
---
type: code | task | driver | group
status: pending | ready | in_progress | blocked | completed | cancelled
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

### Spec Statuses
- **pending**: Initial state; ready for acceptance criteria to be defined or refined
- **ready**: All acceptance criteria are defined; spec can be executed
- **in_progress**: Spec is currently being executed
- **blocked**: Dependencies are unmet; automatically applied when a spec depends on incomplete work
- **completed**: Spec has been executed and acceptance criteria are met
- **cancelled**: Spec has been soft-deleted; does not appear in normal listings

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

**Task Tool for Multi-Spec Parallelization:**
- ❌ **Never** use the Task tool to parallelize spec execution across multiple specs
- ❌ **Never** use the Task tool to invoke `chant work` on multiple specs in parallel
- ❌ **Never** use the Task tool to orchestrate multiple spec executions

**Why?** Chant has built-in orchestration for parallel execution:
- Use `just chant work --parallel` to execute all ready specs in parallel
- Use `just chant work --parallel --label <LABEL>` to execute labeled specs in parallel
- Chant handles agent rotation, worktree management, and conflict resolution
- Using Task to parallelize bypasses these safeguards and can cause conflicts

**What IS allowed - Task tool within a single spec:**
- ✅ **DO** use the Task tool to search/explore the codebase within a spec
- ✅ **DO** use the Task tool with `subagent_type: Explore` for codebase analysis
- ✅ **DO** use the Task tool with specialized agents (Bash, general-purpose, Plan) for research within a single spec
- ✅ **DO** use parallel tool calls within a single spec execution (e.g., reading multiple files in parallel)

**Examples:**

❌ **WRONG - Using Task to parallelize specs:**
```bash
# Don't do this - it bypasses chant's orchestration
Task tool with:
  description: "Run spec 1"
  subagent_type: bash
  prompt: "just chant work 2026-01-27-001-abc"

Task tool with:
  description: "Run spec 2"
  subagent_type: bash
  prompt: "just chant work 2026-01-27-002-def"
```

✅ **RIGHT - Use chant's built-in parallel execution:**
```bash
just chant work --parallel
```

✅ **RIGHT - Using Task for search within a spec:**
```bash
# This is fine - exploring the codebase for the current spec
Task tool with:
  description: "Find all API endpoints"
  subagent_type: Explore
  prompt: "Find all files that define API endpoints"
```

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
