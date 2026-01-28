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
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "chant")]
#[command(version)]
#[command(about = "Intent Driven Development", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize chant in the current directory
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
    },
    /// Add a new spec
    Add {
        /// Description of what to implement
        description: String,
        /// Prompt to use for execution
        #[arg(long)]
        prompt: Option<String>,
        /// Require approval before this spec can be worked
        #[arg(long)]
        needs_approval: bool,
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
    },
    /// Show spec details
    Show {
        /// Spec ID (full or partial) or repo:spec-id for cross-repo specs
        id: String,
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
        /// Create a pull request after spec completes
        #[arg(long)]
        pr: bool,
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
    },
    /// Show ready specs (shortcut for `list --ready`)
    Ready {
        /// Show ready specs across all configured repos
        #[arg(long)]
        global: bool,
        /// Filter to specific repository (implies --global)
        #[arg(long)]
        repo: Option<String>,
    },
    /// Refresh dependency status for all specs
    Refresh {
        /// Show detailed list of ready and blocked specs
        #[arg(short, long)]
        verbose: bool,
    },
    /// Validate all specs for common issues
    Lint,
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
        #[arg(long)]
        commit: bool,
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
        /// Create a pull request after spec completes
        #[arg(long)]
        pr: bool,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            subcommand,
            name,
            silent,
            force,
            minimal,
            agent,
        } => cmd_init(subcommand.as_deref(), name, silent, force, minimal, agent),
        Commands::Add {
            description,
            prompt,
            needs_approval,
        } => cmd::spec::cmd_add(&description, prompt.as_deref(), needs_approval),
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
        ),
        Commands::Show { id, no_render } => cmd::spec::cmd_show(&id, no_render),
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
            pr,
            force,
            parallel,
            label,
            finalize,
            allow_no_commits,
            max_parallel,
            no_cleanup,
            cleanup,
            skip_approval,
        } => cmd::work::cmd_work(
            &ids,
            prompt.as_deref(),
            branch,
            pr,
            force,
            parallel,
            &label,
            finalize,
            allow_no_commits,
            max_parallel,
            no_cleanup,
            cleanup,
            skip_approval,
        ),
        Commands::Mcp => mcp::run_server(),
        Commands::Status { global, repo } => cmd::spec::cmd_status(global, repo.as_deref()),
        Commands::Ready { global, repo } => cmd::spec::cmd_list(
            true,
            &[],
            None,
            None,
            global,
            repo.as_deref(),
            None,
            None,
            None,
            None,
            None,
            false,
        ),
        Commands::Refresh { verbose } => cmd::refresh::cmd_refresh(verbose),
        Commands::Lint => cmd::spec::cmd_lint(),
        Commands::Log {
            id,
            lines,
            no_follow,
        } => cmd::lifecycle::cmd_log(&id, lines, !no_follow),
        Commands::Split { id, model, force } => {
            cmd::lifecycle::cmd_split(&id, model.as_deref(), force)
        }
        Commands::Archive {
            id,
            dry_run,
            older_than,
            force,
            commit,
            no_stage,
        } => {
            cmd::lifecycle::cmd_archive(id.as_deref(), dry_run, older_than, force, commit, no_stage)
        }
        Commands::Merge {
            ids,
            all,
            all_completed,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            rebase,
            auto,
        } => cmd::lifecycle::cmd_merge(
            &ids,
            all,
            all_completed,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            rebase,
            auto,
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
            pr,
            force,
            dry_run,
            yes,
        } => cmd::lifecycle::cmd_replay(&id, prompt.as_deref(), branch, pr, force, dry_run, yes),
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

/// Show setup instructions for the merge driver
fn cmd_merge_driver_setup() -> Result<()> {
    println!("{}", chant::merge_driver::get_setup_instructions());
    Ok(())
}

fn cmd_init(
    subcommand: Option<&str>,
    name: Option<String>,
    silent: bool,
    force: bool,
    minimal: bool,
    agents: Vec<String>,
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

    // Detect if we're in wizard mode (no flags provided)
    let is_wizard_mode = name.is_none() && !silent && !force && !minimal && agents.is_empty();

    // Gather parameters - either from wizard or from flags
    let (final_name, final_silent, final_minimal, final_agents) = if is_wizard_mode {
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
        )
    } else {
        // Direct mode: use provided values
        let project_name = name
            .unwrap_or_else(|| detect_project_name().unwrap_or_else(|| "my-project".to_string()));
        (project_name, silent, minimal, agents)
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

    if chant_dir.exists() {
        // If agents are specified, allow proceeding to update agent files
        // Otherwise, check if --force is set for full reinitialization
        if final_agents.is_empty() && !force {
            println!("{}", "Chant already initialized.".yellow());
            return Ok(());
        }

        // If agents are specified and chant already exists, we continue without full reinitialization
        // If --force is set (and no agents), do full reinitialization below
        if force && !final_agents.is_empty() {
            // force flag with agents: reinitialize everything (preserve specs/config)
            // This allows updating agent instructions without losing existing specs/config
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
        } else if force && final_agents.is_empty() {
            // force flag without agents: do full reinitialization (classic behavior)
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
        // If agents are specified and no --force: just continue to update agent files
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
        let config_content = format!(
            r#"---
project:
  name: {}

defaults:
  prompt: standard
  branch: false
  pr: false
---

# Chant Configuration

Project initialized on {}.
"#,
            project_name,
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
        let gitignore_content = "# Local state (not shared)\n.locks/\n.store/\n";
        std::fs::write(&gitignore_path, gitignore_content)?;
    }

    // Create .gitattributes in repo root (only if it doesn't exist)
    // This enables the custom merge driver for spec files
    let gitattributes_path = PathBuf::from(".gitattributes");
    if !gitattributes_path.exists() {
        let gitattributes_content = r#"# Chant spec files use a custom merge driver for intelligent conflict resolution
# This driver automatically resolves frontmatter conflicts while preserving implementation content
#
# Setup: Run `chant merge-driver-setup` for configuration instructions
.chant/specs/*.md merge=chant-spec
"#;
        std::fs::write(&gitattributes_path, gitattributes_content)?;
    }

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
    let mut created_agent_names = Vec::new();
    if !parsed_agents.is_empty() {
        // Create agents directory
        std::fs::create_dir_all(chant_dir.join("agents"))?;

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

            // Check if file already exists
            if target_path.exists() && !force {
                // File exists and --force not specified
                // Handle TTY vs non-TTY scenarios
                if atty::is(atty::Stream::Stdin) {
                    // TTY mode: prompt for confirmation
                    let should_overwrite = dialoguer::Confirm::new()
                        .with_prompt(format!(
                            "{} already exists. Overwrite?",
                            target_path.display()
                        ))
                        .default(false)
                        .interact()?;

                    if !should_overwrite {
                        continue; // Skip this file
                    }
                } else {
                    // Non-TTY mode: show usage hint and skip
                    eprintln!(
                        "{} {} already exists. Use {} to overwrite.",
                        "•".yellow(),
                        target_path.display(),
                        "--force".cyan()
                    );
                    continue;
                }
            }

            // Write the template
            if let Some(parent) = target_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&target_path, template.content)?;
            created_agents.push(target_path);
            created_agent_names.push(provider.as_str());
        }
    }

    println!("{} .chant/config.md", "Created".green());
    if !final_minimal {
        println!("{} .chant/prompts/standard.md", "Created".green());
        println!("{} .chant/prompts/split.md", "Created".green());
        println!("{} .chant/prompts/verify.md", "Created".green());
    }
    println!("{} .chant/specs/", "Created".green());

    // Print agent files that were actually created
    for target_path in &created_agents {
        println!("{} {}", "Created".green(), target_path.display());
    }

    println!("\nChant initialized for project: {}", project_name.cyan());

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

    if !created_agent_names.is_empty() {
        let agent_names = created_agent_names.join(", ");
        println!(
            "{} Agent configuration created for: {}",
            "ℹ".cyan(),
            agent_names.cyan()
        );
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
            let result1 = cmd_init(None, Some("test".to_string()), false, false, false, vec![]);
            assert!(result1.is_ok());

            // Verify files were created
            assert!(temp_dir.path().join(".chant/config.md").exists());

            // Second init without --force should gracefully exit (not fail)
            let result2 = cmd_init(None, Some("test".to_string()), false, false, false, vec![]);
            assert!(result2.is_ok()); // Should still be Ok, just skip re-initialization

            // Third init with --force should succeed and reinitialize
            let result3 = cmd_init(
                None,
                Some("test-force".to_string()),
                false,
                true,
                false,
                vec![],
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
            let result1 = cmd_init(None, Some("test".to_string()), false, false, false, vec![]);
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
            );
            assert!(result2.is_ok());
            assert!(temp_dir.path().join("CLAUDE.md").exists());

            // Verify content is still the same (agent files are updated)
            let new_content = fs::read_to_string("CLAUDE.md").unwrap();
            assert_eq!(original_content, new_content);

            let _ = std::env::set_current_dir(orig_dir);
        }
    }
}
