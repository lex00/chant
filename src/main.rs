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
    },
    /// Show spec details
    Show {
        /// Spec ID (full or partial)
        id: String,
        /// Disable markdown rendering
        #[arg(long)]
        no_render: bool,
    },
    /// Execute a spec
    Work {
        /// Spec ID (full or partial). If omitted with --parallel, executes all ready specs.
        id: Option<String>,
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
    },
    /// Diagnose spec execution issues
    Diagnose {
        /// Spec ID (full or partial)
        id: String,
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
        Commands::List { ready, label } => cmd::spec::cmd_list(ready, &label),
        Commands::Show { id, no_render } => cmd::spec::cmd_show(&id, no_render),
        Commands::Work {
            id,
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
            id.as_deref(),
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
        Commands::Ready => cmd::spec::cmd_list(true, &[]),
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
        } => cmd::lifecycle::cmd_merge(&ids, all, dry_run, delete_branch, continue_on_error, yes),
        Commands::Diagnose { id } => cmd::lifecycle::cmd_diagnose(&id),
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

    // For silent mode: validate that .chant/ is not already tracked in git
    // Do this check BEFORE the exists check so we catch tracking issues even if dir exists
    if silent {
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
    let project_name =
        name.unwrap_or_else(|| detect_project_name().unwrap_or_else(|| "my-project".to_string()));

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

    if !minimal {
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
    if silent {
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
    let parsed_agents = templates::parse_agent_providers(&agents)?;
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
    if !minimal {
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

    if silent {
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
    if minimal {
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
