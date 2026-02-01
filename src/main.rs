//! CLI entry point and command handlers for chant.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/cli.md
//! - ignore: false

// Internal modules not exposed via library
mod mcp;
mod render;
mod templates;

mod cmd;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "chant")]
#[command(version)]
#[command(about = "Intent Driven Development", long_about = None)]
#[command(
    after_help = "GETTING STARTED:\n    chant init                 Interactive setup wizard (recommended)\n    chant init --help           Show all initialization options\n\n    The wizard guides you through project setup, model selection, and agent configuration."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
        force: bool,
        /// Only create config.md, no prompt templates
        #[arg(long)]
        minimal: bool,
        /// Initialize agent configuration files (claude, cursor, amazonq, generic, or all)
        /// Can be specified multiple times
        #[arg(long, value_name = "PROVIDER")]
        agent: Vec<String>,
        /// Set default model provider (claude, ollama, openai)
        #[arg(long, value_name = "PROVIDER")]
        provider: Option<String>,
        /// Set default model (opus, sonnet, haiku, or custom model name)
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
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
        /// Create a feature branch before executing (optionally with a custom prefix)
        #[arg(long, num_args = 0..=1, require_equals = true, value_name = "PREFIX")]
        branch: Option<String>,
        /// Override dependency checks and skip validation of unchecked acceptance criteria
        #[arg(long)]
        force: bool,
        /// Execute all ready specs in parallel (when no spec ID provided)
        #[arg(long)]
        parallel: bool,
        /// Filter by label (can be specified multiple times, used with --parallel)
        #[arg(long)]
        label: Vec<String>,
        /// Re-finalize an existing spec (update commits and timestamp)
        #[arg(long)]
        finalize: bool,
        /// Allow spec to complete without matching commits (uses HEAD as fallback). Use only in special cases.
        #[arg(long)]
        allow_no_commits: bool,
        /// Override maximum parallel agents (for --parallel)
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
    },
    /// Refresh dependency status for all specs
    Refresh {
        /// Show detailed list of ready and blocked specs
        #[arg(short, long)]
        verbose: bool,
    },
    /// Validate all specs for common issues
    Lint {
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
    },
    /// Split a spec into member specs
    Split {
        /// Spec ID to split (full or partial)
        id: String,
        /// Model to use for split analysis (overrides config)
        #[arg(long)]
        model: Option<String>,
        /// Force split even if spec is not pending
        #[arg(long)]
        force: bool,
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
        /// Force archive of non-completed specs
        #[arg(long)]
        force: bool,
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
    /// Resume a failed spec - resets it to pending and optionally re-runs it
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
    /// Replay a completed spec - re-execute it with the same or updated options
    Replay {
        /// Spec ID (full or partial)
        id: String,
        /// Prompt to use for execution
        #[arg(long)]
        prompt: Option<String>,
        /// Create a feature branch before executing (optionally with a custom prefix)
        #[arg(long, num_args = 0..=1, require_equals = true, value_name = "PREFIX")]
        branch: Option<String>,
        /// Skip validation of unchecked acceptance criteria
        #[arg(long)]
        force: bool,
        /// Preview the replay without executing (show what would happen)
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt and proceed directly with replay
        #[arg(long)]
        yes: bool,
    },
    /// Cancel a spec (soft-delete with status change)
    Cancel {
        /// Spec ID (full or partial)
        id: String,
        /// Force cancel even if not pending
        #[arg(long)]
        force: bool,
        /// Dry run - show what would be cancelled
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Delete a spec and clean up artifacts
    Delete {
        /// Spec ID (full or partial)
        id: String,
        /// Force delete even if not pending
        #[arg(long)]
        force: bool,
        /// Delete driver and all members
        #[arg(long)]
        cascade: bool,
        /// Delete associated git branch
        #[arg(long)]
        delete_branch: bool,
        /// Dry run - show what would be deleted
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
    /// Show disk usage of chant artifacts
    Disk,
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
    #[command(name = "merge-driver")]
    MergeDriver {
        /// Path to base (common ancestor) version
        base: PathBuf,
        /// Path to current version (ours) - result is written here
        current: PathBuf,
        /// Path to other version (theirs)
        other: PathBuf,
    },
    /// Show setup instructions for the spec merge driver
    #[command(name = "merge-driver-setup")]
    MergeDriverSetup,
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
}

/// Subcommands for worktree management
#[derive(Subcommand)]
enum WorktreeCommands {
    /// Show status of all chant worktrees
    Status,
}

/// Subcommands for template management
#[derive(Subcommand)]
enum TemplateCommands {
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
enum SiteCommands {
    /// Initialize theme directory with default templates for customization
    Init {
        /// Overwrite existing theme files
        #[arg(long)]
        force: bool,
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

fn main() -> Result<()> {
    // Spawn the real work on a thread with a larger stack size.
    // Windows defaults to a 1MB stack which is insufficient for this binary
    // in debug builds (Linux/macOS default to 8MB). Using 8MB here matches
    // the Linux default and prevents stack overflows on Windows CI.
    const STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB

    let thread = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(run)
        .expect("failed to spawn main thread");

    match thread.join() {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            subcommand,
            name,
            silent,
            force,
            minimal,
            agent,
            provider,
            model,
        } => cmd_init(
            subcommand.as_deref(),
            name,
            silent,
            force,
            minimal,
            agent,
            provider,
            model,
        ),
        Commands::Add {
            description,
            prompt,
            needs_approval,
            template,
            vars,
        } => {
            if let Some(template_name) = template {
                cmd::template::cmd_add_from_template(
                    &template_name,
                    &vars,
                    prompt.as_deref(),
                    needs_approval,
                )
            } else {
                if description.is_empty() {
                    anyhow::bail!(
                        "Description is required when not using --template.\n\n\
                         Usage:\n  \
                         chant add \"description of work\"\n  \
                         chant add --template <name> [--var key=value...]"
                    );
                }
                cmd::spec::cmd_add(&description, prompt.as_deref(), needs_approval)
            }
        }
        Commands::Template { command } => match command {
            TemplateCommands::List => cmd::template::cmd_template_list(),
            TemplateCommands::Show { name } => cmd::template::cmd_template_show(&name),
        },
        Commands::Approve { id, by } => cmd::spec::cmd_approve(&id, &by),
        Commands::Reject { id, by, reason } => cmd::spec::cmd_reject(&id, &by, &reason),
        Commands::List {
            ready,
            label,
            r#type,
            status,
            global,
            repo,
            project,
            approval,
            created_by,
            activity_since,
            mentions,
            count,
            main_only,
        } => cmd::spec::cmd_list(
            ready,
            &label,
            r#type.as_deref(),
            status.as_deref(),
            global,
            repo.as_deref(),
            project.as_deref(),
            approval.as_deref(),
            created_by.as_deref(),
            activity_since.as_deref(),
            mentions.as_deref(),
            count,
            main_only,
        ),
        Commands::Show {
            id,
            body,
            no_render,
        } => cmd::spec::cmd_show(&id, body, no_render),
        Commands::Search {
            query,
            title_only,
            body_only,
            case_sensitive,
            status,
            type_,
            label,
            since,
            until,
            active_only,
            archived_only,
            global,
            repo,
        } => {
            let opts = cmd::search::build_search_options(
                query,
                title_only,
                body_only,
                case_sensitive,
                status,
                type_,
                label,
                since,
                until,
                active_only,
                archived_only,
                global,
                repo.as_deref(),
            )?;
            cmd::search::cmd_search(opts)
        }
        Commands::Work {
            ids,
            prompt,
            branch,
            force,
            parallel,
            label,
            finalize,
            allow_no_commits,
            max_parallel,
            no_cleanup,
            cleanup,
            skip_approval,
            chain,
            chain_max,
            no_merge,
            no_rebase,
        } => cmd::work::cmd_work(
            &ids,
            prompt.as_deref(),
            branch,
            force,
            parallel,
            &label,
            finalize,
            allow_no_commits,
            max_parallel,
            no_cleanup,
            cleanup,
            skip_approval,
            chain,
            chain_max,
            no_merge,
            no_rebase,
        ),
        Commands::Mcp => mcp::run_server(),
        Commands::Status {
            global,
            repo,
            watch,
            brief,
            json,
        } => cmd::spec::cmd_status(global, repo.as_deref(), watch, brief, json),
        Commands::Refresh { verbose } => cmd::refresh::cmd_refresh(verbose),
        Commands::Lint { format, verbose } => {
            let lint_format = match format.to_lowercase().as_str() {
                "json" => cmd::spec::LintFormat::Json,
                "text" => cmd::spec::LintFormat::Text,
                _ => {
                    eprintln!("Error: Invalid format '{}'. Use 'text' or 'json'.", format);
                    std::process::exit(1);
                }
            };
            cmd::spec::cmd_lint(lint_format, verbose)
        }
        Commands::Log {
            id,
            lines,
            no_follow,
        } => cmd::lifecycle::cmd_log(&id, lines, !no_follow),
        Commands::Split {
            id,
            model,
            force,
            recursive,
            max_depth,
        } => cmd::lifecycle::cmd_split(&id, model.as_deref(), force, recursive, max_depth),
        Commands::Archive {
            id,
            dry_run,
            older_than,
            force,
            commit,
            no_commit,
            no_stage,
        } => {
            let should_commit = commit && !no_commit;
            cmd::lifecycle::cmd_archive(
                id.as_deref(),
                dry_run,
                older_than,
                force,
                should_commit,
                no_stage,
            )
        }
        Commands::Merge {
            ids,
            all,
            all_completed,
            list,
            ready,
            interactive,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            rebase,
            auto,
            finalize,
        } => cmd::lifecycle::cmd_merge(
            &ids,
            all,
            all_completed,
            list,
            ready,
            interactive,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            rebase,
            auto,
            finalize,
        ),
        Commands::Diagnose { id } => cmd::lifecycle::cmd_diagnose(&id),
        Commands::Drift { id } => cmd::lifecycle::cmd_drift(id.as_deref()),
        Commands::Resume {
            id,
            work,
            prompt,
            branch,
        } => cmd::lifecycle::cmd_resume(&id, work, prompt.as_deref(), branch),
        Commands::Replay {
            id,
            prompt,
            branch,
            force,
            dry_run,
            yes,
        } => cmd::lifecycle::cmd_replay(&id, prompt.as_deref(), branch, force, dry_run, yes),
        Commands::Cancel {
            id,
            force,
            dry_run,
            yes,
        } => cmd::spec::cmd_cancel(&id, force, dry_run, yes),
        Commands::Delete {
            id,
            force,
            cascade,
            delete_branch,
            dry_run,
            yes,
        } => cmd::spec::cmd_delete(&id, force, cascade, delete_branch, dry_run, yes),
        Commands::Config { validate } => {
            if validate {
                cmd::config::cmd_config_validate()
            } else {
                println!("Usage: chant config --validate");
                Ok(())
            }
        }
        Commands::Version { verbose } => cmd_version(verbose),
        Commands::Export {
            format,
            status,
            type_,
            label,
            ready,
            from,
            to,
            fields,
            output,
        } => cmd::spec::cmd_export(
            format.as_deref(),
            &status,
            type_.as_deref(),
            &label,
            ready,
            from.as_deref(),
            to.as_deref(),
            fields.as_deref(),
            output.as_deref(),
        ),
        Commands::Disk => cmd::disk::cmd_disk(),
        Commands::Activity { by, since, spec } => {
            cmd::activity::cmd_activity(by.as_deref(), since.as_deref(), spec.as_deref())
        }
        Commands::Cleanup {
            dry_run,
            yes,
            worktrees,
        } => cmd::cleanup::cmd_cleanup(dry_run, yes, worktrees),
        Commands::Worktree { command } => match command {
            WorktreeCommands::Status => cmd::worktree::cmd_worktree_status(),
        },
        Commands::Verify {
            id,
            all,
            label,
            exit_code,
            dry_run,
            prompt,
        } => cmd::verify::cmd_verify(
            id.as_deref(),
            all,
            &label,
            exit_code,
            dry_run,
            prompt.as_deref(),
        ),
        Commands::Prep { id, clean } => {
            let specs_dir = cmd::ensure_initialized()?;
            cmd::prep::cmd_prep(&id, clean, &specs_dir)
        }
        Commands::Derive { id, all, dry_run } => cmd::derive::cmd_derive(id, all, dry_run),
        Commands::Finalize { id } => {
            let specs_dir = cmd::ensure_initialized()?;
            cmd::lifecycle::cmd_finalize(&id, &specs_dir)
        }
        Commands::MergeDriver {
            base,
            current,
            other,
        } => cmd_merge_driver(&base, &current, &other),
        Commands::MergeDriverSetup => cmd_merge_driver_setup(),
        Commands::Completion { shell } => cmd_completion(shell),
        Commands::Site { command } => match command {
            SiteCommands::Init { force } => cmd::site::cmd_site_init(force),
            SiteCommands::Build { output } => cmd::site::cmd_site_build(output.as_deref()),
            SiteCommands::Serve { port, output } => {
                cmd::site::cmd_site_serve(port, output.as_deref())
            }
        },
        Commands::Watch {
            once,
            dry_run,
            poll_interval,
        } => cmd::watch::run_watch(once, dry_run, poll_interval),
    }
}

/// Run the git merge driver for spec files
fn cmd_merge_driver(base: &Path, current: &Path, other: &Path) -> Result<()> {
    match chant::merge_driver::run_merge_driver(base, current, other) {
        Ok(true) => {
            // Clean merge
            std::process::exit(0);
        }
        Ok(false) => {
            // Merge with conflicts
            eprintln!("Spec merge completed with conflicts - manual resolution needed for body");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Merge driver error: {}", e);
            std::process::exit(2);
        }
    }
}

/// Set up the merge driver and show status
fn cmd_merge_driver_setup() -> Result<()> {
    match chant::merge_driver::setup_merge_driver() {
        Ok(result) => {
            if result.git_config_set && result.gitattributes_updated {
                println!("{} Merge driver fully configured", "Done!".green());
                println!("  {} Git config updated", "✓".green());
                println!("  {} .gitattributes updated", "✓".green());
            } else if result.git_config_set {
                println!("{} Merge driver configured", "Done!".green());
                println!("  {} Git config updated", "✓".green());
                println!("  {} .gitattributes already configured", "•".cyan());
            } else if result.gitattributes_updated {
                println!("{} Merge driver partially configured", "Done!".yellow());
                println!("  {} .gitattributes updated", "✓".green());
                if let Some(warning) = result.warning {
                    println!("  {} {}", "⚠".yellow(), warning);
                }
            } else {
                println!("{} Merge driver already configured", "ℹ".cyan());
            }
            println!("\n{}", "How it works:".bold());
            println!("  The merge driver automatically resolves spec file conflicts by:");
            println!("  • Preferring the more advanced status (completed > in_progress > pending)");
            println!("  • Merging commit lists without duplicates");
            println!("  • Taking completed_at and model from whichever side has them");
            println!("  • Using 3-way merge for body content (shows conflict markers if needed)");
            Ok(())
        }
        Err(e) => {
            eprintln!("{} Failed to set up merge driver: {}", "Error:".red(), e);
            eprintln!("\n{}", "Manual setup instructions:".bold());
            println!("{}", chant::merge_driver::get_setup_instructions());
            Err(e)
        }
    }
}

/// Generate shell completion script
fn cmd_completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "chant", &mut io::stdout());
    Ok(())
}

/// Parse a provider string into a normalized form
fn parse_provider_string(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "claude" | "claude-cli" => Some("claude"),
        "ollama" | "local" => Some("ollama"),
        "openai" | "gpt" => Some("openai"),
        _ => None,
    }
}

/// Result of writing an agent config file
#[derive(Debug)]
enum AgentFileResult {
    /// File was created new
    Created,
    /// Existing file was updated (section injected/replaced)
    Updated,
    /// File was skipped (user declined or non-TTY)
    Skipped,
    /// File was unchanged (already up-to-date)
    Unchanged,
}

/// Write agent configuration file, using section injection for Claude's CLAUDE.md
///
/// For Claude provider: Uses section injection to preserve existing CLAUDE.md content
/// For other providers: Uses full template replacement
fn write_agent_config_file(
    provider: &templates::AgentProvider,
    template: &templates::AgentTemplate,
    target_path: &Path,
    force: bool,
    has_mcp: bool,
) -> Result<AgentFileResult> {
    // For Claude provider, use section injection to preserve user content
    if *provider == templates::AgentProvider::Claude {
        let existing_content = if target_path.exists() {
            Some(std::fs::read_to_string(target_path)?)
        } else {
            None
        };

        let result = templates::inject_chant_section(existing_content.as_deref(), has_mcp);

        match result {
            templates::InjectionResult::Created(content) => {
                std::fs::write(target_path, content)?;
                return Ok(AgentFileResult::Created);
            }
            templates::InjectionResult::Appended(content) => {
                std::fs::write(target_path, content)?;
                return Ok(AgentFileResult::Updated);
            }
            templates::InjectionResult::Replaced(content) => {
                std::fs::write(target_path, content)?;
                return Ok(AgentFileResult::Updated);
            }
            templates::InjectionResult::Unchanged => {
                return Ok(AgentFileResult::Unchanged);
            }
        }
    }

    // For non-Claude providers, use full template replacement
    if target_path.exists() && !force {
        if atty::is(atty::Stream::Stdin) {
            let should_overwrite = dialoguer::Confirm::new()
                .with_prompt(format!(
                    "{} already exists. Overwrite?",
                    target_path.display()
                ))
                .default(false)
                .interact()?;

            if !should_overwrite {
                return Ok(AgentFileResult::Skipped);
            }
        } else {
            eprintln!(
                "{} {} already exists. Use {} to overwrite.",
                "•".yellow(),
                target_path.display(),
                "--force".cyan()
            );
            return Ok(AgentFileResult::Skipped);
        }
    }

    // Write the full template for non-Claude providers
    if let Some(parent) = target_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(target_path, template.content)?;

    if target_path.exists() && force {
        Ok(AgentFileResult::Updated)
    } else {
        Ok(AgentFileResult::Created)
    }
}

/// Handle updating only agent configuration files (used for re-running init with --agent)
fn handle_agent_update(chant_dir: &Path, agents: &[String], force: bool) -> Result<()> {
    let parsed_agents = templates::parse_agent_providers(agents)?;

    if parsed_agents.is_empty() {
        println!("{}", "No agents specified.".yellow());
        return Ok(());
    }

    // Create agents directory
    std::fs::create_dir_all(chant_dir.join("agents"))?;

    // Check if MCP is configured (affects which chant section template to use)
    let has_mcp = PathBuf::from(".mcp.json").exists();

    let mut created_agents = Vec::new();
    let mut updated_agents = Vec::new();
    let mut unchanged_agents = Vec::new();

    for provider in &parsed_agents {
        let template = templates::get_template(provider.as_str())?;

        // Determine the target path based on provider
        let target_path = match provider.config_filename() {
            ".amazonq/rules.md" => {
                std::fs::create_dir_all(".amazonq")?;
                PathBuf::from(".amazonq/rules.md")
            }
            filename => PathBuf::from(filename),
        };

        // Write the agent config file using the helper
        let result = write_agent_config_file(provider, &template, &target_path, force, has_mcp)?;

        match result {
            AgentFileResult::Created => {
                created_agents.push((target_path, provider.as_str()));
            }
            AgentFileResult::Updated => {
                updated_agents.push((target_path, provider.as_str()));
            }
            AgentFileResult::Unchanged => {
                unchanged_agents.push((target_path, provider.as_str()));
            }
            AgentFileResult::Skipped => {
                // Already logged in write_agent_config_file
            }
        }
    }

    // Report results
    for (target_path, _) in &created_agents {
        println!("{} {}", "Created".green(), target_path.display());
    }
    for (target_path, _) in &updated_agents {
        println!("{} {}", "Updated".green(), target_path.display());
    }
    for (target_path, _) in &unchanged_agents {
        println!(
            "{} {} (already up-to-date)",
            "•".cyan(),
            target_path.display()
        );
    }

    let all_modified: Vec<_> = created_agents
        .iter()
        .chain(updated_agents.iter())
        .map(|(_, name)| *name)
        .collect();

    if all_modified.is_empty() && unchanged_agents.is_empty() {
        println!("{}", "No agent files were updated.".yellow());
    } else if !all_modified.is_empty() {
        let agent_names = all_modified.join(", ");
        println!(
            "{} Agent configuration updated for: {}",
            "✓".green(),
            agent_names.cyan()
        );
    }

    // Create MCP config if any provider supports it
    let mut mcp_created = false;
    for provider in &parsed_agents {
        if provider.mcp_config_filename().is_some() {
            // Update global ~/.claude/mcp.json (actually used by Claude Code)
            match update_claude_mcp_config() {
                Ok(result) => {
                    if result.created {
                        println!(
                            "{} Created {} with chant MCP server",
                            "✓".green(),
                            result.path.display()
                        );
                    } else if result.updated {
                        println!(
                            "{} Added chant MCP server to {}",
                            "✓".green(),
                            result.path.display()
                        );
                    } else {
                        println!(
                            "{} Updated chant MCP server in {}",
                            "✓".green(),
                            result.path.display()
                        );
                    }
                    if let Some(warning) = result.warning {
                        eprintln!("{} {}", "Warning:".yellow(), warning);
                    }
                    mcp_created = true;
                }
                Err(e) => {
                    eprintln!("{} Failed to update global MCP config: {}", "✗".red(), e);
                }
            }

            // Also create project-local .mcp.json as reference
            let mcp_path = PathBuf::from(".mcp.json");
            if !mcp_path.exists() || force {
                let mcp_config = r#"{
  "mcpServers": {
    "chant": {
      "type": "stdio",
      "command": "chant",
      "args": ["mcp"]
    }
  }
}
"#;
                if let Err(e) = std::fs::write(&mcp_path, mcp_config) {
                    // Project-local write failure is non-critical
                    eprintln!(
                        "{} Could not create {} (reference copy): {}",
                        "•".yellow(),
                        mcp_path.display(),
                        e
                    );
                } else {
                    println!(
                        "{} {} (reference copy)",
                        "Created".green(),
                        mcp_path.display()
                    );
                }
            }

            if mcp_created {
                println!(
                    "{} Restart Claude Code to activate MCP integration",
                    "ℹ".cyan()
                );
            }
            break; // Only create one MCP config file
        }
    }

    Ok(())
}

/// Update config.md with surgical changes to specific fields
fn update_config_field(config_path: &Path, field: &str, value: &str) -> Result<()> {
    let content = std::fs::read_to_string(config_path)?;

    // Split into frontmatter and body
    let (frontmatter_opt, body) = chant::spec::split_frontmatter(&content);
    let frontmatter = frontmatter_opt.ok_or_else(|| anyhow::anyhow!("No frontmatter found"))?;

    // Parse YAML into a Value for manipulation
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&frontmatter)?;

    // Navigate to the appropriate field and update it
    match field {
        "provider" => {
            // Ensure defaults section exists
            if yaml.get("defaults").is_none() {
                yaml["defaults"] = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
            }
            yaml["defaults"]["provider"] = serde_yaml::Value::String(value.to_string());
        }
        "model" => {
            // Ensure defaults section exists
            if yaml.get("defaults").is_none() {
                yaml["defaults"] = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
            }
            yaml["defaults"]["model"] = serde_yaml::Value::String(value.to_string());
        }
        _ => anyhow::bail!("Unknown field: {}", field),
    }

    // Serialize back to YAML
    let new_frontmatter = serde_yaml::to_string(&yaml)?;

    // Reconstruct the file content
    let new_content = format!("---\n{}---\n{}", new_frontmatter, body);
    std::fs::write(config_path, new_content)?;

    Ok(())
}

/// Result of updating the global Claude MCP config
#[derive(Debug)]
struct McpConfigResult {
    /// Whether the global config was created (new file)
    created: bool,
    /// Whether the global config was updated (existing file merged)
    updated: bool,
    /// Path to the global config file
    path: PathBuf,
    /// Warning message if something went wrong but we recovered
    warning: Option<String>,
}

/// Update the global Claude MCP config at ~/.claude/mcp.json
///
/// This function:
/// - Creates ~/.claude/ directory if it doesn't exist
/// - Creates a new mcp.json if it doesn't exist
/// - Merges with existing mcp.json without overwriting other servers
/// - Creates a backup if the existing file has invalid JSON
fn update_claude_mcp_config() -> Result<McpConfigResult> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let global_mcp_path = home_dir.join(".claude").join("mcp.json");

    // Ensure ~/.claude/ directory exists
    if let Some(parent) = global_mcp_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!("Failed to create directory {}: {}", parent.display(), e)
        })?;
    }

    // Define the chant MCP server config
    let chant_server = serde_json::json!({
        "type": "stdio",
        "command": "chant",
        "args": ["mcp"]
    });

    // Read existing config if it exists
    let (mut config, is_new, warning) = if global_mcp_path.exists() {
        let content = std::fs::read_to_string(&global_mcp_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", global_mcp_path.display(), e))?;

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(parsed) => (parsed, false, None),
            Err(_) => {
                // Invalid JSON - create backup and show manual instructions
                let backup_path = home_dir.join(".claude").join("mcp.json.backup");
                std::fs::copy(&global_mcp_path, &backup_path)?;

                // Print the manual instructions
                eprintln!(
                    "{} Could not parse existing {}",
                    "✗".red(),
                    global_mcp_path.display()
                );
                eprintln!("{} Please manually add the chant MCP server:", "→".cyan());
                eprintln!();
                eprintln!(
                    r#"{{
  "mcpServers": {{
    "chant": {{
      "type": "stdio",
      "command": "chant",
      "args": ["mcp"]
    }}
  }}
}}"#
                );
                eprintln!();
                eprintln!("{} Backup saved to: {}", "ℹ".cyan(), backup_path.display());

                // Start fresh with a new config
                let warning_msg = format!(
                    "Existing {} had invalid JSON. Backup saved to {}",
                    global_mcp_path.display(),
                    backup_path.display()
                );
                (
                    serde_json::json!({
                        "mcpServers": {}
                    }),
                    true,
                    Some(warning_msg),
                )
            }
        }
    } else {
        // Create new config structure
        (
            serde_json::json!({
                "mcpServers": {}
            }),
            true,
            None,
        )
    };

    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    // Check if chant server already exists (for reporting purposes)
    let already_had_chant = config
        .get("mcpServers")
        .and_then(|s| s.get("chant"))
        .is_some();

    // Add or update chant MCP server
    if let Some(servers) = config.get_mut("mcpServers") {
        servers["chant"] = chant_server;
    }

    // Write updated config
    let formatted = serde_json::to_string_pretty(&config)
        .map_err(|e| anyhow::anyhow!("Failed to serialize MCP config: {}", e))?;
    std::fs::write(&global_mcp_path, formatted).map_err(|e| {
        anyhow::anyhow!(
            "Failed to write {}: {} (permission denied?)",
            global_mcp_path.display(),
            e
        )
    })?;

    Ok(McpConfigResult {
        created: is_new,
        updated: !is_new && !already_had_chant,
        path: global_mcp_path,
        warning,
    })
}

#[allow(clippy::too_many_arguments)]
fn cmd_init(
    subcommand: Option<&str>,
    name: Option<String>,
    silent: bool,
    force: bool,
    minimal: bool,
    agents: Vec<String>,
    provider: Option<String>,
    model: Option<String>,
) -> Result<()> {
    // Handle 'prompts' subcommand for upgrading prompts on existing projects
    if let Some("prompts") = subcommand {
        let chant_dir = PathBuf::from(".chant");
        if !chant_dir.exists() {
            anyhow::bail!("Chant not initialized. Run 'chant init' first.");
        }
        write_bundled_prompts(&chant_dir)?;
        println!("{} Prompts updated.", "Done!".green());
        return Ok(());
    }

    let chant_dir = PathBuf::from(".chant");
    let config_path = chant_dir.join("config.md");

    // Check if this is an existing project
    let already_initialized = chant_dir.exists() && config_path.exists();

    // For existing projects with --silent flag: validate git tracking status first
    // This is checked here because we may return early below for surgical updates
    if already_initialized && silent {
        let ls_output = std::process::Command::new("git")
            .args(["ls-files", "--error-unmatch", ".chant/config.md"])
            .output();

        if let Ok(output) = ls_output {
            if output.status.success() {
                anyhow::bail!(
                    "Cannot enable silent mode: .chant/ is already tracked in git. \
                     Silent mode requires .chant/ to be local-only. \
                     Either remove .chant/ from git tracking or initialize without --silent."
                );
            }
        }
    }

    // Handle surgical updates for existing projects (--provider or --model flags only)
    if already_initialized && !force {
        // Check if this is a surgical update (only --provider or --model specified)
        let is_surgical_provider =
            provider.is_some() && name.is_none() && agents.is_empty() && model.is_none();
        let is_surgical_model =
            model.is_some() && name.is_none() && agents.is_empty() && provider.is_none();
        let is_surgical_both =
            provider.is_some() && model.is_some() && name.is_none() && agents.is_empty();
        let is_agent_only =
            !agents.is_empty() && name.is_none() && provider.is_none() && model.is_none();

        if is_surgical_provider || is_surgical_model || is_surgical_both {
            // Surgical config update
            if let Some(ref prov) = provider {
                let normalized = parse_provider_string(prov).ok_or_else(|| {
                    anyhow::anyhow!("Invalid provider: {}. Use claude, ollama, or openai.", prov)
                })?;
                update_config_field(&config_path, "provider", normalized)?;
                println!("{} Updated provider to: {}", "✓".green(), normalized.cyan());
            }
            if let Some(ref m) = model {
                update_config_field(&config_path, "model", m)?;
                println!("{} Updated model to: {}", "✓".green(), m.cyan());
            }
            return Ok(());
        }

        if is_agent_only {
            // Only update agent files, don't touch config
            return handle_agent_update(&chant_dir, &agents, force);
        }

        // No specific flags - show configuration menu in TTY mode
        if atty::is(atty::Stream::Stdin)
            && name.is_none()
            && !silent
            && !minimal
            && agents.is_empty()
            && provider.is_none()
            && model.is_none()
        {
            // Read current project name from config
            let current_name = if let Ok(config) = chant::config::Config::load() {
                config.project.name
            } else {
                "unknown".to_string()
            };

            println!(
                "\n{} {}",
                "Chant already initialized for:".cyan(),
                current_name.bold()
            );

            let config_options = vec![
                "Add/update agent configuration",
                "Change default model provider",
                "Change default model",
                "Exit (no changes)",
            ];

            let selection = dialoguer::Select::new()
                .with_prompt("What would you like to configure?")
                .items(&config_options)
                .default(3)
                .interact()?;

            match selection {
                0 => {
                    // Add/update agent configuration
                    let agent_options = vec![
                        "Claude Code (CLAUDE.md)",
                        "Cursor (.cursorrules)",
                        "Amazon Q (.amazonq/rules.md)",
                        "Generic (.ai-instructions)",
                        "All of the above",
                    ];

                    let agent_selection = dialoguer::Select::new()
                        .with_prompt("Which agent configuration?")
                        .items(&agent_options)
                        .default(0)
                        .interact()?;

                    let selected_agents = match agent_selection {
                        0 => vec!["claude".to_string()],
                        1 => vec!["cursor".to_string()],
                        2 => vec!["amazonq".to_string()],
                        3 => vec!["generic".to_string()],
                        4 => vec!["all".to_string()],
                        _ => vec![],
                    };

                    return handle_agent_update(&chant_dir, &selected_agents, force);
                }
                1 => {
                    // Change default model provider
                    let provider_options =
                        vec!["Claude CLI (recommended)", "Ollama (local)", "OpenAI API"];

                    let provider_selection = dialoguer::Select::new()
                        .with_prompt("Default model provider?")
                        .items(&provider_options)
                        .default(0)
                        .interact()?;

                    let selected_provider = match provider_selection {
                        0 => "claude",
                        1 => "ollama",
                        2 => "openai",
                        _ => "claude",
                    };

                    update_config_field(&config_path, "provider", selected_provider)?;
                    println!(
                        "{} Updated provider to: {}",
                        "✓".green(),
                        selected_provider.cyan()
                    );
                    return Ok(());
                }
                2 => {
                    // Change default model
                    let model_options = vec![
                        "opus (most capable)",
                        "sonnet (balanced)",
                        "haiku (fastest)",
                        "Custom model name",
                    ];

                    let model_selection = dialoguer::Select::new()
                        .with_prompt("Default model?")
                        .items(&model_options)
                        .default(1)
                        .interact()?;

                    let selected_model = match model_selection {
                        0 => "claude-opus-4".to_string(),
                        1 => "claude-sonnet-4".to_string(),
                        2 => "claude-haiku-4".to_string(),
                        3 => dialoguer::Input::new()
                            .with_prompt("Custom model name")
                            .interact_text()?,
                        _ => "claude-sonnet-4".to_string(),
                    };

                    update_config_field(&config_path, "model", &selected_model)?;
                    println!(
                        "{} Updated model to: {}",
                        "✓".green(),
                        selected_model.cyan()
                    );
                    return Ok(());
                }
                _ => {
                    println!("{}", "No changes made.".yellow());
                    return Ok(());
                }
            }
        }

        // Non-TTY mode without specific flags
        println!("{}", "Chant already initialized.".yellow());
        return Ok(());
    }

    // Detect if we're in wizard mode (no flags provided for fresh init)
    let is_wizard_mode = name.is_none()
        && !silent
        && !force
        && !minimal
        && agents.is_empty()
        && provider.is_none()
        && model.is_none();

    // Gather parameters - either from wizard or from flags
    let (final_name, final_silent, final_minimal, final_agents, final_provider, final_model) =
        if is_wizard_mode && atty::is(atty::Stream::Stdin) {
            // Detect default project name for wizard
            let detected_name = detect_project_name().unwrap_or_else(|| "my-project".to_string());

            // Prompt for project name
            let project_name = dialoguer::Input::new()
                .with_prompt("Project name")
                .default(detected_name.clone())
                .interact_text()?;

            // Prompt for prompt templates
            let include_templates = dialoguer::Confirm::new()
                .with_prompt("Include prompt templates?")
                .default(true)
                .interact()?;

            // Prompt for silent mode
            let enable_silent = dialoguer::Confirm::new()
                .with_prompt("Keep .chant/ local only (gitignored)?")
                .default(false)
                .interact()?;

            // Prompt for model provider
            let provider_options = vec!["Claude CLI (recommended)", "Ollama (local)", "OpenAI API"];

            let provider_selection = dialoguer::Select::new()
                .with_prompt("Default model provider?")
                .items(&provider_options)
                .default(0)
                .interact()?;

            let selected_provider = match provider_selection {
                0 => Some("claude".to_string()),
                1 => Some("ollama".to_string()),
                2 => Some("openai".to_string()),
                _ => None,
            };

            // Prompt for default model
            let model_options = vec![
                "opus (most capable)",
                "sonnet (balanced)",
                "haiku (fastest)",
                "Custom model name",
                "None (use provider default)",
            ];

            let model_selection = dialoguer::Select::new()
                .with_prompt("Default model?")
                .items(&model_options)
                .default(1)
                .interact()?;

            let selected_model = match model_selection {
                0 => Some("claude-opus-4".to_string()),
                1 => Some("claude-sonnet-4".to_string()),
                2 => Some("claude-haiku-4".to_string()),
                3 => {
                    let custom: String = dialoguer::Input::new()
                        .with_prompt("Custom model name")
                        .interact_text()?;
                    Some(custom)
                }
                _ => None,
            };

            // Prompt for agent configuration
            let agent_options = vec![
                "None",
                "Claude Code (CLAUDE.md)",
                "Cursor (.cursorrules)",
                "Amazon Q (.amazonq/rules.md)",
                "Generic (.ai-instructions)",
                "All of the above",
            ];

            let agent_selection = dialoguer::Select::new()
                .with_prompt("Initialize agent configuration?")
                .items(&agent_options)
                .default(0)
                .interact()?;

            let selected_agents = match agent_selection {
                0 => vec![], // None
                1 => vec!["claude".to_string()],
                2 => vec!["cursor".to_string()],
                3 => vec!["amazonq".to_string()],
                4 => vec!["generic".to_string()],
                5 => vec!["all".to_string()],
                _ => vec![],
            };

            (
                project_name,
                enable_silent,
                !include_templates, // invert: minimal is "no templates"
                selected_agents,
                selected_provider,
                selected_model,
            )
        } else {
            // Direct mode: use provided values
            let project_name = name.unwrap_or_else(|| {
                detect_project_name().unwrap_or_else(|| "my-project".to_string())
            });

            // Validate provider if specified
            let validated_provider = if let Some(ref p) = provider {
                let normalized = parse_provider_string(p).ok_or_else(|| {
                    anyhow::anyhow!("Invalid provider: {}. Use claude, ollama, or openai.", p)
                })?;
                Some(normalized.to_string())
            } else {
                None
            };

            (
                project_name,
                silent,
                minimal,
                agents,
                validated_provider,
                model,
            )
        };

    // For silent mode: validate that .chant/ is not already tracked in git
    // Do this check BEFORE the exists check so we catch tracking issues even if dir exists
    if final_silent {
        let ls_output = std::process::Command::new("git")
            .args(["ls-files", "--error-unmatch", ".chant/config.md"])
            .output();

        if let Ok(output) = ls_output {
            if output.status.success() {
                anyhow::bail!(
                    "Cannot enable silent mode: .chant/ is already tracked in git. \
                     Silent mode requires .chant/ to be local-only. \
                     Either remove .chant/ from git tracking or initialize without --silent."
                );
            }
        }
    }

    if chant_dir.exists() && force {
        // force flag: do full reinitialization (preserve specs/config)
        let specs_backup = chant_dir.join("specs");
        let config_backup = chant_dir.join("config.md");
        let prompts_backup = chant_dir.join("prompts");
        let gitignore_backup = chant_dir.join(".gitignore");
        let locks_backup = chant_dir.join(".locks");
        let store_backup = chant_dir.join(".store");

        // Check which directories exist before deletion
        let has_specs = specs_backup.exists();
        let has_config = config_backup.exists();
        let has_prompts = prompts_backup.exists();
        let has_gitignore = gitignore_backup.exists();
        let has_locks = locks_backup.exists();
        let has_store = store_backup.exists();

        // Temporarily move important files
        let temp_dir = PathBuf::from(".chant_temp_backup");
        std::fs::create_dir_all(&temp_dir)?;

        if has_specs {
            std::fs::rename(&specs_backup, temp_dir.join("specs"))?;
        }
        if has_config {
            std::fs::rename(&config_backup, temp_dir.join("config.md"))?;
        }
        if has_prompts {
            std::fs::rename(&prompts_backup, temp_dir.join("prompts"))?;
        }
        if has_gitignore {
            std::fs::rename(&gitignore_backup, temp_dir.join(".gitignore"))?;
        }
        if has_locks {
            std::fs::rename(&locks_backup, temp_dir.join(".locks"))?;
        }
        if has_store {
            std::fs::rename(&store_backup, temp_dir.join(".store"))?;
        }

        // Remove the old .chant directory
        std::fs::remove_dir_all(&chant_dir)?;

        // Create fresh directory structure
        std::fs::create_dir_all(chant_dir.join("specs"))?;
        std::fs::create_dir_all(chant_dir.join("prompts"))?;
        std::fs::create_dir_all(chant_dir.join(".locks"))?;
        std::fs::create_dir_all(chant_dir.join(".store"))?;

        // Restore backed-up files
        if has_specs {
            std::fs::rename(temp_dir.join("specs"), chant_dir.join("specs"))?;
        }
        if has_config {
            std::fs::rename(temp_dir.join("config.md"), chant_dir.join("config.md"))?;
        }
        if has_prompts {
            std::fs::rename(temp_dir.join("prompts"), chant_dir.join("prompts"))?;
        }
        if has_gitignore {
            std::fs::rename(temp_dir.join(".gitignore"), chant_dir.join(".gitignore"))?;
        }
        if has_locks {
            std::fs::rename(temp_dir.join(".locks"), chant_dir.join(".locks"))?;
        }
        if has_store {
            std::fs::rename(temp_dir.join(".store"), chant_dir.join(".store"))?;
        }

        // Clean up temp directory
        let _ = std::fs::remove_dir(&temp_dir);
    }

    // Detect project name
    let project_name = final_name;

    // Create directory structure (only if not already created during force/restore)
    std::fs::create_dir_all(chant_dir.join("specs"))?;
    std::fs::create_dir_all(chant_dir.join("prompts"))?;
    std::fs::create_dir_all(chant_dir.join(".locks"))?;
    std::fs::create_dir_all(chant_dir.join(".store"))?;

    // Create config.md only if it doesn't exist (preserve during --force)
    let config_path = chant_dir.join("config.md");
    if !config_path.exists() {
        // Build defaults section with optional provider and model
        let mut defaults_lines = vec![
            "  prompt: standard".to_string(),
            "  branch: false".to_string(),
        ];
        if let Some(ref prov) = final_provider {
            defaults_lines.push(format!("  provider: {}", prov));
        }
        if let Some(ref m) = final_model {
            defaults_lines.push(format!("  model: {}", m));
        }

        let config_content = format!(
            r#"---
project:
  name: {}

defaults:
{}
---

# Chant Configuration

Project initialized on {}.
"#,
            project_name,
            defaults_lines.join("\n"),
            chrono::Local::now().format("%Y-%m-%d")
        );
        std::fs::write(&config_path, config_content)?;
    }

    if !final_minimal {
        // Write bundled prompts to .chant/prompts/ (only if they don't exist)
        // This ensures existing customizations are preserved
        write_bundled_prompts(&chant_dir)?;
    }

    // Create .gitignore (only if it doesn't exist)
    let gitignore_path = chant_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let gitignore_content = "# Local state (not shared)\n.locks/\n.store/\nstore/\nlogs/\n\n# Agent configuration (contains API keys, not shared)\nagents.md\n";
        std::fs::write(&gitignore_path, gitignore_content)?;
    }

    // Set up the merge driver for spec files (handles .gitattributes and git config)
    // This ensures branch mode works correctly by auto-resolving frontmatter conflicts
    let merge_driver_result = chant::merge_driver::setup_merge_driver();
    let merge_driver_warning = match &merge_driver_result {
        Ok(result) => result.warning.clone(),
        Err(e) => Some(format!("Failed to set up merge driver: {}", e)),
    };

    // Handle silent mode: add .chant/ to .git/info/exclude
    if final_silent {
        // Get git common dir (supports worktrees)
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--git-common-dir"])
            .output()?;

        if output.status.success() {
            let git_dir = String::from_utf8(output.stdout)?.trim().to_string();
            let exclude_path = PathBuf::from(&git_dir).join("info/exclude");

            // Create info directory if it doesn't exist
            std::fs::create_dir_all(exclude_path.parent().unwrap())?;

            // Read existing exclude file
            let mut exclude_content = std::fs::read_to_string(&exclude_path).unwrap_or_default();

            // Add .chant/ if not already present
            if !exclude_content.contains(".chant/") && !exclude_content.contains(".chant") {
                if !exclude_content.ends_with('\n') && !exclude_content.is_empty() {
                    exclude_content.push('\n');
                }
                exclude_content.push_str(".chant/\n");
                std::fs::write(&exclude_path, exclude_content)?;
            }
        }
    }

    // Handle agent configuration if specified
    let parsed_agents = templates::parse_agent_providers(&final_agents)?;
    let mut created_agents = Vec::new();
    let mut updated_agents = Vec::new();
    let mut unchanged_agents = Vec::new();
    if !parsed_agents.is_empty() {
        // Create agents directory
        std::fs::create_dir_all(chant_dir.join("agents"))?;

        // Check if MCP will be created (affects which chant section template to use)
        // MCP is created for Claude provider, so if Claude is in the list, we'll have MCP
        let will_have_mcp = parsed_agents
            .iter()
            .any(|p| p.mcp_config_filename().is_some())
            || PathBuf::from(".mcp.json").exists();

        // Create agent configuration files for each provider
        for provider in &parsed_agents {
            let template = templates::get_template(provider.as_str())?;

            // Determine the target path based on provider
            let target_path = match provider.config_filename() {
                ".amazonq/rules.md" => {
                    // Create .amazonq directory in root
                    std::fs::create_dir_all(".amazonq")?;
                    PathBuf::from(".amazonq/rules.md")
                }
                filename => {
                    // Other providers: write to root
                    PathBuf::from(filename)
                }
            };

            // Write the agent config file using the helper
            let result =
                write_agent_config_file(provider, &template, &target_path, force, will_have_mcp)?;

            match result {
                AgentFileResult::Created => {
                    created_agents.push((target_path, provider.as_str()));
                }
                AgentFileResult::Updated => {
                    updated_agents.push((target_path, provider.as_str()));
                }
                AgentFileResult::Unchanged => {
                    unchanged_agents.push((target_path, provider.as_str()));
                }
                AgentFileResult::Skipped => {
                    // Already logged in write_agent_config_file
                }
            }
        }

        // Create MCP config if any provider supports it
        let mut mcp_created = false;
        for provider in &parsed_agents {
            if provider.mcp_config_filename().is_some() {
                // Update global ~/.claude/mcp.json (actually used by Claude Code)
                match update_claude_mcp_config() {
                    Ok(result) => {
                        if result.created {
                            println!(
                                "{} Created {} with chant MCP server",
                                "✓".green(),
                                result.path.display()
                            );
                        } else if result.updated {
                            println!(
                                "{} Added chant MCP server to {}",
                                "✓".green(),
                                result.path.display()
                            );
                        } else {
                            println!(
                                "{} Updated chant MCP server in {}",
                                "✓".green(),
                                result.path.display()
                            );
                        }
                        if let Some(warning) = result.warning {
                            eprintln!("{} {}", "Warning:".yellow(), warning);
                        }
                        mcp_created = true;
                    }
                    Err(e) => {
                        eprintln!("{} Failed to update global MCP config: {}", "✗".red(), e);
                    }
                }

                // Also create project-local .mcp.json as reference
                let mcp_path = PathBuf::from(".mcp.json");
                if !mcp_path.exists() || force {
                    let mcp_config = r#"{
  "mcpServers": {
    "chant": {
      "type": "stdio",
      "command": "chant",
      "args": ["mcp"]
    }
  }
}
"#;
                    if let Err(e) = std::fs::write(&mcp_path, mcp_config) {
                        // Project-local write failure is non-critical
                        eprintln!(
                            "{} Could not create {} (reference copy): {}",
                            "•".yellow(),
                            mcp_path.display(),
                            e
                        );
                    } else {
                        println!(
                            "{} {} (reference copy)",
                            "Created".green(),
                            mcp_path.display()
                        );
                    }
                }

                if mcp_created {
                    println!(
                        "{} Restart Claude Code to activate MCP integration",
                        "ℹ".cyan()
                    );
                }
                break; // Only create one MCP config file
            }
        }
    }

    println!("{} .chant/config.md", "Created".green());
    if !final_minimal {
        println!("{} .chant/prompts/standard.md", "Created".green());
        println!("{} .chant/prompts/split.md", "Created".green());
        println!("{} .chant/prompts/verify.md", "Created".green());
    }
    println!("{} .chant/specs/", "Created".green());

    // Print agent files that were created/updated
    for (target_path, _) in &created_agents {
        println!("{} {}", "Created".green(), target_path.display());
    }
    for (target_path, _) in &updated_agents {
        println!("{} {}", "Updated".green(), target_path.display());
    }
    for (target_path, _) in &unchanged_agents {
        println!(
            "{} {} (already up-to-date)",
            "•".cyan(),
            target_path.display()
        );
    }

    println!("\nChant initialized for project: {}", project_name.cyan());

    // Show provider and model settings
    if let Some(ref prov) = final_provider {
        println!("{} Default provider: {}", "ℹ".cyan(), prov.cyan());
    }
    if let Some(ref m) = final_model {
        println!("{} Default model: {}", "ℹ".cyan(), m.cyan());
    }

    if final_silent {
        println!(
            "{} Silent mode enabled - .chant/ is local-only (not tracked in git)",
            "ℹ".cyan()
        );
        println!(
            "  {} Specs won't be committed to the repository",
            "•".cyan()
        );
        println!(
            "  {} Use {} to convert to shared mode",
            "•".cyan(),
            "--force".cyan()
        );
    }
    if final_minimal {
        println!(
            "{} Minimal mode enabled - only config.md created",
            "ℹ".cyan()
        );
    }

    // Summarize agent configuration changes
    let all_modified: Vec<_> = created_agents
        .iter()
        .chain(updated_agents.iter())
        .map(|(_, name)| *name)
        .collect();

    if !all_modified.is_empty() {
        let agent_names = all_modified.join(", ");
        println!(
            "{} Agent configuration created/updated for: {}",
            "ℹ".cyan(),
            agent_names.cyan()
        );
    }

    // Show merge driver setup status
    if let Some(warning) = merge_driver_warning {
        eprintln!("{} Merge driver: {}", "Warning:".yellow(), warning);
        eprintln!(
            "  {} Run {} for manual setup instructions",
            "•".yellow(),
            "chant merge-driver-setup".cyan()
        );
    } else if let Ok(result) = merge_driver_result {
        if result.git_config_set {
            println!(
                "{} Merge driver configured (auto-resolves spec file conflicts)",
                "ℹ".cyan()
            );
        }
    }

    if is_wizard_mode {
        println!(
            "\n{} Run 'chant add \"description\"' to create your first spec.",
            "Done!".green()
        );
    }

    Ok(())
}

/// Write bundled prompts to .chant/prompts/ directory
///
/// Only writes prompts that don't already exist, preserving any user customizations.
fn write_bundled_prompts(chant_dir: &std::path::Path) -> Result<()> {
    use chant::prompts;

    for prompt in prompts::all_bundled_prompts() {
        let prompt_path = chant_dir
            .join("prompts")
            .join(format!("{}.md", prompt.name));

        // Only write if the file doesn't exist (preserve user customizations)
        if !prompt_path.exists() {
            std::fs::write(&prompt_path, prompt.content)?;
        }
    }

    Ok(())
}

fn detect_project_name() -> Option<String> {
    // Try package.json
    if let Ok(content) = std::fs::read_to_string("package.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
                return Some(name.to_string());
            }
        }
    }

    // Try Cargo.toml
    if let Ok(content) = std::fs::read_to_string("Cargo.toml") {
        for line in content.lines() {
            if line.starts_with("name") {
                if let Some(name) = line.split('=').nth(1) {
                    return Some(name.trim().trim_matches('"').to_string());
                }
            }
        }
    }

    // Try go.mod
    if let Ok(content) = std::fs::read_to_string("go.mod") {
        if let Some(line) = content.lines().next() {
            if line.starts_with("module") {
                if let Some(module) = line.split_whitespace().nth(1) {
                    // Get last segment of module path
                    return Some(module.rsplit('/').next().unwrap_or(module).to_string());
                }
            }
        }
    }

    // Fallback to directory name
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
}

fn cmd_version(verbose: bool) -> Result<()> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("chant {}", VERSION);

    if verbose {
        const GIT_SHA: &str = env!("GIT_SHA");
        const BUILD_DATE: &str = env!("BUILD_DATE");
        println!("commit: {}", GIT_SHA);
        println!("built: {}", BUILD_DATE);
    }

    Ok(())
}

/// Result of log file lookup (used in tests)
#[cfg(test)]
#[derive(Debug)]
enum LogLookupResult {
    /// Log file exists at the given path
    Found(PathBuf),
    /// Log file not found for the spec
    NotFound { spec_id: String, log_path: PathBuf },
}

/// Look up the log file for a spec (used for testing)
#[cfg(test)]
fn lookup_log_file(base_path: &std::path::Path, id: &str) -> anyhow::Result<LogLookupResult> {
    let specs_dir = base_path.join("specs");
    let logs_dir = base_path.join("logs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    let spec = chant::spec::resolve_spec(&specs_dir, id)?;
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if log_path.exists() {
        Ok(LogLookupResult::Found(log_path))
    } else {
        Ok(LogLookupResult::NotFound {
            spec_id: spec.id,
            log_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[serial_test::serial]
    fn test_init_direct_mode_with_name() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // Test direct mode with --name flag
            let result = cmd_init(
                None,
                Some("test-project".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );

            assert!(result.is_ok());
            assert!(temp_dir.path().join(".chant/config.md").exists());
            assert!(temp_dir.path().join(".chant/prompts/standard.md").exists());
            assert!(temp_dir.path().join(".chant/specs").exists());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_direct_mode_minimal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // Test direct mode with --minimal flag
            let result = cmd_init(
                None,
                Some("minimal-project".to_string()),
                false,
                false,
                true,
                vec![],
                None,
                None,
            );

            assert!(result.is_ok());
            assert!(temp_dir.path().join(".chant/config.md").exists());
            // Minimal mode should not create prompt templates
            assert!(!temp_dir.path().join(".chant/prompts/standard.md").exists());
            assert!(temp_dir.path().join(".chant/specs").exists());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_direct_mode_with_agent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // Test direct mode with --agent flag
            let result = cmd_init(
                None,
                Some("agent-project".to_string()),
                false,
                false,
                false,
                vec!["claude".to_string()],
                None,
                None,
            );

            assert!(result.is_ok());
            assert!(temp_dir.path().join(".chant/config.md").exists());
            assert!(temp_dir.path().join("CLAUDE.md").exists());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_prevents_duplicate() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // First init
            let result1 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );
            assert!(result1.is_ok());

            // Verify files were created
            assert!(temp_dir.path().join(".chant/config.md").exists());

            // Second init without --force should gracefully exit (not fail)
            let result2 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );
            assert!(result2.is_ok()); // Should still be Ok, just skip re-initialization

            // Third init with --force should succeed and reinitialize
            let result3 = cmd_init(
                None,
                Some("test-force".to_string()),
                false,
                true,
                false,
                vec![],
                None,
                None,
            );
            assert!(result3.is_ok());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_detect_project_name_from_cargo_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            let _ = fs::write("Cargo.toml", "name = \"my-rust-project\"\n");

            let detected = detect_project_name();
            assert_eq!(detected, Some("my-rust-project".to_string()));

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_force_preserves_specs_and_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // First init
            let result1 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );
            assert!(result1.is_ok());
            assert!(temp_dir.path().join(".chant/config.md").exists());

            // Create a dummy spec file
            let specs_dir = temp_dir.path().join(".chant/specs");
            let spec_file = specs_dir.join("2026-01-25-abc-def.md");
            let _ = fs::write(
                &spec_file,
                "---\ntype: code\nstatus: pending\n---\n# Test Spec\n",
            );
            assert!(spec_file.exists());

            // Second init with --force should preserve specs
            let result2 = cmd_init(
                None,
                Some("test-force".to_string()),
                false,
                true,
                false,
                vec![],
                None,
                None,
            );
            assert!(result2.is_ok());

            // Verify spec was preserved
            assert!(spec_file.exists());
            // Verify config was preserved
            assert!(temp_dir.path().join(".chant/config.md").exists());
            // Verify directory structure exists
            assert!(specs_dir.exists());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_force_reinstalls_agents() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // First init with Claude agent
            let result1 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec!["claude".to_string()],
                None,
                None,
            );
            assert!(result1.is_ok());
            assert!(temp_dir.path().join("CLAUDE.md").exists());

            // Get original file content
            let original_content = fs::read_to_string("CLAUDE.md").unwrap();

            // Second init with --force should recreate agent files
            let result2 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                true,
                false,
                vec!["claude".to_string()],
                None,
                None,
            );
            assert!(result2.is_ok());
            assert!(temp_dir.path().join("CLAUDE.md").exists());

            // Verify content is still the same (agent files are updated)
            let new_content = fs::read_to_string("CLAUDE.md").unwrap();
            assert_eq!(original_content, new_content);

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_with_provider_and_model() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // Test init with --provider and --model flags
            let result = cmd_init(
                None,
                Some("test-project".to_string()),
                false,
                false,
                false,
                vec![],
                Some("ollama".to_string()),
                Some("llama3".to_string()),
            );

            assert!(result.is_ok());
            assert!(temp_dir.path().join(".chant/config.md").exists());

            // Verify config contains provider and model
            let config_content = fs::read_to_string(".chant/config.md").unwrap();
            assert!(config_content.contains("provider: ollama"));
            assert!(config_content.contains("model: llama3"));

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_surgical_provider_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // First init without provider
            let result1 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );
            assert!(result1.is_ok());

            // Verify initial config doesn't have provider
            let config_content = fs::read_to_string(".chant/config.md").unwrap();
            assert!(!config_content.contains("provider:"));

            // Second init with only --provider should surgically update
            let result2 = cmd_init(
                None,
                None, // no name
                false,
                false,
                false,
                vec![],
                Some("ollama".to_string()),
                None,
            );
            assert!(result2.is_ok());

            // Verify config now contains provider
            let config_content = fs::read_to_string(".chant/config.md").unwrap();
            assert!(config_content.contains("provider: ollama"));

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_surgical_model_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // First init without model
            let result1 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );
            assert!(result1.is_ok());

            // Second init with only --model should surgically update
            let result2 = cmd_init(
                None,
                None, // no name
                false,
                false,
                false,
                vec![],
                None,
                Some("claude-opus-4".to_string()),
            );
            assert!(result2.is_ok());

            // Verify config now contains model
            let config_content = fs::read_to_string(".chant/config.md").unwrap();
            assert!(config_content.contains("model: claude-opus-4"));

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_init_agent_only_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // First init without agent
            let result1 = cmd_init(
                None,
                Some("test".to_string()),
                false,
                false,
                false,
                vec![],
                None,
                None,
            );
            assert!(result1.is_ok());
            assert!(!temp_dir.path().join("CLAUDE.md").exists());

            // Second init with only --agent should only add agent file
            let result2 = cmd_init(
                None,
                None,
                false,
                false,
                false,
                vec!["claude".to_string()],
                None,
                None,
            );
            assert!(result2.is_ok());
            assert!(temp_dir.path().join("CLAUDE.md").exists());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }
}
