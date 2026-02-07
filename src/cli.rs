//! CLI argument definitions for chant.

use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "chant")]
#[command(version)]
#[command(about = "Intent Driven Development", long_about = None)]
#[command(
    after_help = "GETTING STARTED:\n    chant init                 Interactive setup wizard (recommended)\n    chant init --help           Show all initialization options\n\n    The wizard guides you through project setup, model selection, and agent configuration."
)]
pub struct Cli {
    /// Suppress all non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize chant in the current directory
    ///
    /// TIP: Run 'chant init' with no arguments for an interactive setup wizard.
    /// The wizard guides you through all configuration options including:
    ///   - Project name and settings
    ///   - Model provider selection (Claude CLI, Ollama, OpenAI)
    ///   - Default model selection
    ///   - Agent configuration (creates CLAUDE.md, .mcp.json automatically)
    Init {
        /// Optional subcommand: 'prompts' to install/update prompts on existing projects
        #[arg(value_name = "SUBCOMMAND")]
        subcommand: Option<String>,
        /// Override detected project name
        #[arg(long)]
        name: Option<String>,
        /// Keep .chant/ local only (not tracked in git)
        #[arg(long)]
        silent: bool,
        /// Overwrite existing .chant/ directory
        #[arg(long)]
        force_overwrite: bool,
        /// Only create config.md, no prompt templates
        #[arg(long)]
        minimal: bool,
        /// Initialize agent configuration files (claude, cursor, kiro, generic, or all)
        /// Can be specified multiple times
        #[arg(long, value_name = "PROVIDER")]
        agent: Vec<String>,
        /// Set default model provider (claude, ollama, openai)
        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,
        /// Set default model (opus, sonnet, haiku, or custom model name)
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
        /// Set up git merge driver for spec files
        #[arg(long)]
        merge_driver: bool,
    },
    /// Add a new spec
    Add {
        /// Description of what to implement (ignored when using --template)
        #[arg(default_value = "")]
        description: String,
        /// Prompt to use for execution
        #[arg(long)]
        prompt: Option<String>,
        /// Require approval before this spec can be worked
        #[arg(long)]
        needs_approval: bool,
        /// Create spec from a template
        #[arg(long, value_name = "NAME")]
        template: Option<String>,
        /// Set template variable (can be specified multiple times, format: key=value)
        #[arg(long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,
    },
    /// Manage spec templates
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
    /// Approve a spec for work
    Approve {
        /// Spec ID (full or partial)
        id: String,
        /// Name of the person approving (validated against git committers)
        #[arg(long)]
        by: String,
    },
    /// Reject a spec with a reason
    Reject {
        /// Spec ID (full or partial)
        id: String,
        /// Name of the person rejecting (validated against git committers)
        #[arg(long)]
        by: String,
        /// Reason for rejection
        #[arg(long)]
        reason: String,
    },
    /// List specs
    List {
        /// Show only ready specs
        #[arg(long)]
        ready: bool,
        /// Filter by label (can be specified multiple times, shows specs with any matching label)
        #[arg(long)]
        label: Vec<String>,
        /// Filter by type (code, task, driver, documentation, research)
        #[arg(long)]
        r#type: Option<String>,
        /// Filter by status (pending, in_progress, completed, failed, blocked, cancelled)
        #[arg(long)]
        status: Option<String>,
        /// List specs from all configured repos in global config
        #[arg(long)]
        global: bool,
        /// Filter to specific repository (implies --global)
        #[arg(long)]
        repo: Option<String>,
        /// Filter to specific project within repository
        #[arg(long)]
        project: Option<String>,
        /// Filter by approval status (pending, approved, rejected)
        #[arg(long)]
        approval: Option<String>,
        /// Filter by spec creator name (from git log)
        #[arg(long)]
        created_by: Option<String>,
        /// Filter by recent activity (e.g., "2h", "1d", "1w")
        #[arg(long)]
        activity_since: Option<String>,
        /// Filter specs mentioning a person in approval discussion
        #[arg(long)]
        mentions: Option<String>,
        /// Show only count of matching specs
        #[arg(long)]
        count: bool,
        /// Skip branch resolution for in_progress specs (debug option)
        #[arg(long)]
        main_only: bool,
        /// Show project status summary
        #[arg(long)]
        summary: bool,
        /// Watch mode - refresh every 5 seconds (requires --summary)
        #[arg(long)]
        watch: bool,
        /// Brief single-line output (requires --summary)
        #[arg(long)]
        brief: bool,
        /// JSON output (requires --summary)
        #[arg(long)]
        json: bool,
    },
    /// Show spec details
    Show {
        /// Spec ID (full or partial) or repo:spec-id for cross-repo specs
        id: String,
        /// Show full spec body (default: summary only)
        #[arg(long)]
        body: bool,
        /// Disable markdown rendering
        #[arg(long)]
        no_render: bool,
        /// Output raw spec body without frontmatter or formatting (for agents)
        #[arg(long)]
        raw: bool,
        /// Strip agent conversation sections (used with --raw)
        #[arg(long)]
        clean: bool,
    },
    /// Edit a spec in $EDITOR
    Edit {
        /// Spec ID (full or partial)
        id: String,
    },
    /// Search specs by title and body content
    Search {
        /// Search query (omit to launch interactive wizard)
        query: Option<String>,
        /// Search title only
        #[arg(long)]
        title_only: bool,
        /// Search body only
        #[arg(long)]
        body_only: bool,
        /// Case-sensitive matching
        #[arg(long)]
        case_sensitive: bool,
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
        /// Filter by type
        #[arg(long)]
        type_: Option<String>,
        /// Filter by label (can be specified multiple times)
        #[arg(long)]
        label: Vec<String>,
        /// Filter by date (relative: 7d, 2w, 1m; or absolute: YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        /// Filter until date (relative: 7d, 2w, 1m; or absolute: YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
        /// Search active specs only
        #[arg(long)]
        active_only: bool,
        /// Search archived specs only
        #[arg(long)]
        archived_only: bool,
        /// Search across all configured repos
        #[arg(long)]
        global: bool,
        /// Filter to specific repository (implies --global)
        #[arg(long)]
        repo: Option<String>,
    },
    /// Execute a spec
    Work {
        /// Spec ID(s) (full or partial). If omitted with --parallel, executes all ready specs.
        #[arg(value_name = "ID")]
        ids: Vec<String>,
        /// Prompt to use
        #[arg(long)]
        prompt: Option<String>,
        /// Override dependency checks (work on a blocked spec)
        #[arg(long)]
        skip_deps: bool,
        /// Skip validation of unchecked acceptance criteria
        #[arg(long)]
        skip_criteria: bool,
        /// Execute all ready specs in parallel (when no spec ID provided). Optionally specify number of parallel workers.
        #[arg(long, value_name = "N", num_args = 0..=1, default_missing_value = "0", require_equals = true)]
        parallel: Option<usize>,
        /// Filter by label (can be specified multiple times, used with --parallel)
        #[arg(long)]
        label: Vec<String>,
        /// Re-finalize an existing spec (update commits and timestamp)
        #[arg(long)]
        finalize: bool,
        /// Allow spec to complete without matching commits (uses HEAD as fallback). Use only in special cases.
        #[arg(long)]
        allow_no_commits: bool,
        /// Override maximum parallel agents (deprecated: use --parallel=N instead)
        #[arg(long = "max")]
        max_parallel: Option<usize>,
        /// Skip cleanup prompt after parallel execution
        #[arg(long)]
        no_cleanup: bool,
        /// Force cleanup prompt even on success
        #[arg(long)]
        cleanup: bool,
        /// Skip approval check (for emergencies)
        #[arg(long)]
        skip_approval: bool,
        /// Chain through specs. If spec IDs provided, chains through only those IDs in order. If no IDs, chains through all ready specs.
        #[arg(long)]
        chain: bool,
        /// Maximum number of specs to chain (0 = unlimited, only with --chain)
        #[arg(long, default_value = "0")]
        chain_max: usize,
        /// Disable auto-merge after parallel execution (branches are kept for manual merge)
        #[arg(long)]
        no_merge: bool,
        /// Disable auto-rebase when merging branches in parallel execution
        #[arg(long)]
        no_rebase: bool,
        /// Disable auto-start of watch process (for testing)
        #[arg(long)]
        no_watch: bool,
    },
    /// Start MCP server (Model Context Protocol)
    Mcp,
    /// Show project status summary
    Status {
        /// Show status across all configured repos
        #[arg(long)]
        global: bool,
        /// Filter to specific repository (implies --global)
        #[arg(long)]
        repo: Option<String>,
        /// Watch mode - refresh every 5 seconds
        #[arg(long)]
        watch: bool,
        /// Brief single-line output
        #[arg(long)]
        brief: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
        /// Show disk usage of chant artifacts
        #[arg(long)]
        disk: bool,
        /// Show status of all chant worktrees
        #[arg(long)]
        worktrees: bool,
    },
    /// Refresh dependency status for all specs
    Refresh {
        /// Show detailed list of ready and blocked specs
        #[arg(short, long)]
        verbose: bool,
    },
    /// Validate all specs for common issues
    Lint {
        /// Spec ID (full or partial) to lint a single spec
        spec_id: Option<String>,
        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Show all dimension details including isolation and AC quality
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show log for a spec
    Log {
        /// Spec ID (full or partial)
        id: String,
        /// Number of lines to show (default: 50)
        #[arg(long, short = 'n', default_value = "50")]
        lines: usize,
        /// Do not follow the log in real-time (show static output)
        #[arg(long)]
        no_follow: bool,
        /// Show only a specific run (e.g., 'latest' for most recent run)
        #[arg(long)]
        run: Option<String>,
    },
    /// Split a spec into member specs
    Split {
        /// Spec ID to split (full or partial)
        id: String,
        /// Model to use for split analysis (overrides config)
        #[arg(long)]
        model: Option<String>,
        /// Force split even if spec status is not pending
        #[arg(long)]
        force_status: bool,
        /// Recursively split over-complex members (experimental)
        #[arg(long)]
        recursive: bool,
        /// Maximum recursion depth for recursive split (default: 2)
        #[arg(long, default_value = "2")]
        max_depth: usize,
    },
    /// Archive completed specs
    Archive {
        /// Spec ID (full or partial). If omitted, archives all completed specs.
        id: Option<String>,
        /// Dry run - show what would be archived without moving
        #[arg(long)]
        dry_run: bool,
        /// Archive specs older than N days
        #[arg(long)]
        older_than: Option<u64>,
        /// Allow archiving of non-completed specs
        #[arg(long)]
        allow_non_completed: bool,
        /// Create a commit after archiving (only in git repos)
        #[arg(long, default_value = "true", action = clap::ArgAction::SetTrue)]
        commit: bool,
        /// Skip creating a commit after archiving
        #[arg(long, conflicts_with = "commit")]
        no_commit: bool,
        /// Use fs::rename instead of git mv (for special cases)
        #[arg(long)]
        no_stage: bool,
    },
    /// Merge completed spec branches back to main
    Merge {
        /// Spec ID(s) to merge (one or more)
        #[arg(value_name = "ID")]
        ids: Vec<String>,
        /// Merge all completed spec branches
        #[arg(long)]
        all: bool,
        /// Merge all completed specs that have branches (convenience flag for post-parallel execution)
        #[arg(long)]
        all_completed: bool,
        /// List branch status without merging
        #[arg(long)]
        list: bool,
        /// Merge all ready branches (can fast-forward, all criteria met)
        #[arg(long)]
        ready: bool,
        /// Interactive mode to select which branches to merge
        #[arg(short, long)]
        interactive: bool,
        /// Preview merges without executing
        #[arg(long)]
        dry_run: bool,
        /// Delete branch after successful merge
        #[arg(long)]
        delete_branch: bool,
        /// Continue even if a single spec merge fails
        #[arg(long)]
        continue_on_error: bool,
        /// Skip confirmation prompt and proceed with merges
        #[arg(long)]
        yes: bool,
        /// Rebase branches onto main before merging (enables sequential merge of parallel branches)
        #[arg(long)]
        rebase: bool,
        /// Auto-resolve conflicts using agent (requires --rebase)
        #[arg(long)]
        auto: bool,
        /// Mark specs as completed after successful merge
        #[arg(long)]
        finalize: bool,
    },
    /// Diagnose spec execution issues
    Diagnose {
        /// Spec ID (full or partial)
        id: String,
    },
    /// Check for drift in documentation and research specs
    Drift {
        /// Spec ID (full or partial). If omitted, check all completed specs.
        id: Option<String>,
    },
    /// Reset a failed spec - resets it to pending and optionally re-runs it
    Reset {
        /// Spec ID (full or partial)
        id: String,
        /// Automatically re-execute the spec after resetting
        #[arg(long)]
        work: bool,
        /// Prompt to use if --work is specified
        #[arg(long)]
        prompt: Option<String>,
        /// Create a feature branch before re-executing (only with --work)
        #[arg(long, num_args = 0..=1, require_equals = true, value_name = "PREFIX")]
        branch: Option<String>,
    },
    /// Resume a failed spec - resets it to pending and optionally re-runs it (deprecated: use 'reset' instead)
    #[command(hide = true)]
    Resume {
        /// Spec ID (full or partial)
        id: String,
        /// Automatically re-execute the spec after resuming
        #[arg(long)]
        work: bool,
        /// Prompt to use if --work is specified
        #[arg(long)]
        prompt: Option<String>,
        /// Create a feature branch before re-executing (only with --work)
        #[arg(long, num_args = 0..=1, require_equals = true, value_name = "PREFIX")]
        branch: Option<String>,
    },
    /// Pause a running work process for a spec
    Pause {
        /// Spec ID (full or partial)
        id: String,
        /// Force pause without confirmation
        #[arg(long)]
        force: bool,
    },
    /// Take over a running spec, stopping the agent and analyzing progress
    Takeover {
        /// Spec ID (full or partial)
        id: String,
        /// Force takeover even if spec is not running
        #[arg(long)]
        force: bool,
    },
    /// Cancel a spec (soft-delete with status change, or hard-delete with --delete)
    Cancel {
        /// Spec ID (full or partial)
        id: String,
        /// Skip safety checks (status and dependency validation)
        #[arg(long)]
        skip_checks: bool,
        /// Hard delete: permanently remove spec file and artifacts (default: soft cancel to 'cancelled' status)
        #[arg(long)]
        delete: bool,
        /// Delete driver and all members (only with --delete)
        #[arg(long)]
        cascade: bool,
        /// Delete associated git branch (only with --delete)
        #[arg(long)]
        delete_branch: bool,
        /// Dry run - show what would be cancelled or deleted
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Validate configuration
    Config {
        /// Validate config semantically (check paths, prompts, limits)
        #[arg(long)]
        validate: bool,
    },
    /// Manage silent mode for suppressing git tracking and output
    Silent {
        /// Apply to global config instead of project config
        #[arg(long)]
        global: bool,
        /// Disable silent mode
        #[arg(long)]
        off: bool,
        /// Show current silent mode status
        #[arg(long)]
        status: bool,
    },
    /// Show version information
    Version {
        /// Show additional build information
        #[arg(long, short)]
        verbose: bool,
    },
    /// Export specs to JSON, CSV, or Markdown format
    Export {
        /// Output format (json, csv, markdown, or omit for interactive wizard)
        #[arg(long)]
        format: Option<String>,
        /// Filter by status (can be specified multiple times)
        #[arg(long)]
        status: Vec<String>,
        /// Filter by type (code, task, driver, etc.)
        #[arg(long)]
        type_: Option<String>,
        /// Filter by labels (can be specified multiple times, OR logic)
        #[arg(long)]
        label: Vec<String>,
        /// Only export ready specs
        #[arg(long)]
        ready: bool,
        /// Filter by date range (from date in YYYY-MM-DD format)
        #[arg(long)]
        from: Option<String>,
        /// Filter by date range (to date in YYYY-MM-DD format)
        #[arg(long)]
        to: Option<String>,
        /// Comma-separated fields to include (or 'all' for all fields)
        #[arg(long)]
        fields: Option<String>,
        /// Output file (if not specified, prints to stdout)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Show git-based activity feed for spec operations
    Activity {
        /// Filter by author name (case-insensitive substring match)
        #[arg(long)]
        by: Option<String>,
        /// Show activity since duration (e.g., "2h", "1d", "1w", "1m")
        #[arg(long)]
        since: Option<String>,
        /// Filter by spec ID (substring match)
        #[arg(long)]
        spec: Option<String>,
    },
    /// Remove orphan worktrees and stale artifacts
    Cleanup {
        /// Show what would be cleaned without removing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt and proceed with cleanup
        #[arg(long)]
        yes: bool,
        /// Only cleanup worktrees (skip other cleanup operations)
        #[arg(long)]
        worktrees: bool,
    },
    /// Worktree management commands
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommands,
    },
    /// Verify specs meet their acceptance criteria
    Verify {
        /// Spec ID to verify (full or partial). If omitted, verifies all specs.
        #[arg(value_name = "ID")]
        id: Option<String>,
        /// Verify all specs
        #[arg(long)]
        all: bool,
        /// Filter by label (can be specified multiple times)
        #[arg(long)]
        label: Vec<String>,
        /// Exit with code 1 if any spec fails verification
        #[arg(long)]
        exit_code: bool,
        /// Show what would be verified without making changes
        #[arg(long)]
        dry_run: bool,
        /// Prompt to use for verification
        #[arg(long)]
        prompt: Option<String>,
    },
    /// Output cleaned spec content for agent preparation
    Prep {
        /// Spec ID (full or partial)
        id: String,
        /// Strip agent conversation sections from spec body
        #[arg(long)]
        clean: bool,
    },
    /// Derive fields for specs based on enterprise config
    Derive {
        /// Spec ID (full or partial). If omitted with --all, derives for all specs.
        id: Option<String>,
        /// Derive for all specs
        #[arg(long)]
        all: bool,
        /// Show what would be derived without modifying specs
        #[arg(long)]
        dry_run: bool,
    },
    /// Finalize a completed spec - validate criteria, update status and model
    Finalize {
        /// Spec ID (full or partial)
        id: String,
    },
    /// Git merge driver for spec files (called by git, not directly by users)
    #[command(name = "merge-driver", hide = true)]
    MergeDriver {
        /// Path to base (common ancestor) version
        base: PathBuf,
        /// Path to current version (ours) - result is written here
        current: PathBuf,
        /// Path to other version (theirs)
        other: PathBuf,
    },
    /// Generate shell completion script
    Completion {
        /// Shell to generate completions for (bash, zsh, fish, powershell)
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Static site generation for spec documentation
    Site {
        #[command(subcommand)]
        command: SiteCommands,
    },
    /// Watch for spec completion and automatically finalize/merge
    Watch {
        /// Run only one iteration then exit (for testing)
        #[arg(long)]
        once: bool,
        /// Dry run - show what would be done without executing
        #[arg(long)]
        dry_run: bool,
        /// Poll interval in milliseconds (overrides config)
        #[arg(long)]
        poll_interval: Option<u64>,
    },
    /// Show dependency graph for specs
    Dag {
        /// Level of detail (minimal, titles, full)
        #[arg(long, default_value = "full")]
        detail: String,
        /// Filter by status (pending, in_progress, completed, failed, blocked, cancelled)
        #[arg(long)]
        status: Option<String>,
        /// Filter by label (can be specified multiple times)
        #[arg(long)]
        label: Vec<String>,
        /// Filter by type (code, task, driver, documentation, research)
        #[arg(long)]
        type_: Option<String>,
    },
    /// Generate man page
    #[command(hide = true)]
    Man {
        /// Output directory for the man page (defaults to current directory)
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
}

/// Subcommands for worktree management
#[derive(Subcommand)]
pub enum WorktreeCommands {
    /// Show status of all chant worktrees
    Status,
}

/// Subcommands for template management
#[derive(Subcommand)]
pub enum TemplateCommands {
    /// List available templates
    List,
    /// Show template details
    Show {
        /// Template name
        name: String,
    },
}

/// Subcommands for site generation
#[derive(Subcommand)]
pub enum SiteCommands {
    /// Initialize theme directory with default templates for customization
    Init {
        /// Overwrite existing theme files
        #[arg(long)]
        force_overwrite: bool,
    },
    /// Build the static site
    Build {
        /// Output directory (overrides config)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Start a local HTTP server to preview the site
    Serve {
        /// Port to serve on (default: 3000)
        #[arg(long, short, default_value = "3000")]
        port: u16,
        /// Output directory to serve (default: from config)
        #[arg(long, short)]
        output: Option<String>,
    },
}
