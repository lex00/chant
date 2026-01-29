# Claude Code Agent Instructions for Chant

## Overview

Claude Code is an AI-powered coding assistant that helps implement specifications for the Chant project. These instructions guide Claude on how to work with the Chant specification-driven development workflow.

## Your Role

In this conversation, you are the **orchestrator**. You:
- Create specs with `chant add`
- Dispatch specs with `chant work`
- Review results with `chant show` and `chant log`

You do NOT implement code directly. Implementation happens inside isolated worktrees managed by chant, executed by a separate agent instance.

If a user says "work on X" or "implement X", your response should be `chant work <spec-id>`, not direct file edits.

## Spec Creation Workflow

⚠️ **Important**: Do NOT immediately work a freshly-created spec. Follow this 3-step workflow using `just chant`:

1. **Create the spec skeleton** with `just chant add "description"`
   - This creates a minimal spec with just a description

2. **Edit the spec file** to add detailed information
   - Add a detailed problem description
   - Describe the solution approach
   - Define clear acceptance criteria as a checklist
   - Example acceptance criteria structure:
     ```markdown
     ## Acceptance Criteria

     - [ ] Feature X implemented
     - [ ] All tests passing
     - [ ] Code linted and formatted
     ```

3. **Work the spec** with `just chant work <spec-id>`
   - Now that acceptance criteria are defined, the agent knows exactly what "done" means
   - Implementation can be validated against clear criteria

This workflow ensures:
- Clear definition of done before work starts
- Agent doesn't guess what "complete" means
- Work matches expectations
- All specs are thoroughly documented

## Orchestrator Pattern - Monitoring Agent Execution

As the orchestrator, you should actively monitor agents executing specs. Use `chant log <spec-id>` to check progress and detect struggling agents early.

### Struggling Agent Indicators

Watch for these signs that an agent is struggling:

- **Repeated errors**: The same error appearing multiple times, especially compilation or test failures that the agent cannot resolve
- **Circular fixes**: The agent repeatedly modifying the same code, undoing and redoing changes
- **Scope confusion**: The agent modifying files outside the spec's `target_files` or working on unrelated concerns
- **Long silences**: Extended periods with no meaningful progress in the log
- **Misunderstanding the task**: The agent implementing something different from what the spec describes
- **Excessive exploration**: Reading many files without making progress toward implementation

### Stop-and-Split Workflow

When you detect a struggling agent:

1. **Stop the agent** - Cancel the current execution
2. **Review the log** - Use `chant log <spec-id>` to understand where the agent got stuck
3. **Restructure the spec into phases**:
   - **Phase 1 - Research**: Create a spec to investigate the problem, identify the right approach, and document findings. This spec's acceptance criteria should produce a concrete plan, not code changes.
   - **Phase 2 - Implementation**: Create a spec that references the research findings and implements the solution with clear, narrow acceptance criteria.
4. **Evaluate further splitting** - If the implementation phase is still complex, split it into multiple focused specs

### Research vs Implementation Phase Split

**Research spec** (type: `task`):
- Goal: Understand the problem and produce a plan
- Acceptance criteria: Document findings, identify affected files, propose approach
- Does NOT modify production code
- Example: "Research how authentication middleware is structured and document the integration points for OAuth support"

**Implementation spec** (type: `code`):
- Goal: Make specific code changes based on known approach
- Acceptance criteria: Concrete, verifiable code changes
- References the research spec's findings
- Example: "Add OAuth provider configuration to authentication middleware using the integration points identified in spec 2026-01-28-001-abc"

### When to Split Specs

Split a spec into multiple specs when:

- **Multiple unrelated files**: The spec touches files in different subsystems with no shared logic
- **Research required**: The agent needs to understand a complex area before it can implement changes
- **Large acceptance criteria list**: More than 5-6 acceptance criteria often indicates the spec is too broad
- **Mixed concerns**: The spec combines refactoring with new features, or infrastructure changes with business logic
- **Sequential dependencies**: Part of the work must be complete and verified before the next part can begin

**Do NOT split** when:
- The changes are small and cohesive, even if they touch multiple files
- The acceptance criteria are all closely related aspects of one feature
- Splitting would create specs that can't be tested independently

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
  - Non-TTY hint: When stdin is not a terminal, run with explicit query: `chant search "keyword"`
  - Supports filters: `--status`, `--type`, `--label`, `--since`, `--until`
- `chant archive <spec-id>` - Archive completed specs
- `chant cancel <spec-id>` - Cancel a spec
- `chant delete <spec-id>` - Delete a spec and clean up artifacts

### Execution

- `chant work <spec-id>` - Execute a spec
  - Non-TTY hint: When stdin is not a terminal, provide spec ID explicitly: `chant work <SPEC_ID>`
  - Optional: `--prompt <name>`, `--branch`, `--force`, `--finalize`
- `chant work <spec-id> --branch` - Execute with feature branch
- `chant work --parallel` - Execute all ready specs in parallel
  - Supports: `--max-parallel N` to limit concurrent agents
  - Supports: `--label <LABEL>` to execute only labeled specs
  - Supports: `--no-merge` to disable auto-merge (branches preserved for manual merge)
  - Auto-merge behavior: Completed specs are automatically merged to main; failed specs preserve branches for debugging
- `chant work --chain` - Chain through ready specs until none remain or failure
  - Executes specs sequentially, one after another
  - Stops on first failure with proper exit code
  - Stops gracefully on Ctrl+C (SIGINT)
  - Supports: `--chain-max N` to limit number of specs to chain
  - Supports: `--label <LABEL>` to chain through labeled specs only
  - Supports: Starting spec ID: `chant work <spec-id> --chain` starts with that spec, then chains
  - Use cases: Overnight processing, CI/CD, unattended execution
- `chant resume <spec-id>` - Resume a failed spec
- `chant resume <spec-id> --work` - Resume and automatically re-execute

### Additional Tools

- `chant refresh` - Refresh dependency status for all specs
  - Reloads specs and recalculates ready/blocked status
  - Use `--verbose` for detailed list of ready and blocked specs
- `chant log <spec-id>` - Show spec execution log
- `chant split <spec-id>` - Split spec into member specs
- `chant merge --all --rebase --auto` - Merge specs with conflict auto-resolution
- `chant merge --finalize` - Merge and mark specs as completed atomically
- `chant finalize <spec-id>` - Finalize a completed spec (validate criteria, update status and model)
  - Automatically detects if spec has an active worktree
  - If worktree exists, finalizes in worktree and commits changes (prevents merge conflicts)
  - If no worktree, finalizes on current branch
- `chant diagnose <spec-id>` - Diagnose spec execution issues
- `chant drift [spec-id]` - Check for drift in documentation specs
- `chant export` - Export specs with wizard or direct options
  - Non-TTY hint: When stdin is not a terminal, provide format explicitly: `chant export --format json`
  - Formats: `--format json|csv|markdown`
  - Supports filters: `--status`, `--type`, `--label`, `--ready-only`
  - Options: `--output <file>` to save to file
- `chant disk` - Show disk usage of chant artifacts
- `chant cleanup` - Remove orphan worktrees and stale artifacts
- `chant init [--force]` - Initialize or reinitialize .chant/ directory
  - `--force`: Fully reinitialize while preserving specs, config, and custom files
  - Use when updating agent configurations or resetting to defaults

### Merge Conflict Resolution

When `chant merge` encounters conflicts, it provides detailed diagnostics to help you resolve them quickly.

**Conflict Detection**:
- Automatically detects conflict type (fast-forward, content, tree)
- Lists all conflicting files in the error output
- Suggests recovery strategies specific to the conflict type

**Enhanced Error Messages**:
```
Error: Merge failed due to conflicts
Files with conflicts:
  - src/main.rs
  - tests/integration_tests.rs

Next steps:
  1. Resolve conflicts manually, then: git merge --continue
  2. Or try automatic rebase: chant merge 00x-v6m --rebase --auto
  3. Or abort: git merge --abort
```

**Recovery Options**:
- `--rebase`: Rebase feature branches onto main before merging (resolves fast-forward issues)
- `--auto`: Auto-resolve conflicts using AI agent (requires `--rebase`)
- Manual resolution: Fix conflicts in your editor, stage files with `git add`, then run `git merge --continue`

## Configuration

### Configuration Hierarchy

Chant uses a layered configuration system with merge semantics:

1. **Global config** (`~/.config/chant/config.md`) - User-wide defaults
2. **Project config** (`.chant/config.md`) - Project-specific settings (committed to git)
3. **Agents config** (`.chant/agents.md`) - Agent definitions (gitignored, optional)

Later configs override earlier ones. The agents config only overrides the `parallel.agents` section.

### Global Configuration: `~/.config/chant/config.md`

The global config is the recommended place for agent definitions since they often contain API keys or account-specific settings:

```yaml
---
defaults:
  model: claude-opus-4
  rotation_strategy: round-robin

parallel:
  stagger_delay_ms: 1000
  agents:
    - name: main
      command: claude
      max_concurrent: 2
    - name: worker1
      command: claude-alt1
      max_concurrent: 3
    - name: worker2
      command: claude-alt2
      max_concurrent: 3

providers:
  ollama:
    max_retries: 3
    retry_delay_ms: 1000
  openai:
    max_retries: 5
    retry_delay_ms: 2000
---

# Global Chant Settings

My agent configuration for all projects.
```

### Project Configuration: `.chant/config.md`

Project config contains settings that should be shared with the team (committed to git):

```yaml
---
project:
  name: my-project

defaults:
  prompt: standard
  branch: false
---
```

**Note**: Agent definitions should NOT be in project config. Use global config or `.chant/agents.md` instead.

### Project Agents Override: `.chant/agents.md`

For project-specific agent overrides (rare case), create `.chant/agents.md`. This file is gitignored by default:

```yaml
---
parallel:
  agents:
    - name: project-specific
      command: claude-project
      max_concurrent: 2
---
```

This file only overrides the `parallel.agents` section. Other settings come from global or project config.

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

**Bash Backgrounding for Parallel Spec Work:**
- ❌ **Never** background chant commands with `&` (e.g., `chant work spec-1 &; chant work spec-2 &; wait`)
- ❌ **Never** use shell job control (`&`, `jobs`, `wait`) to parallelize spec execution
- ❌ **Never** manually parallelize spec work in bash

**Why?** Chant has built-in orchestration for parallel execution:
- Use `chant work --parallel` to execute all ready specs in parallel
- Use `chant work --parallel --label <LABEL>` to execute labeled specs in parallel
- Use `chant work spec-1 spec-2 spec-3` to work on multiple specific specs sequentially or with parallel mode
- Chant handles agent rotation, worktree management, and conflict resolution
- Using bash backgrounding or manual parallelization bypasses these safeguards, loses output visibility, and can cause conflicts

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
- Use `chant finalize <spec-id>` to complete a spec:
  - Validates all acceptance criteria are checked
  - Updates status to `completed`
  - Adds model and timestamp information to frontmatter
  - Ensures clean, auditable spec completion
- Enable the custom merge driver to auto-resolve frontmatter conflicts when merging branches (see "Custom Merge Driver for Specs" section below)

### Merge Conflicts

When merging specs back to main:
- Use `chant merge --all --rebase --auto` for automatic conflict resolution
- Review enhanced error messages for specific conflict details and recovery steps
- See [Merge Conflict Resolution](#merge-conflict-resolution) for detailed diagnostics

### Finalize Workflow (Worktree-Aware)

When using parallel execution (`chant work --parallel`) or feature branches, finalization
happens IN the worktree before the branch is merged to main:

1. **Agent completes work** → Changes committed to feature branch
2. **Auto-finalize in worktree** → Updates spec status to `completed`, adds `completed_at` and `model`
3. **Finalization committed** → `chant(<spec-id>): finalize spec` commit in feature branch
4. **Branch merged to main** → Both branches have same spec metadata, no conflict

This prevents the merge conflict that would occur if finalization happened on main:
- ✅ Feature branch: `status: completed`, `completed_at: ...`, `model: ...`
- ✅ Main branch (after merge): Same metadata, clean merge

Without worktree-aware finalization:
- ❌ Feature branch: `status: in_progress`
- ❌ Main branch: `status: completed`, `completed_at: ...`, `model: ...`
- ❌ Merge conflict on spec frontmatter

### Custom Merge Driver for Specs

Chant includes a custom git merge driver that automatically resolves frontmatter conflicts in `.chant/specs/*.md` files.

#### What It Does

When merging spec branches back to main, frontmatter conflicts commonly occur:
- Main branch: `status: completed` (from finalize)
- Feature branch: `status: in_progress`
- Conflict: Both sides modified the same fields

The merge driver:
- Detects frontmatter vs body conflicts
- Intelligently merges status, completed_at, and model fields
- Preserves implementation content (never discards code)
- Prevents accidental data loss from manual conflict resolution

**Merge Strategy:**
- `status`: Prefers the more "advanced" status (completed > in_progress > pending)
- `completed_at`, `model`: Takes values from whichever side has them (prefers finalized values)
- `commits`: Merges both lists, deduplicates
- `labels`, `target_files`, `context`: Merges lists, deduplicates
- Body content: Uses standard 3-way merge (shows conflict markers if both sides changed)

#### Installation

**Automatic** (recommended):
```bash
chant init --install-merge-driver
```

**Manual**:
1. Add to `.gitattributes` in your repository root:
   ```
   .chant/specs/*.md merge=chant-spec
   ```

2. Configure the git merge driver:
   ```bash
   git config merge.chant-spec.driver "chant merge-driver %O %A %B"
   git config merge.chant-spec.name "Chant spec merge driver"
   ```

   Or add directly to `.git/config`:
   ```ini
   [merge "chant-spec"]
       name = Chant spec merge driver
       driver = chant merge-driver %O %A %B
   ```

#### When It Activates

The driver activates automatically when:
- Merging any branch that modifies `.chant/specs/*.md` files
- Git detects a conflict in spec files
- `.gitattributes` is properly configured with the `merge=chant-spec` pattern

#### Verification

Check if the driver is configured:
```bash
# Check git config
git config --get merge.chant-spec.driver

# Check .gitattributes
grep chant-spec .gitattributes
```

Test with a merge scenario:
```bash
# Work on a spec with feature branch
chant work spec-id --branch

# Make changes, finalize on main
# Then merge the branch
git merge chant/spec-id  # Should auto-resolve frontmatter conflicts
```

#### Troubleshooting

**Driver not activating?**
- Verify `.gitattributes` exists and contains: `.chant/specs/*.md merge=chant-spec`
- Check git config: `git config --get merge.chant-spec.driver`
- Ensure chant binary is in PATH: `which chant`
- Make sure the file being merged matches the pattern `.chant/specs/*.md`

**Still getting conflicts?**
- Check if the conflict is in the spec body (not frontmatter)
- Body conflicts require manual resolution - the driver only auto-resolves frontmatter
- Review the conflict markers to understand what changed on each side

**Unexpected merge results?**
- The driver prefers "completed" status over "in_progress"
- Implementation content from your branch should be preserved
- If results seem wrong, run `chant show <spec-id>` to inspect the merged spec

### Testing
- Write tests that validate the spec's acceptance criteria
- Run tests frequently during implementation
- Ensure all tests pass before marking spec complete

## Interactive Wizard Modes

Several commands support interactive wizards for easier operation. Wizards only activate in TTY (terminal) contexts:

- `chant search` - Launch interactive search wizard (omit query to trigger)
  - In non-TTY contexts (piped input, CI/CD): Provide explicit query
  - Example: `chant search "keyword"`

- `chant work` - Launch interactive spec selector (omit spec ID to trigger)
  - In non-TTY contexts: Provide explicit spec ID
  - Example: `chant work 2026-01-27-001-abc`

- `chant export` - Launch interactive export wizard (omit `--format` to trigger)
  - In non-TTY contexts: Provide explicit format flag
  - Example: `chant export --format json`

These wizards guide you through available filters and options when running interactively in a terminal.

## Reinitialization with --force

The `chant init --force` flag allows full reinitialization of the `.chant/` directory while preserving important data:

- **When to use**: Update agent configurations, reset settings, or reinitialize without losing work
- **What it preserves**:
  - `.chant/specs/` - All active specs
  - `.chant/config.md` - Configuration settings
  - `.chant/prompts/` - Custom prompts
  - `.chant/.gitignore` - Git ignore rules
  - `.chant/.locks/` - Lock files
  - `.chant/.store/` - Data store

- **Usage**:
  ```bash
  # Interactive reinitialization (uses wizard if TTY)
  chant init

  # Force full reinitialization with specific agent
  chant init --force --agent claude-opus-4-5

  # Force with multiple agents
  chant init --force --agent claude-opus-4-5 --agent claude-haiku-4-5

  # Silent mode (non-interactive, validates no git tracking conflict)
  chant init --force --silent
  ```

## Keeping Agent Configuration in Sync

The repository contains two CLAUDE.md-related files that must stay in sync:

- **`CLAUDE.md`** (repo root) - Instructions for the **orchestrator** role. Contains orchestrator-specific sections (monitoring agents, configuration, merge workflows).
- **`templates/agent-claude.md`** - Template embedded in the chant binary for the **spec implementer** role. Installed as `CLAUDE.md` in user projects via `chant init --agent claude`.

These files share common sections (Core Commands, Spec Format, Important Constraints, Best Practices, etc.) but differ in role-specific content. When updating shared sections, update both files. The template is compiled into the binary via `include_str!()` in `src/templates.rs`, so changes require a rebuild.

**Update checklist:**
- When adding/modifying command documentation → update both files
- When adding orchestrator-specific guidance → update only `CLAUDE.md`
- When adding implementer-specific guidance → update only `templates/agent-claude.md`
- After updating `templates/agent-claude.md` → rebuild the binary and test with `chant init --force --agent claude`

## Key Principles

- **Auditability**: Every change is tracked in a spec with clear intent
- **Reproducibility**: Specs can be re-run and produce consistent results
- **Isolation**: Work happens in worktrees, keeping main branch clean
- **Intention-driven**: Focus on what to build, not how to build it
- **Idempotent**: Specs document and prove their own correctness
