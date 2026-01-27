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
use std::path::PathBuf;

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
    },
    /// Show spec details
    Show {
        /// Spec ID (full or partial)
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
        /// Skip validation of unchecked acceptance criteria
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
    },
    /// Start MCP server (Model Context Protocol)
    Mcp,
    /// Show project status summary
    Status,
    /// Show ready specs (shortcut for `list --ready`)
    Ready,
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
    },
    /// Merge completed spec branches back to main
    Merge {
        /// Spec ID(s) to merge (one or more)
        #[arg(value_name = "ID")]
        ids: Vec<String>,
        /// Merge all completed spec branches
        #[arg(long)]
        all: bool,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            name,
            silent,
            force,
            minimal,
            agent,
        } => cmd_init(name, silent, force, minimal, agent),
        Commands::Add {
            description,
            prompt,
        } => cmd::spec::cmd_add(&description, prompt.as_deref()),
        Commands::List {
            ready,
            label,
            r#type,
            status,
        } => cmd::spec::cmd_list(ready, &label, r#type.as_deref(), status.as_deref()),
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
        } => cmd::search::cmd_search(
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
        ),
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
        ),
        Commands::Mcp => mcp::run_server(),
        Commands::Status => cmd::spec::cmd_status(),
        Commands::Ready => cmd::spec::cmd_list(true, &[], None, None),
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
        } => cmd::lifecycle::cmd_archive(id.as_deref(), dry_run, older_than, force),
        Commands::Merge {
            ids,
            all,
            dry_run,
            delete_branch,
            continue_on_error,
            yes,
            rebase,
            auto,
        } => cmd::lifecycle::cmd_merge(
            &ids,
            all,
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
    }
}

fn cmd_init(
    name: Option<String>,
    silent: bool,
    force: bool,
    minimal: bool,
    agents: Vec<String>,
) -> Result<()> {
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
        if !force {
            println!("{}", "Chant already initialized.".yellow());
            return Ok(());
        }
        // force flag: remove existing .chant directory
        std::fs::remove_dir_all(&chant_dir)?;
    }

    // Detect project name
    let project_name = final_name;

    // Create directory structure
    std::fs::create_dir_all(chant_dir.join("specs"))?;
    std::fs::create_dir_all(chant_dir.join("prompts"))?;
    std::fs::create_dir_all(chant_dir.join(".locks"))?;
    std::fs::create_dir_all(chant_dir.join(".store"))?;

    // Create config.md
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
    std::fs::write(chant_dir.join("config.md"), config_content)?;

    if !final_minimal {
        // Create standard prompt
        let prompt_content = r#"---
name: standard
purpose: Default execution prompt
---

# Execute Spec

You are implementing a spec for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Instructions

1. **Read** the relevant code first
2. **Plan** your approach before coding
3. **Implement** the changes
4. **Verify** the implementation works
5. **Commit** with message: `chant({{spec.id}}): <description>`

## Constraints

- Only modify files related to this spec
- Follow existing code patterns
- Do not refactor unrelated code
"#;
        std::fs::write(chant_dir.join("prompts/standard.md"), prompt_content)?;

        // Create split prompt
        let split_prompt_content = r#"---
name: split
purpose: Split a driver spec into members with detailed acceptance criteria
---

# Split Driver Specification into Member Specs

You are analyzing a driver specification for the {{project.name}} project and proposing how to split it into smaller, ordered member specs.

## Driver Specification to Split

**ID:** {{spec.id}}
**Title:** {{spec.title}}

{{spec.description}}

## Your Task

1. Analyze the specification and its acceptance criteria
2. Propose a sequence of member specs where:
   - Each member leaves code in a compilable state
   - Each member is independently testable and valuable
   - Dependencies are minimized (parallelize where possible)
   - Common patterns are respected (add new alongside old → update callers → remove old)
3. For each member, provide:
   - A clear, concise title
   - Description of what should be implemented
   - Explicit acceptance criteria with checkboxes for verification
   - Edge cases that should be considered
   - Example test cases where applicable
   - List of affected files (if identifiable from the spec)
   - Clear "done" conditions that can be verified

## Why Thorough Acceptance Criteria?

These member specs will be executed by Claude Haiku, a capable but smaller model. A strong model (Opus/Sonnet) doing the split should think through edge cases and requirements thoroughly. Each member must have:

- **Specific checkboxes** for each piece of work (not just "implement it")
- **Edge case callouts** to prevent oversights
- **Test scenarios** to clarify expected behavior
- **Clear success metrics** so Haiku knows when it's done

This way, Haiku has a detailed specification to follow and won't miss important aspects.

## Output Format

For each member, output exactly this format:

```
## Member N: <title>

<description of what this member accomplishes>

### Acceptance Criteria

- [ ] Specific criterion 1
- [ ] Specific criterion 2
- [ ] Specific criterion 3

### Edge Cases

- Edge case 1: Describe what should happen and how to test it
- Edge case 2: Describe what should happen and how to test it

### Example Test Cases

For this feature, verify:
- Case 1: Input X should produce Y
- Case 2: Input A should produce B

**Affected Files:**
- file1.rs
- file2.rs
```

If no files are identified, you can omit the Affected Files section.

Create as many members as needed (typically 3-5 for a medium spec).
"#;
        std::fs::write(chant_dir.join("prompts/split.md"), split_prompt_content)?;
    }

    // Create .gitignore
    let gitignore_content = "# Local state (not shared)\n.locks/\n.store/\n";
    std::fs::write(chant_dir.join(".gitignore"), gitignore_content)?;

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

            // Write the template
            if let Some(parent) = target_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&target_path, template.content)?;
        }
    }

    println!("{} .chant/config.md", "Created".green());
    if !final_minimal {
        println!("{} .chant/prompts/standard.md", "Created".green());
        println!("{} .chant/prompts/split.md", "Created".green());
    }
    println!("{} .chant/specs/", "Created".green());

    // Print agent files created
    for provider in &parsed_agents {
        match provider.config_filename() {
            ".amazonq/rules.md" => {
                println!("{} .amazonq/rules.md", "Created".green());
            }
            filename => {
                println!("{} {}", "Created".green(), filename);
            }
        }
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

    if !parsed_agents.is_empty() {
        let agent_names = parsed_agents
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ");
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
            let result1 = cmd_init(Some("test".to_string()), false, false, false, vec![]);
            assert!(result1.is_ok());

            // Verify files were created
            assert!(temp_dir.path().join(".chant/config.md").exists());

            // Second init without --force should gracefully exit (not fail)
            let result2 = cmd_init(Some("test".to_string()), false, false, false, vec![]);
            assert!(result2.is_ok()); // Should still be Ok, just skip re-initialization

            // Third init with --force should succeed and reinitialize
            let result3 = cmd_init(Some("test-force".to_string()), false, true, false, vec![]);
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
}
