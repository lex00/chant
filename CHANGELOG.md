# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.13.2] - 2026-02-01

### Fixed

- **Flaky integration test**: Fixed `test_status_blocked_filter_with_dependencies`
  - Replaced hardcoded `/tmp` path with `tempfile::TempDir` for proper isolation
  - Fixed invalid spec IDs in test that caused base36 parser overflow

## [0.13.1] - 2026-02-01

### Fixed

- **Base36 sequence sorting**: Spec IDs now sort correctly after 36+ specs per day
  - Previously `010` (seq 37) sorted before `00a` (seq 11) due to lexicographic comparison
  - Now parses base36 sequence numerically for correct ordering
  - Added `release` prompt for automating the release workflow

## [0.13.0] - 2026-02-01

### Added

- **Enhanced `chant status` command**: Comprehensive project status with multiple output modes
  - Activity tracking: Shows specs worked today and recent completions
  - Attention items: Highlights failed specs, blocked work, and items needing review
  - Ready queue: Lists next actionable specs
  - `--brief` flag: Single-line summary for IDE status bars and scripts
  - `--json` flag: Machine-readable output for tooling integration
  - `--watch` flag: Live updating status display

- **Consolidated git operations**: New `git.rs` module with reusable git helpers
  - `get_commits_in_range()` - Get commits between two refs
  - `get_commit_changed_files()` - Get files changed in a commit
  - `get_recent_commits()` - Get N most recent commits
  - `get_commits_for_path()` - Get commits touching a specific path
  - `get_file_at_commit()` - Read file content at a specific commit
  - Eliminates duplicate `Command::new("git")` calls across codebase

### Changed

- **Modular lifecycle command**: Split 4200-line `lifecycle.rs` into focused modules
  - `src/cmd/lifecycle/mod.rs` - Common types and re-exports
  - `src/cmd/lifecycle/split.rs` - Spec splitting logic
  - `src/cmd/lifecycle/merge.rs` - Branch merging
  - `src/cmd/lifecycle/archive.rs` - Spec archival
  - `src/cmd/lifecycle/drift.rs` - Drift detection
  - `src/cmd/lifecycle/resume.rs` - Failed spec resumption

- **Modular work command**: Split 4100-line `work.rs` into execution modes
  - `src/cmd/work/mod.rs` - Common types and re-exports
  - `src/cmd/work/single.rs` - Single spec execution
  - `src/cmd/work/chain.rs` - Chained execution
  - `src/cmd/work/parallel.rs` - Parallel execution with worktrees
  - `src/cmd/work/wizard.rs` - Interactive wizard mode

- **Reduced memory allocations**: Refactored lint.rs to use references
  - Clone count reduced from 28 to 0 in validation functions
  - Uses `&str` instead of `String` where ownership not needed
  - `HashSet<&str>` for spec ID lookups

### Fixed

- **Worktree spec status**: Specs now correctly show `in_progress` in worktrees
  - Previously showed stale `pending` status from committed state
  - Now copies updated spec file to worktree after creation
  - `chant list --status in_progress` works correctly during parallel execution

- **Driver spec completion**: Organizational specs without acceptance criteria complete immediately
  - Group/driver specs used as containers now auto-complete when all members done
  - No longer hang waiting for non-existent criteria to be checked

- **Split command output**: Improved quality of generated member specs
  - "### Provides" section headers preserved correctly
  - Independent members numbered before dependent ones
  - Original task mapping maintained after reordering

### Tests

- **Config error handling**: Added 14 tests for malformed configuration scenarios
  - Missing frontmatter, invalid YAML syntax
  - Missing required sections, nonexistent files
  - Malformed merge configurations

## [0.10.1] - 2026-01-31

### Added

- **Prompt extends/inheritance system**: Prompts can now extend other prompts
  - `extends:` field in prompt frontmatter specifies parent prompt
  - `{{> parent}}` marker in child prompt body for content injection
  - Parent content replaces the marker, allowing wrapper patterns
  - Enables DRY prompt organization with shared base prompts

- **Prompt extensions system**: Modular prompt extensions that can be combined
  - `prompt_extensions` array in config defaults section
  - Extensions loaded from `.chant/prompts/extensions/` directory
  - Extensions appended after main prompt content
  - First extension: `output-concise` for reducing agent output verbosity

- **Output-concise prompt extension**: Reduce agent output verbosity
  - Guides agents to produce minimal, essential output only
  - Avoid narration, status updates, and thinking-out-loud patterns
  - Focus on actions over descriptions
  - Installed via `prompt_extensions: [output-concise]` in config

- **`ensure_on_main_branch` safeguard**: Prevent main repo branch flipping
  - Automatically snaps main repository back to main branch
  - Called at command boundaries (start/end of work command)
  - Prevents parallel worktree operations from leaving main repo on feature branch
  - Logs warning when correction is needed

- **Smart spec resolution from working branches**: Read in-progress specs from their branches
  - `load_with_branch_resolution` reads spec content via `git show` from feature branch
  - No checkout required - reads directly from branch ref
  - Ensures spec status reflects actual branch state, not stale main copy
  - Particularly useful after interrupted parallel execution

- **Auto-rebase before merge in parallel**: Automatic conflict resolution
  - Detects when feature branch is behind main before merge
  - Attempts automatic rebase onto main
  - Falls back gracefully if rebase fails (merge proceeds without rebase)
  - Reduces manual conflict resolution in parallel workflows

- **Selective merge command**: Fine-grained control over branch merging
  - `chant merge --list` shows all spec branches with status (ahead/behind/diverged)
  - `chant merge --ready` merges only branches that are ahead and not diverged
  - `chant merge -i` interactive mode for selecting which branches to merge
  - Better visibility into branch state before merging

- **Automatic worktree cleanup on interrupt**: Clean state after Ctrl+C
  - `ParallelExecutionState` tracks active worktrees during parallel execution
  - SIGINT handler cleans up incomplete worktrees on interrupt
  - Panic hook also triggers cleanup for unexpected failures
  - Prevents orphaned worktrees from accumulating

- **Improved split command**: Better dependency graphs and member quality
  - Dependency DAG instead of linear chains (parallelizable when logical)
  - Context inheritance from parent to members (constraints, design principles)
  - Infrastructure ordering (logging/config specs prioritized early)
  - Requires section parsing for explicit member dependencies
  - Cycle detection in dependency validation

- **Group spec lifecycle improvements**: Better container spec handling
  - Parent group blocked until all members complete via `depends_on`
  - Groups filtered from `chant_ready` (containers, not actionable work)
  - Auto-complete parent when all members finalized
  - Cleaner orchestration of split workflows

- **Archive auto-commit**: Keep git status clean after archive
  - `chant archive` now auto-commits by default
  - Commit message format: `chant: Archive {spec-id}`
  - Skips commit if working directory has other uncommitted changes (warns user)
  - `--no-commit` flag to disable auto-commit behavior

- **Condensed archive warnings**: Reduce noise from repeated warnings
  - Groups identical warning types when count > 3
  - Shows condensed format: `⚠ {warning-type}: {count} specs`
  - Individual warnings still shown when count ≤ 3
  - Target_files mismatch warnings now concise and less alarming

- **Member spec sorting**: Correct ordering for large groups
  - Numeric sort for member numbers (y44.2 before y44.10)
  - Fixes display issues when groups have >10 members

### Fixed

- **Branch resolution after merge**: Auto-delete merged branches
  - Prevents stale branch reads after successful merge
  - Checks merge status before attempting branch resolution
  - Eliminates "branch not found" errors in workflows

- **Conflict spec YAML**: Fixed blocked_specs field format
  - Conflict spec templates now generate valid YAML
  - Proper array formatting for blocked_specs field

### Tests

- **Branch resolution tests**: Comprehensive tests for smart spec resolution
  - Tests for `branch_exists` detection
  - Tests for `read_spec_from_branch` content retrieval
  - Tests for `load_with_branch_resolution` full workflow
  - Edge cases: missing branches, invalid refs, concurrent modifications
  - Fixed flaky integration tests for branch resolution scenarios

## [0.7.1] - 2026-01-30

### Added

- **`chant worktree status` command**: View active worktrees with comprehensive details
  - Shows path, branch, HEAD commit, size, and age for each worktree
  - Useful for debugging and monitoring parallel execution
  - Helps identify stale worktrees that need cleanup

- **Worktree environment variables for agents**: Context injection during spec execution
  - `CHANT_WORKTREE` - Set to "true" when agent runs in a worktree
  - `CHANT_WORKTREE_PATH` - Absolute path to the worktree directory
  - `CHANT_BRANCH` - Name of the feature branch
  - Enables agents to detect and adapt to worktree execution context

- **Worktree context in agent prompts**: Template variables for prompt customization
  - `{{worktree.active}}` - Boolean indicating worktree execution
  - `{{worktree.path}}` - Path to worktree directory
  - `{{worktree.branch}}` - Feature branch name
  - Allows prompts to include worktree-specific instructions

- **MCP support for Cursor IDE**: Model Context Protocol configuration for Cursor
  - `chant init --agent cursor` now generates `.cursor/mcp.json`
  - Enables Cursor to use chant's MCP tools for spec management
  - Same tool surface as Claude: query, mutate, and manage specs

### Fixed

- **`chant init` agent update wizard creates MCP config**: Fixed missing MCP configuration on agent updates
  - Previously, running `chant init` to update agent configuration skipped MCP config creation
  - Now properly generates `.mcp.json` (for Claude) or `.cursor/mcp.json` (for Cursor) during updates
  - Ensures consistent configuration whether initializing new projects or updating existing ones

## [0.6.1] - 2026-01-29

### Added

- **Chain with specific IDs**: `chant work --chain spec1 spec2 spec3` now chains through only those specified specs in order
  - When spec IDs provided, chains through only those IDs (not all ready specs)
  - Invalid spec IDs fail fast with clear error before execution starts
  - Non-ready specs in the list are skipped with warning, chain continues
  - `--chain-max` limit applies to specified IDs
  - `--label` filter is ignored when specific IDs are provided
  - Documentation: `chant work --help` for usage

- **Agent approval workflow**: Automatic approval requirement for agent-assisted commits
  - Detects agent co-authorship in commits (Co-Authored-By: Claude, GPT, Copilot, Gemini, etc.)
  - Auto-sets `approval.required: true` when agent detected during finalization
  - New config setting: `approval.require_approval_for_agent_work` to enable/disable
  - Prevents merge without approval when required
  - Works with existing `chant approve`/`chant reject` commands

### Fixed

- **Race condition in branch mode finalization**: Specs are now finalized after merge, not before
  - Previously, specs were marked `Completed` in feature branch before merge to main
  - Now finalization is deferred until after successful merge
  - If merge fails, spec stays `in_progress` (not `Completed`)
  - If finalization fails after merge, spec is marked `NeedsAttention` with clear error
  - Eliminates status mismatch between main and feature branches

- **Performance**: Fix `chant list` performance regression (0.9s → 0.05s, 18x faster)
  - Batch git metadata loading instead of running 2 git commands per spec
  - Limit git history traversal to last 200 commits for speed
  - Only load creator info when `--created-by` filter is used

## [0.6.0] - 2026-01-29

### Added

- **`chant work --chain`**: Autonomous chaining through ready specs
  - `--chain` flag loops through ready specs until none remain or failure
  - `--chain-max N` limits chain length
  - Respects dependencies and label filters
  - Shows progress `[N/M]` with elapsed time
  - Graceful Ctrl+C handling
  - Perfect for overnight work queues, CI/CD, batch processing

- **Auto-merge for parallel + branch workflow**
  - Parallel execution now auto-merges completed branches to main
  - `chant work --parallel --no-merge` disables auto-merge for manual review
  - `chant merge --finalize` atomically merges and marks specs completed
  - Fixes 5 workflow issues: branch finalization, status preservation, cleanup errors

- **Automatic merge driver setup**
  - `chant init` now automatically configures git merge driver for spec files
  - No more manual `chant merge-driver-setup` needed
  - Graceful handling when outside git repo
  - Intelligent merging of status, commits, timestamps

- **Shell autocomplete support**
  - `chant completion <shell>` generates completions for bash, zsh, fish, PowerShell, elvish
  - Completes commands, flags, and options
  - Installation instructions in README and docs

- **Enterprise documentation**
  - Research workflow guide with ASCII dependency graph
  - Dual paths: academic (PhD student) and developer (staff engineer)
  - Shows how spec types coordinate across research phases
  - TDD workflow guide for enterprise teams
  - KPI/OKR terminology fixes and clarifications

### Changed

- **Agent configuration separation** (Breaking)
  - Agent configs moved from `.chant/config.md` (project) to `~/.config/chant/config.md` (global)
  - Optional `.chant/agents.md` for project-specific agent overrides (gitignored)
  - Config merge order: global → project → agents.md
  - **Migration**: Move `parallel.agents` section from `.chant/config.md` to global config
  - Keeps sensitive agent settings (API keys, accounts) out of git

- **Improved finalization**
  - Finalization now checks spec's branch field before main branch
  - Better error messages: suggests running `chant merge` when commits found on branch
  - Fixes parallel + branch workflow finalization failures

- **Merge improvements**
  - No longer tries to checkout deleted branches after successful merge
  - Preserves completed status from branch during merge (via custom merge driver)
  - Better error handling and status reporting

- **Default .gitignore updates**
  - `logs/` now included in `.chant/.gitignore` template
  - User-specific execution logs no longer accidentally committed
  - `agents.md` also gitignored by default

### Fixed

- **Windows CI**: Fixed stack overflow on Windows by spawning main logic on 8MB thread
- **Git-dependent tests**: Fixed integration tests to use temp repos with proper git setup
- **Branch initialization**: Fixed `git init -b main` for consistent branch naming
- **Documentation**: Fixed 20+ inaccuracies from audit (removed non-existent commands, moved planned features to planning/)
- **Config cleanup**: Removed stale `pr:` field from config and 18 test fixtures

### Documentation

- Complete shell completion setup guide
- Updated CLAUDE.md with agent config separation
- Created `docs/planning/` for unimplemented features
- `chant show` now displays frontmatter summary by default (use `--body` for full content)
- Fixed git hooks examples to use actual commands

### Licensing

- **Apache 2.0**: Migrated from MIT to Apache 2.0 license
  - Added LICENSE file with full Apache 2.0 text
  - Added NOTICE file with dependency attributions
  - Updated Cargo.toml, README.md, and all documentation

## [0.5.0] - 2026-01-28

### Added

- **Approval system**: Human-in-the-loop governance for spec execution
  - `chant approve <spec-id> --by <name>` - Approve specs with committer validation
  - `chant reject <spec-id> --by <name> --reason <text>` - Reject specs with reason
  - `chant add --needs-approval` - Create specs requiring approval
  - `chant work --skip-approval` - Emergency override for approval gate
  - `approval:` frontmatter schema (`required`, `status`, `by`, `at`)
  - "Approval Discussion" section in spec body for threaded conversation
  - Configurable rejection handling via `approval.rejection_action` (manual/dependency/group)

- **Activity tracking**: Git-based activity feed for all spec operations
  - `chant activity [--by] [--since] [--spec]` - Show chronological activity
  - Detects: CREATED, APPROVED, REJECTED, WORKED, COMPLETED events
  - Duration parsing: "2h", "1d", "1w", "1m"
  - Colored output with timestamp, author, action, spec-id

- **Enhanced list filtering**: Powerful filtering and visual indicators
  - `--approval <status>` - Filter by approval status (pending/approved/rejected)
  - `--created-by <name>` - Filter by spec creator (from git log)
  - `--activity-since <duration>` - Filter by recent activity
  - `--mentions <name>` - Filter specs mentioning person in approval discussion
  - `--count` - Show only count of matching specs
  - Visual indicators for creator, activity, comments, and approval status
  - Status markers: `[needs approval]`, `[rejected]`, `[approved]`

- **Rejection action handlers**: Configurable workflows for rejected specs
  - Manual mode: Leave rejected, user handles it
  - Dependency mode: Create fix spec, original becomes blocked
  - Group mode: Convert to driver with numbered member specs
  - `members:` frontmatter field for driver/group specs

- **Approval documentation**: Comprehensive approval system documentation
  - New guide: `docs/guides/approval-workflow.md` with examples
  - Full CLI reference coverage for all new commands
  - Config reference with approval settings
  - Updated lifecycle diagram with approval gate
  - Examples for team development workflows

### Changed

- **Roadmap restructured**: Now forward-looking only
  - Removed 492 lines of historical/implemented feature content
  - Roadmap focuses on planned features (v0.5.0, v1.0.0)
  - Updated 11 cross-references to remove phase mentions

- **CLAUDE.md improvements**: Better orchestrator guidance
  - Added "Orchestrator Pattern - Monitoring Agent Execution" section
  - Documents struggling agent indicators and stop-and-split workflow
  - Synced repo and template versions for consistency

### Removed (Breaking)

- **GitHub PR integration**: Completely removed
  - Removed `--pr` flag from `chant work` and `chant replay`
  - Deleted PR creation for GitHub, GitLab, Bitbucket
  - Removed `GitConfig`, `GitProvider`, `pr` field from config and frontmatter
  - Removed 774 lines of PR-related code
  - Documentation updated to emphasize spec-as-PR model
  - **Migration**: Specs are already the PR primitive. Use `chant work --branch` for feature branches without external PR creation.

### Fixed

- **mdBook links**: Fixed broken GitHub icon and internal links
  - Fixed GitHub repository URL (`chant-dev/chant` → `lex00/chant`)
  - Fixed 8 broken internal documentation links
  - Added `docs-check-links` justfile recipe for maintenance

### Documentation

- Complete approval workflow guide with team/solo examples
- CLI reference: All new commands and flags documented
- Config reference: Approval configuration with examples
- FEATURE_STATUS.md: Marked approval features as implemented
- Link audit: Fixed all broken internal and external links

## [0.4.0] - 2026-01-28

### Added

- **`chant refresh` command**: Reload specs and recalculate dependency status
  - Shows summary counts (completed, ready, in-progress, pending, blocked)
  - `--verbose` flag lists ready and blocked specs with their dependencies
  - Useful for debugging dependency chains and verifying status after manual changes
  - Fully documented in CLI reference and CLAUDE.md

- **`chant merge --all-completed` flag**: Convenience flag for post-parallel workflows
  - Merges only completed specs that have branches (perfect after `chant work --parallel`)
  - Differs from `--all` which merges all completed specs regardless of branches
  - Documented with comparison table and examples in CLI reference

- **Custom merge driver for spec frontmatter**: Auto-resolve conflicts when merging spec branches
  - Intelligently merges `status`, `completed_at`, and `model` fields
  - Prevents accidental loss of implementation during conflict resolution
  - Install with `chant init --install-merge-driver` (planned)
  - Manual setup via `.gitattributes` and git config
  - Full installation and troubleshooting guide in CLAUDE.md

- **Enhanced merge conflict detection**: Detailed error messages for merge failures
  - Detects conflict type (fast-forward, content, tree)
  - Lists all conflicting files
  - Provides numbered recovery steps
  - Suggests appropriate flags (`--rebase`, `--auto`)
  - Documented in "Merge Conflict Resolution" section in CLAUDE.md

- **Archive target file verification**: Warns when archiving specs without implementation
  - Checks if `target_files` were actually modified by spec commits
  - Detects merge conflicts resolved incorrectly
  - Shows net additions for each file
  - Prevents accidental archival of specs with lost implementations

- **`chant work --force` flag**: Override dependency checks when working specs
  - Allows working blocked specs for testing or urgent fixes
  - Documented with warning about dependency violations

- **Improved blocked spec errors**: Show detailed dependency information
  - Lists blocking dependencies with their status
  - Shows `completed_at` for completed dependencies
  - Warns if dependency is complete but spec still shows as blocked
  - Provides actionable next steps (`chant refresh`, `--force`, etc.)

- **Finalize in worktree**: Prevents merge conflicts during parallel execution
  - Finalization now happens in the feature branch worktree before merge
  - Both main and feature branch have same metadata = no frontmatter conflicts
  - Automatic in `chant work --parallel` and `chant work --branch --finalize`
  - Documented in "Finalize Workflow (Worktree-Aware)" section

### Fixed

- **Compilation error**: Added missing `all_completed` parameter to `cmd_merge` call
  - Was causing build failure after incomplete integration of 00x-v6m feature
  - Now properly passes flag through to merge implementation

- **Spec status updates**: Parallel execution now properly updates spec status to completed
  - Fixed issue where specs remained `in_progress` after successful completion
  - Added automatic status updates in worktree finalization

- **Divergence detection**: Automatic `--no-ff` for diverged branches during merge
  - Detects when branches have diverged from main
  - Prevents fast-forward that would lose parallel work
  - Ensures merge commits preserve full history

- **Worktree cleanup**: Automatic cleanup after parallel execution
  - Removes stale worktrees from `/tmp/chant-*`
  - Cleans up after both successful and failed spec execution
  - Prevents disk space accumulation from parallel runs

### Changed

- **Reconcile renamed to merge**: Help text and documentation updated
  - All references to `reconcile` changed to `merge` for consistency
  - CLI command remains `chant merge` (was already the name)

### Documentation

- **CLAUDE.md improvements**:
  - Added "Merge Conflict Resolution" section with recovery strategies
  - Added "Custom Merge Driver for Specs" with installation guide
  - Updated finalization workflow documentation for worktree-aware operation
  - Clarified parallel execution model selection behavior

- **CLI reference updates** (`docs/reference/cli.md`):
  - Full `chant refresh` command documentation with examples
  - Documented `--all-completed` flag with comparison to `--all`
  - Added post-parallel workflow examples

### Tests

- **Integration tests for derivation**:
  - Path-based derivation from git branch names
  - `chant derive --all` batch processing
  - `chant derive --dry-run` preview mode
  - Invalid regex pattern graceful handling
  - Unicode and special characters in values

- **Integration tests for parallel execution**:
  - Parallel work and merge workflow end-to-end
  - `--force` flag overriding dependency checks
  - Automatic worktree cleanup verification

- **Unit tests**:
  - Special characters in derived field values
  - Unicode handling in derivation sources
  - Blocking dependency status calculation

## [0.3.7] - 2026-01-28

### Fixed

- **Remote branches deleted after merge**: After merging a spec branch to main, now also deletes the remote branch
  - Prevents stale remote branches from accumulating after parallel execution
  - Best-effort deletion - merge still succeeds if remote is unavailable

## [0.3.6] - 2026-01-28

### Fixed

- **Commits now reliably recorded in frontmatter**: Fixed bug where commits were found during validation but not recorded
  - Previously `get_commits_for_spec` was called twice (once to validate, once in finalize)
  - Now commits are fetched once and passed directly to `finalize_spec`
  - Eliminates race condition that could cause commits to be lost

## [0.3.5] - 2026-01-27

### Added

- **`chant finalize` command**: Properly complete specs with validation
  - Validates all acceptance criteria are checked
  - Sets status to completed, adds model/timestamp/commits
  - Works on in_progress, completed, or failed specs

- **Auto-finalize in `chant work`**: After agent exits, automatically finalize if criteria checked
  - Checks for commits first (ensures work was done)
  - Runs lint to validate spec
  - Auto-finalizes if all criteria checked
  - Fails with clear message if criteria unchecked

- **Lint warns on unchecked criteria**: `chant lint` now warns if completed/in_progress specs have unchecked boxes

### Fixed

- **Resume handles in_progress specs**: `chant resume` now accepts in_progress specs (not just failed)
- **Finalize accepts failed specs**: All finalize paths now accept failed status

## [0.3.4] - 2026-01-27

### Fixed

- **Archive uses git mv**: Archive command now uses `git mv` for flat archive migration to preserve history

### Changed

- **Agent docs improved**: Clearer guidance for agents working with chant
  - Explicit warning against bash backgrounding for parallel work (use `chant work --parallel`)
  - 3-step workflow documented: add spec → edit spec → work spec
  - Warning against immediately working a freshly-created spec

## [0.3.3] - 2026-01-27

### Added

- **Bundled prompts**: All 11 standard prompts are now embedded in the binary
  - `chant init` automatically creates `.chant/prompts/` with all bundled prompts
  - Prompts include: bootstrap (default), standard, split, verify, documentation, doc-audit, merge-conflict, parallel-cleanup, ollama

- **`chant init prompts`**: Install/update prompts on already-initialized projects
  - Adds missing prompts without overwriting user customizations
  - Upgrade path for existing chant users to get new prompts

## [0.3.2] - 2026-01-27

### Added

- **Bootstrap prompt**: Minimal default prompt that fetches spec content via `chant prep`
  - Reduces API concurrency issues by starting with minimal prompt
  - Agent runs `chant prep {{spec.id}}` to get full spec and instructions
  - Better support for replay/resume scenarios with `chant prep --clean`
  - Cleaner separation between spec content and agent instructions

- **`chant prep` command**: Fetch and output spec content for direct agent use
  - Returns cleaned spec content with agent conversation sections removed (on replays)
  - Used by bootstrap prompt to get instructions at execution time
  - Supports `--clean` flag for replay scenarios

- **Spawn jitter**: Configurable delay jitter for parallel agent spawning
  - Reduces thundering herd in concurrent execution
  - `stagger_jitter_ms` config option (default: 200ms, 20% of stagger delay)
  - Prevents synchronized retries and API spike cascades

### Changed

- **Default prompt changed to bootstrap**
  - Reduces initial prompt size and API load
  - Improves handling of large specs and concurrent execution
  - More robust for replay and resume scenarios
  - Use `--prompt standard` explicitly if you prefer full spec upfront

## [0.1.2] - 2026-01-25

### Added

- **Native Ollama/OpenAI provider support**: Use local or cloud LLMs directly
  - `OllamaProvider` for local models via OpenAI-compatible API
  - `OpenaiProvider` for OpenAI API with authentication
  - Provider abstraction via `ModelProvider` trait
  - Configurable via `defaults.provider` and `providers` section in config
  - Streaming output for all providers
  - Clear error messages for connection, auth, and model issues

- **`chant init --agent`**: Generate AI assistant configuration files
  - `--agent claude` creates CLAUDE.md for Claude Code
  - `--agent cursor` creates .cursorrules for Cursor IDE
  - `--agent amazonq` creates .amazonq/rules.md for Amazon Q
  - `--agent generic` creates .ai-instructions for any assistant
  - `--agent all` creates all configuration files
  - Templates embedded in binary (no network required)

- **Silent mode for private usage**: Keep chant local-only
  - `chant init --silent` adds `.chant/` to `.git/info/exclude`
  - `--pr` blocked in silent mode (prevents revealing usage)
  - `--branch` warns in silent mode (branch names visible)
  - `chant status` shows "(silent mode)" indicator
  - `--force` flag for reinitializing
  - `--minimal` flag for config-only initialization

- **`chant version` command**: Display version and build info
  - `chant --version` and `chant version` both work
  - `--verbose` flag shows commit hash and build date

- **Homebrew tap**: Install via `brew install lex00/tap/chant`

### Fixed

- **Cross-platform CI compatibility**:
  - Use `git init -b main` for consistent branch naming
  - PathBuf comparison for Windows path separators
  - Skip Unix-specific tests on Windows
  - Replace curl subprocess with ureq HTTP library

- **Standard prompt ordering**: Format/lint now runs before commit
  - Prevents uncommitted changes from `cargo fmt` after commit
  - Added "verify git status is clean" step

- **Windows binary extension**: Fixed double `.exe.exe` in release artifacts

### Changed

- mdBook tagline updated to "Idempotent Intention"

## [0.1.1] - 2026-01-25

### Added

- **`chant delete` command**: Safely remove specs with comprehensive cleanup
  - `--force` for in-progress/completed specs
  - `--cascade` to delete driver and all members
  - `--delete-branch` to remove associated git branches
  - `--dry-run` to preview deletions
  - Automatic cleanup of log files and worktrees
  - Safety checks for dependencies and member specs

- **Markdown rendering for `chant show`**: Rich terminal output using pulldown-cmark
  - Formatted headings, bold, italic, code blocks
  - Syntax highlighting for code
  - `--no-render` flag for raw output
  - Respects `NO_COLOR` environment variable
  - Auto-detects TTY for smart rendering

- **Conflict auto-spawn**: Automatic conflict resolution spec creation
  - Detects merge conflicts during parallel execution
  - Creates detailed conflict specs with context
  - Tracks blocked specs and conflicting files
  - New `type: conflict` spec type with ⚡ indicator

- **Archive folder organization**: Date-based archive structure
  - Specs archived to `.chant/archive/YYYY-MM-DD/` folders
  - Automatic migration of flat archive files
  - `chant show` finds archived specs in subfolders

- **README badges**: CI status, license, and release badges
- **Installation documentation**: Comprehensive install guide with curl, cargo, and build instructions
- **Enhanced standard prompt**: Guidance for out-of-scope issues and duplicate prevention

### Fixed

- Release workflow now properly triggers on version tags
- `chant show` now finds archived specs
- Test failures from parallel execution interference
- Formatting issues in generated code

### Changed

- Log command now auto-follows by default (`--no-follow` to disable)
- Archive command automatically includes all group members

## [0.1.0] - 2026-01-25

### Added

- **Core Spec System**: Markdown-based spec format with YAML frontmatter for declaring work intentions
- **Spec Execution**: Agent-driven execution of specs with acceptance criteria validation
- **Isolated Worktrees**: Automatic creation and management of git worktrees for spec execution
- **Git Integration**: Seamless branch creation, merging, and commit management for each spec
- **Command-Line Interface**:
  - `chant init` - Initialize chant in a project
  - `chant add` - Create new specs
  - `chant list` - List all specs with status
  - `chant work` - Execute a spec
  - `chant show` - Display spec details
  - `chant merge` - Merge completed specs back to main branch
  - `chant status` - View project status
  - `chant diagnose` - Check spec execution health
  - `chant split` - Break specs into smaller components
  - `chant archive` - Archive completed specs
  - `chant log` - View spec execution logs
- **Spec Types**: Support for code, task, driver, and group specs
- **Driver Specs**: Coordinate execution of multiple dependent specs
- **Parallel Execution**: Run multiple ready specs in parallel with isolated worktrees
- **Configuration Management**: Global and project-level configuration with git provider support
- **Model Context Protocol (MCP)**: Server implementation for integrating with Claude and other AI models
- **Acceptance Criteria**: Checkbox-based tracking of completion requirements
- **Labels**: Tag specs for organization and filtering
- **Model Persistence**: Track which model completed each spec
- **Pull Request Creation**: Automatic PR creation with merge summaries
- **Spec Member System**: Split specs into numbered components with dependency ordering
- **Dry-Run Mode**: Preview merge operations before executing
- **Comprehensive Testing**: 227 unit tests + integration tests ensuring reliability

### Features

- **Intent-Driven Development**: Specs document intentions; agents implement them
- **Reproducibility**: Specs can be re-run and produce consistent results
- **Auditability**: All work tracked in git with clear lineage
- **Drift Detection**: Identify when reality diverges from specifications
- **Idempotent Operations**: Specs designed for safe re-execution
- **Flexible Execution**: Branch mode (PR-based) or direct mode (commits to main)

### Technical Details

- Written in Rust with strong type safety
- Built on clap for CLI argument parsing
- Uses git2-rs for git operations
- Supports multiple git providers (GitHub, GitLab, Gitea)
- YAML parsing with serde_yaml
- Cross-platform (tested on Linux, macOS, Windows)

## Getting Started

See [README.md](README.md) for installation and quick start instructions.

For comprehensive documentation, visit [lex00.github.io/chant](https://lex00.github.io/chant).
