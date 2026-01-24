mod config;
mod git;
mod id;
mod mcp;
mod prompt;
mod spec;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use config::Config;
use spec::{Spec, SpecStatus};

#[derive(Parser)]
#[command(name = "chant")]
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
    },
    /// Add a new spec
    Add {
        /// Description of what to implement
        description: String,
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
    },
    /// Execute a spec
    Work {
        /// Spec ID (full or partial). If omitted with --parallel, executes all ready specs.
        id: Option<String>,
        /// Prompt to use
        #[arg(long)]
        prompt: Option<String>,
        /// Create a feature branch before executing
        #[arg(long)]
        branch: bool,
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
        /// Follow the log in real-time
        #[arg(long, short = 'f')]
        follow: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(name),
        Commands::Add { description } => cmd_add(&description),
        Commands::List { ready, label } => cmd_list(ready, &label),
        Commands::Show { id } => cmd_show(&id),
        Commands::Work {
            id,
            prompt,
            branch,
            pr,
            force,
            parallel,
            label,
        } => cmd_work(
            id.as_deref(),
            prompt.as_deref(),
            branch,
            pr,
            force,
            parallel,
            &label,
        ),
        Commands::Mcp => mcp::run_server(),
        Commands::Status => cmd_status(),
        Commands::Ready => cmd_list(true, &[]),
        Commands::Lint => cmd_lint(),
        Commands::Log { id, lines, follow } => cmd_log(&id, lines, follow),
    }
}

fn cmd_init(name: Option<String>) -> Result<()> {
    let chant_dir = PathBuf::from(".chant");

    if chant_dir.exists() {
        println!("{}", "Chant already initialized.".yellow());
        return Ok(());
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

    // Create .gitignore
    let gitignore_content = "# Local state (not shared)\n.locks/\n.store/\n";
    std::fs::write(chant_dir.join(".gitignore"), gitignore_content)?;

    println!("{} .chant/config.md", "Created".green());
    println!("{} .chant/prompts/standard.md", "Created".green());
    println!("{} .chant/specs/", "Created".green());
    println!("\nChant initialized for project: {}", project_name.cyan());

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

fn cmd_add(description: &str) -> Result<()> {
    let _config = Config::load()?;
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Generate ID
    let id = id::generate_id(&specs_dir)?;
    let filename = format!("{}.md", id);
    let filepath = specs_dir.join(&filename);

    // Create spec content
    let content = format!(
        r#"---
type: code
status: pending
---

# {}
"#,
        description
    );

    std::fs::write(&filepath, content)?;

    println!("{} {}", "Created".green(), id.cyan());
    println!("Edit: {}", filepath.display());

    Ok(())
}

fn cmd_list(ready_only: bool, labels: &[String]) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    let mut specs = spec::load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    if ready_only {
        let all_specs = specs.clone();
        specs.retain(|s| s.is_ready(&all_specs));
    }

    // Filter by labels if specified (OR logic - show specs with any matching label)
    if !labels.is_empty() {
        specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    if specs.is_empty() {
        if ready_only && !labels.is_empty() {
            println!("No ready specs with specified labels.");
        } else if ready_only {
            println!("No ready specs.");
        } else if !labels.is_empty() {
            println!("No specs with specified labels.");
        } else {
            println!("No specs. Create one with `chant add \"description\"`");
        }
        return Ok(());
    }

    for spec in &specs {
        let status_icon = match spec.frontmatter.status {
            SpecStatus::Pending => "○".white(),
            SpecStatus::InProgress => "◐".yellow(),
            SpecStatus::Completed => "●".green(),
            SpecStatus::Failed => "✗".red(),
        };

        println!(
            "{} {} {}",
            status_icon,
            spec.id.cyan(),
            spec.title.as_deref().unwrap_or("(no title)")
        );
    }

    Ok(())
}

fn cmd_show(id: &str) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    let spec = spec::resolve_spec(&specs_dir, id)?;

    println!("{}: {}", "ID".bold(), spec.id.cyan());
    println!(
        "{}: {}",
        "Status".bold(),
        format!("{:?}", spec.frontmatter.status).to_lowercase()
    );
    if let Some(title) = &spec.title {
        println!("{}: {}", "Title".bold(), title);
    }
    if let Some(commit) = &spec.frontmatter.commit {
        println!("{}: {}", "Commit".bold(), commit);
    }
    if let Some(completed_at) = &spec.frontmatter.completed_at {
        println!("{}: {}", "Completed".bold(), completed_at);
    }

    println!("\n{}", "--- Body ---".dimmed());
    println!("{}", spec.body);

    Ok(())
}

fn cmd_status() -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    let specs = spec::load_all_specs(&specs_dir)?;

    // Count by status
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;

    for spec in &specs {
        match spec.frontmatter.status {
            SpecStatus::Pending => pending += 1,
            SpecStatus::InProgress => in_progress += 1,
            SpecStatus::Completed => completed += 1,
            SpecStatus::Failed => failed += 1,
        }
    }

    let total = specs.len();

    println!("{}", "Chant Status".bold());
    println!("============");
    println!("  {:<12} {}", "Pending:", pending);
    println!("  {:<12} {}", "In Progress:", in_progress);
    println!("  {:<12} {}", "Completed:", completed);
    println!("  {:<12} {}", "Failed:", failed);
    println!("  ─────────────");
    println!("  {:<12} {}", "Total:", total);

    Ok(())
}

fn cmd_lint() -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    println!("Linting specs...");

    let mut issues: Vec<(String, String)> = Vec::new();
    let mut total_specs = 0;

    // First pass: collect all spec IDs and check for parse errors
    let mut all_spec_ids: Vec<String> = Vec::new();
    let mut specs_to_check: Vec<Spec> = Vec::new();

    for entry in std::fs::read_dir(&specs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            total_specs += 1;
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            match Spec::load(&path) {
                Ok(spec) => {
                    all_spec_ids.push(spec.id.clone());
                    specs_to_check.push(spec);
                }
                Err(e) => {
                    let issue = format!("Invalid YAML frontmatter: {}", e);
                    println!("{} {}: {}", "✗".red(), id, issue);
                    issues.push((id, issue));
                }
            }
        }
    }

    // Second pass: validate each spec
    for spec in &specs_to_check {
        let mut spec_issues: Vec<String> = Vec::new();

        // Check for title
        if spec.title.is_none() {
            spec_issues.push("Missing title".to_string());
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id) {
                    spec_issues.push(format!("Unknown dependency '{}'", dep_id));
                }
            }
        }

        if spec_issues.is_empty() {
            println!("{} {}", "✓".green(), spec.id);
        } else {
            for issue in spec_issues {
                println!("{} {}: {}", "✗".red(), spec.id, issue);
                issues.push((spec.id.clone(), issue));
            }
        }
    }

    if issues.is_empty() {
        println!("\nAll {} specs valid.", total_specs);
        Ok(())
    } else {
        println!(
            "\nFound {} {} in {} specs.",
            issues.len(),
            if issues.len() == 1 { "issue" } else { "issues" },
            total_specs
        );
        std::process::exit(1);
    }
}

fn cmd_log(id: &str, lines: usize, follow: bool) -> Result<()> {
    cmd_log_at(&PathBuf::from(".chant"), id, lines, follow)
}

/// Result of log file lookup
#[derive(Debug)]
enum LogLookupResult {
    /// Log file exists at the given path
    Found(PathBuf),
    /// Log file not found for the spec
    NotFound { spec_id: String, log_path: PathBuf },
}

fn cmd_log_at(base_path: &std::path::Path, id: &str, lines: usize, follow: bool) -> Result<()> {
    let specs_dir = base_path.join("specs");
    let logs_dir = base_path.join("logs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Resolve spec ID to get the full ID
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if !log_path.exists() {
        println!(
            "{} No log file found for spec '{}'.",
            "⚠".yellow(),
            spec.id.cyan()
        );
        println!("\nLogs are created when a spec is executed with `chant work`.");
        println!("Log path: {}", log_path.display());
        return Ok(());
    }

    // Use tail command to show/follow the log
    let mut args = vec!["-n".to_string(), lines.to_string()];

    if follow {
        args.push("-f".to_string());
    }

    args.push(log_path.to_string_lossy().to_string());

    let status = std::process::Command::new("tail")
        .args(&args)
        .status()
        .context("Failed to run tail command")?;

    if !status.success() {
        anyhow::bail!("tail command exited with status: {}", status);
    }

    Ok(())
}

/// Look up the log file for a spec (used for testing)
fn lookup_log_file(base_path: &std::path::Path, id: &str) -> Result<LogLookupResult> {
    let specs_dir = base_path.join("specs");
    let logs_dir = base_path.join("logs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    let spec = spec::resolve_spec(&specs_dir, id)?;
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

fn cmd_work(
    id: Option<&str>,
    prompt_name: Option<&str>,
    cli_branch: bool,
    cli_pr: bool,
    force: bool,
    parallel: bool,
    labels: &[String],
) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");
    let prompts_dir = PathBuf::from(".chant/prompts");
    let config = Config::load()?;

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Handle parallel execution mode
    if parallel && id.is_none() {
        return cmd_work_parallel(&specs_dir, &prompts_dir, &config, prompt_name, labels);
    }

    // If no ID and not parallel, require an ID
    let id = id.ok_or_else(|| anyhow::anyhow!("Spec ID required (or use --parallel)"))?;

    // Resolve spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Check if already completed
    if spec.frontmatter.status == SpecStatus::Completed && !force {
        println!("{} Spec already completed.", "⚠".yellow());
        println!("Use {} to replay.", "--force".cyan());
        return Ok(());
    }

    // Check if in progress
    if spec.frontmatter.status == SpecStatus::InProgress {
        println!("{} Spec already in progress.", "⚠".yellow());
        return Ok(());
    }

    // Check if dependencies are satisfied
    let all_specs = spec::load_all_specs(&specs_dir)?;
    if !spec.is_ready(&all_specs) {
        // Find which dependencies are blocking
        let mut blocking: Vec<String> = Vec::new();

        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                let dep = all_specs.iter().find(|s| s.id == *dep_id);
                match dep {
                    Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                    Some(d) => blocking
                        .push(format!("{} ({:?})", dep_id, d.frontmatter.status).to_lowercase()),
                    None => blocking.push(format!("{} (not found)", dep_id)),
                }
            }
        }

        if !blocking.is_empty() {
            println!("{} Spec has unsatisfied dependencies.", "✗".red());
            println!("Blocked by: {}", blocking.join(", "));
            anyhow::bail!("Cannot execute spec with unsatisfied dependencies");
        }
    }

    // CLI flags override config defaults
    let create_pr = cli_pr || config.defaults.pr;
    let create_branch = cli_branch || config.defaults.branch || create_pr;

    // Handle branch creation/switching if requested
    let branch_name = if create_branch {
        let branch_name = format!("{}{}", config.defaults.branch_prefix, spec.id);
        create_or_switch_branch(&branch_name)?;
        spec.frontmatter.branch = Some(branch_name.clone());
        println!("{} Branch: {}", "→".cyan(), branch_name);
        Some(branch_name)
    } else {
        None
    };

    // Resolve prompt
    let prompt_name = prompt_name
        .or(spec.frontmatter.prompt.as_deref())
        .unwrap_or(&config.defaults.prompt);

    let prompt_path = prompts_dir.join(format!("{}.md", prompt_name));
    if !prompt_path.exists() {
        anyhow::bail!("Prompt not found: {}", prompt_name);
    }

    // Update status to in_progress
    spec.frontmatter.status = SpecStatus::InProgress;
    spec.save(&spec_path)?;

    println!(
        "{} {} with prompt '{}'",
        "Working".cyan(),
        spec.id,
        prompt_name
    );

    // Assemble prompt
    let message = prompt::assemble(&spec, &prompt_path, &config)?;

    // Invoke agent
    let result = invoke_agent(&message, &spec, prompt_name);

    match result {
        Ok(agent_output) => {
            // Reload spec (it may have been modified by the agent)
            let mut spec = spec::resolve_spec(&specs_dir, &spec.id)?;

            // Check for unchecked acceptance criteria
            let unchecked_count = spec.count_unchecked_checkboxes();
            if unchecked_count > 0 && !force {
                println!(
                    "\n{} Found {} unchecked acceptance {}.",
                    "⚠".yellow(),
                    unchecked_count,
                    if unchecked_count == 1 {
                        "criterion"
                    } else {
                        "criteria"
                    }
                );
                println!("Use {} to skip this validation.", "--force".cyan());
                // Mark as failed since we can't complete with unchecked items
                spec.frontmatter.status = SpecStatus::Failed;
                spec.save(&spec_path)?;
                anyhow::bail!(
                    "Cannot complete spec with {} unchecked acceptance criteria",
                    unchecked_count
                );
            }

            // Get the commit hash
            let commit = get_latest_commit_for_spec(&spec.id)?;

            // Update spec to completed
            spec.frontmatter.status = SpecStatus::Completed;
            spec.frontmatter.commit = commit;
            spec.frontmatter.completed_at = Some(
                chrono::Local::now()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            );
            spec.frontmatter.model = get_model_name();

            println!("\n{} Spec completed!", "✓".green());
            if let Some(commit) = &spec.frontmatter.commit {
                println!("Commit: {}", commit);
            }
            if let Some(model) = &spec.frontmatter.model {
                println!("Model: {}", model);
            }

            // Create PR if requested
            if create_pr {
                let branch_name = branch_name
                    .as_ref()
                    .expect("branch_name should exist when create_pr is true");
                println!("\n{} Pushing branch to remote...", "→".cyan());
                push_branch(branch_name)?;

                let provider = git::get_provider(config.git.provider);
                println!(
                    "{} Creating pull request via {}...",
                    "→".cyan(),
                    provider.name()
                );
                let pr_title = spec.title.clone().unwrap_or_else(|| spec.id.clone());
                let pr_body = spec.body.clone();
                let pr_url = provider.create_pr(&pr_title, &pr_body)?;

                spec.frontmatter.pr = Some(pr_url.clone());
                println!("{} PR created: {}", "✓".green(), pr_url);
            }

            // Append agent output to spec body
            append_agent_output(&mut spec, &agent_output);

            spec.save(&spec_path)?;
        }
        Err(e) => {
            // Update spec to failed
            let mut spec = spec::resolve_spec(&specs_dir, &spec.id)?;
            spec.frontmatter.status = SpecStatus::Failed;
            spec.save(&spec_path)?;

            println!("\n{} Spec failed: {}", "✗".red(), e);
            return Err(e);
        }
    }

    Ok(())
}

/// Result of a single spec execution in parallel mode
struct ParallelResult {
    spec_id: String,
    success: bool,
    commit: Option<String>,
    error: Option<String>,
}

fn cmd_work_parallel(
    specs_dir: &PathBuf,
    prompts_dir: &PathBuf,
    config: &Config,
    prompt_name: Option<&str>,
    labels: &[String],
) -> Result<()> {
    use std::sync::mpsc;
    use std::thread;

    // Load all specs and filter to ready ones
    let all_specs = spec::load_all_specs(specs_dir)?;
    let mut ready_specs: Vec<Spec> = all_specs
        .iter()
        .filter(|s| s.is_ready(&all_specs))
        .cloned()
        .collect();

    // Filter by labels if specified
    if !labels.is_empty() {
        ready_specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    if ready_specs.is_empty() {
        if !labels.is_empty() {
            println!("No ready specs with specified labels.");
        } else {
            println!("No ready specs to execute.");
        }
        return Ok(());
    }

    println!(
        "{} Starting {} specs in parallel...\n",
        "→".cyan(),
        ready_specs.len()
    );

    // Resolve prompt name for all specs
    let default_prompt = &config.defaults.prompt;

    // Create channels for collecting results
    let (tx, rx) = mpsc::channel::<ParallelResult>();

    // Spawn threads for each spec
    let mut handles = Vec::new();

    for spec in ready_specs.iter() {
        // Determine prompt for this spec
        let spec_prompt = prompt_name
            .or(spec.frontmatter.prompt.as_deref())
            .unwrap_or(default_prompt);

        let prompt_path = prompts_dir.join(format!("{}.md", spec_prompt));
        if !prompt_path.exists() {
            println!(
                "{} [{}] Prompt not found: {}",
                "✗".red(),
                spec.id,
                spec_prompt
            );
            continue;
        }

        // Update spec status to in_progress
        let spec_path = specs_dir.join(format!("{}.md", spec.id));
        let mut spec_clone = spec.clone();
        spec_clone.frontmatter.status = SpecStatus::InProgress;
        if let Err(e) = spec_clone.save(&spec_path) {
            println!("{} [{}] Failed to update status: {}", "✗".red(), spec.id, e);
            continue;
        }

        println!("[{}] Working with prompt '{}'", spec.id.cyan(), spec_prompt);

        // Assemble the prompt message
        let message = match prompt::assemble(&spec_clone, &prompt_path, config) {
            Ok(m) => m,
            Err(e) => {
                println!(
                    "{} [{}] Failed to assemble prompt: {}",
                    "✗".red(),
                    spec.id,
                    e
                );
                continue;
            }
        };

        // Clone data for the thread
        let tx_clone = tx.clone();
        let spec_id = spec.id.clone();
        let specs_dir_clone = specs_dir.clone();
        let prompt_name_clone = spec_prompt.to_string();

        let handle = thread::spawn(move || {
            let result = invoke_agent_with_prefix(&message, &spec_id, &prompt_name_clone);
            let (success, commit, error) = match result {
                Ok(_) => {
                    // Get the commit hash
                    let commit = get_latest_commit_for_spec(&spec_id).ok().flatten();

                    // Update spec to completed
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        spec.frontmatter.status = SpecStatus::Completed;
                        spec.frontmatter.commit = commit.clone();
                        spec.frontmatter.completed_at = Some(
                            chrono::Local::now()
                                .format("%Y-%m-%dT%H:%M:%SZ")
                                .to_string(),
                        );
                        spec.frontmatter.model = get_model_name();
                        let _ = spec.save(&spec_path);
                    }

                    (true, commit, None)
                }
                Err(e) => {
                    // Update spec to failed
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        spec.frontmatter.status = SpecStatus::Failed;
                        let _ = spec.save(&spec_path);
                    }

                    (false, None, Some(e.to_string()))
                }
            };

            let _ = tx_clone.send(ParallelResult {
                spec_id,
                success,
                commit,
                error,
            });
        });

        handles.push(handle);
    }

    // Drop the original sender so the receiver knows when all threads are done
    drop(tx);

    // Collect results
    let mut completed = 0;
    let mut failed = 0;

    println!();

    for result in rx {
        if result.success {
            completed += 1;
            if let Some(commit) = result.commit {
                println!(
                    "[{}] {} Completed (commit: {})",
                    result.spec_id.cyan(),
                    "✓".green(),
                    commit
                );
            } else {
                println!("[{}] {} Completed", result.spec_id.cyan(), "✓".green());
            }
        } else {
            failed += 1;
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            println!(
                "[{}] {} Failed: {}",
                result.spec_id.cyan(),
                "✗".red(),
                error_msg
            );
        }
    }

    // Wait for all threads to finish
    for handle in handles {
        let _ = handle.join();
    }

    // Print summary
    println!(
        "\n{}: {} completed, {} failed",
        "Summary".bold(),
        completed,
        failed
    );

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Invoke the agent with output prefixed by spec ID
fn invoke_agent_with_prefix(message: &str, spec_id: &str, prompt_name: &str) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!(".chant/specs/{}.md", spec_id))?;

    let mut child = Command::new("claude")
        .arg("--print")
        .arg("--dangerously-skip-permissions")
        .arg(message)
        .env("CHANT_SPEC_ID", spec_id)
        .env("CHANT_SPEC_FILE", &spec_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to invoke claude CLI. Is it installed and in PATH?")?;

    // Stream stdout with prefix and capture it
    let mut captured_output = String::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let prefix = format!("[{}]", spec_id);
        for line in reader.lines().map_while(Result::ok) {
            println!("{} {}", prefix.cyan(), line);
            captured_output.push_str(&line);
            captured_output.push('\n');
        }
    }

    let status = child.wait()?;

    // Write full output to log file (regardless of success/failure)
    if let Err(e) = write_agent_log(spec_id, prompt_name, &captured_output) {
        eprintln!(
            "{} [{}] Failed to write agent log: {}",
            "⚠".yellow(),
            spec_id,
            e
        );
    }

    if !status.success() {
        anyhow::bail!("Agent exited with status: {}", status);
    }

    Ok(())
}

fn invoke_agent(message: &str, spec: &Spec, prompt_name: &str) -> Result<String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!(".chant/specs/{}.md", spec.id))?;

    let mut child = Command::new("claude")
        .arg("--print")
        .arg("--dangerously-skip-permissions")
        .arg(message)
        .env("CHANT_SPEC_ID", &spec.id)
        .env("CHANT_SPEC_FILE", &spec_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to invoke claude CLI. Is it installed and in PATH?")?;

    // Stream stdout and capture it
    let mut captured_output = String::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            println!("{}", line);
            captured_output.push_str(&line);
            captured_output.push('\n');
        }
    }

    let status = child.wait()?;

    // Write full output to log file (regardless of success/failure)
    if let Err(e) = write_agent_log(&spec.id, prompt_name, &captured_output) {
        eprintln!("{} Failed to write agent log: {}", "⚠".yellow(), e);
    }

    if !status.success() {
        anyhow::bail!("Agent exited with status: {}", status);
    }

    Ok(captured_output)
}

fn get_latest_commit_for_spec(spec_id: &str) -> Result<Option<String>> {
    use std::process::Command;

    // Look for a commit with the chant(spec_id) pattern
    let pattern = format!("chant({})", spec_id);

    let output = Command::new("git")
        .args(["log", "--oneline", "-1", "--grep", &pattern])
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = stdout.split_whitespace().next() {
            if !hash.is_empty() {
                return Ok(Some(hash.to_string()));
            }
        }
    }

    // Fallback: get HEAD commit if no spec-specific commit found
    let head_output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()?;

    if head_output.status.success() {
        let head_hash = String::from_utf8_lossy(&head_output.stdout)
            .trim()
            .to_string();
        if !head_hash.is_empty() {
            eprintln!(
                "{} No commit with 'chant({})' found, using HEAD: {}",
                "⚠".yellow(),
                spec_id,
                head_hash
            );
            return Ok(Some(head_hash));
        }
    }

    // No commit found at all - log warning
    eprintln!(
        "{} Could not find any commit for spec '{}'",
        "⚠".yellow(),
        spec_id
    );

    Ok(None)
}

fn create_or_switch_branch(branch_name: &str) -> Result<()> {
    use std::process::Command;

    // Try to create a new branch
    let create_output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .output()
        .context("Failed to run git checkout")?;

    if create_output.status.success() {
        return Ok(());
    }

    // Branch might already exist, try to switch to it
    let switch_output = Command::new("git")
        .args(["checkout", branch_name])
        .output()
        .context("Failed to run git checkout")?;

    if switch_output.status.success() {
        return Ok(());
    }

    // Both failed, return error
    let stderr = String::from_utf8_lossy(&switch_output.stderr);
    anyhow::bail!(
        "Failed to create or switch to branch '{}': {}",
        branch_name,
        stderr
    )
}

fn push_branch(branch_name: &str) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["push", "-u", "origin", branch_name])
        .output()
        .context("Failed to run git push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to push branch '{}': {}", branch_name, stderr);
    }

    Ok(())
}

const MAX_AGENT_OUTPUT_CHARS: usize = 5000;

/// Ensure the logs directory exists and is in .gitignore
fn ensure_logs_dir() -> Result<()> {
    ensure_logs_dir_at(&PathBuf::from(".chant"))
}

/// Ensure the logs directory exists and is in .gitignore at the given base path
fn ensure_logs_dir_at(base_path: &PathBuf) -> Result<()> {
    let logs_dir = base_path.join("logs");
    let gitignore_path = base_path.join(".gitignore");

    // Create logs directory if it doesn't exist
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)?;
    }

    // Add logs/ to .gitignore if not already present
    let gitignore_content = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    if !gitignore_content.lines().any(|line| line.trim() == "logs/") {
        let new_content = if gitignore_content.is_empty() {
            "logs/\n".to_string()
        } else if gitignore_content.ends_with('\n') {
            format!("{}logs/\n", gitignore_content)
        } else {
            format!("{}\nlogs/\n", gitignore_content)
        };
        std::fs::write(&gitignore_path, new_content)?;
    }

    Ok(())
}

/// Write full agent output to log file
fn write_agent_log(spec_id: &str, prompt_name: &str, output: &str) -> Result<()> {
    write_agent_log_at(&PathBuf::from(".chant"), spec_id, prompt_name, output)
}

/// Write full agent output to log file at the given base path
fn write_agent_log_at(
    base_path: &PathBuf,
    spec_id: &str,
    prompt_name: &str,
    output: &str,
) -> Result<()> {
    ensure_logs_dir_at(base_path)?;

    let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let log_content = format!(
        "# Agent Log: {}\n# Started: {}\n# Prompt: {}\n\n{}",
        spec_id, timestamp, prompt_name, output
    );

    std::fs::write(&log_path, log_content)?;

    Ok(())
}

/// Get the model name from environment variables.
/// Checks CHANT_MODEL first, then ANTHROPIC_MODEL.
fn get_model_name() -> Option<String> {
    std::env::var("CHANT_MODEL")
        .ok()
        .or_else(|| std::env::var("ANTHROPIC_MODEL").ok())
        .filter(|s| !s.is_empty())
}

fn append_agent_output(spec: &mut Spec, output: &str) {
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let formatted_output = if output.len() > MAX_AGENT_OUTPUT_CHARS {
        let truncated = &output[..MAX_AGENT_OUTPUT_CHARS];
        format!(
            "{}\n\n... (output truncated, {} chars total)",
            truncated,
            output.len()
        )
    } else {
        output.to_string()
    };

    let agent_section = format!(
        "\n\n## Agent Output\n\n{}\n\n```\n{}```\n",
        timestamp,
        formatted_output.trim_end()
    );

    spec.body.push_str(&agent_section);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_logs_dir_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Logs dir shouldn't exist yet
        assert!(!base_path.join("logs").exists());

        // Call ensure_logs_dir_at
        ensure_logs_dir_at(&base_path).unwrap();

        // Logs dir should now exist
        assert!(base_path.join("logs").exists());
        assert!(base_path.join("logs").is_dir());
    }

    #[test]
    fn test_ensure_logs_dir_updates_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create base dir without .gitignore
        // (tempdir already exists, no need to create)

        // Call ensure_logs_dir_at
        ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should now exist and contain "logs/"
        let gitignore_path = base_path.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("logs/"));
    }

    #[test]
    fn test_ensure_logs_dir_appends_to_existing_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create existing .gitignore with other content
        let gitignore_path = base_path.join(".gitignore");
        std::fs::write(&gitignore_path, "*.tmp\n").unwrap();

        // Call ensure_logs_dir_at
        ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should contain both original and new content
        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("*.tmp"));
        assert!(content.contains("logs/"));
    }

    #[test]
    fn test_ensure_logs_dir_no_duplicate_gitignore_entry() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create existing .gitignore that already has logs/
        let gitignore_path = base_path.join(".gitignore");
        std::fs::write(&gitignore_path, "logs/\n").unwrap();

        // Create logs dir (since ensure_logs_dir only updates gitignore when creating dir)
        std::fs::create_dir_all(base_path.join("logs")).unwrap();

        // Call ensure_logs_dir_at
        ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should still have only one "logs/" entry
        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        let count = content
            .lines()
            .filter(|line| line.trim() == "logs/")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_write_agent_log_format() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";
        let output = "Test agent output\nWith multiple lines";

        // Write the log
        write_agent_log_at(&base_path, spec_id, prompt_name, output).unwrap();

        // Read it back
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        assert!(log_path.exists());

        let content = std::fs::read_to_string(&log_path).unwrap();

        // Check header format
        assert!(content.starts_with("# Agent Log: 2026-01-24-00a-xyz\n"));
        assert!(content.contains("# Started: "));
        assert!(content.contains("# Prompt: standard\n"));

        // Check output is preserved
        assert!(content.contains("Test agent output\nWith multiple lines"));
    }

    #[test]
    fn test_write_agent_log_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00b-abc";
        let prompt_name = "standard";

        // Write first log
        write_agent_log_at(&base_path, spec_id, prompt_name, "Content A").unwrap();

        // Write second log (simulating replay)
        write_agent_log_at(&base_path, spec_id, prompt_name, "Content B").unwrap();

        // Read it back
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        let content = std::fs::read_to_string(&log_path).unwrap();

        // Should contain only Content B
        assert!(content.contains("Content B"));
        assert!(!content.contains("Content A"));
    }

    #[test]
    fn test_lookup_log_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create specs directory and a spec file
        let specs_dir = base_path.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let spec_content = r#"---
type: code
status: pending
---

# Test spec
"#;
        std::fs::write(specs_dir.join("2026-01-24-00a-xyz.md"), spec_content).unwrap();

        // Lookup log without creating logs directory
        let result = lookup_log_file(&base_path, "xyz").unwrap();

        match result {
            LogLookupResult::NotFound { spec_id, log_path } => {
                assert_eq!(spec_id, "2026-01-24-00a-xyz");
                assert!(log_path
                    .to_string_lossy()
                    .contains("2026-01-24-00a-xyz.log"));
            }
            LogLookupResult::Found(_) => panic!("Expected NotFound, got Found"),
        }
    }

    #[test]
    fn test_lookup_log_file_found() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create specs directory and a spec file
        let specs_dir = base_path.join("specs");
        let logs_dir = base_path.join("logs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::create_dir_all(&logs_dir).unwrap();

        let spec_content = r#"---
type: code
status: pending
---

# Test spec
"#;
        std::fs::write(specs_dir.join("2026-01-24-00b-abc.md"), spec_content).unwrap();

        // Create a log file
        std::fs::write(
            logs_dir.join("2026-01-24-00b-abc.log"),
            "# Agent Log\nTest output",
        )
        .unwrap();

        // Lookup log
        let result = lookup_log_file(&base_path, "abc").unwrap();

        match result {
            LogLookupResult::Found(path) => {
                assert!(path.to_string_lossy().contains("2026-01-24-00b-abc.log"));
            }
            LogLookupResult::NotFound { .. } => panic!("Expected Found, got NotFound"),
        }
    }

    #[test]
    fn test_lookup_log_file_spec_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create specs directory and multiple spec files
        let specs_dir = base_path.join("specs");
        let logs_dir = base_path.join("logs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::create_dir_all(&logs_dir).unwrap();

        let spec_content = r#"---
type: code
status: pending
---

# Test spec
"#;
        std::fs::write(specs_dir.join("2026-01-24-00c-def.md"), spec_content).unwrap();
        std::fs::write(specs_dir.join("2026-01-24-00d-ghi.md"), spec_content).unwrap();

        // Create log file for one spec
        std::fs::write(
            logs_dir.join("2026-01-24-00c-def.log"),
            "# Agent Log\nOutput for def",
        )
        .unwrap();

        // Lookup using partial ID should resolve correctly
        let result = lookup_log_file(&base_path, "def").unwrap();
        match result {
            LogLookupResult::Found(path) => {
                assert!(path.to_string_lossy().contains("2026-01-24-00c-def.log"));
            }
            LogLookupResult::NotFound { .. } => panic!("Expected Found for 'def'"),
        }

        // Lookup for spec without log
        let result = lookup_log_file(&base_path, "ghi").unwrap();
        match result {
            LogLookupResult::NotFound { spec_id, .. } => {
                assert_eq!(spec_id, "2026-01-24-00d-ghi");
            }
            LogLookupResult::Found(_) => panic!("Expected NotFound for 'ghi'"),
        }
    }

    #[test]
    fn test_lookup_log_file_not_initialized() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Don't create specs directory
        let result = lookup_log_file(&base_path, "abc");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Chant not initialized"));
    }

    #[test]
    fn test_get_model_name_from_chant_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set CHANT_MODEL
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_name();
        assert_eq!(result, Some("claude-opus-4-5".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_get_model_name_from_anthropic_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set only ANTHROPIC_MODEL
        std::env::remove_var("CHANT_MODEL");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = get_model_name();
        assert_eq!(result, Some("claude-sonnet-4".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    fn test_get_model_name_chant_takes_precedence() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set both env vars
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = get_model_name();
        // CHANT_MODEL takes precedence
        assert_eq!(result, Some("claude-opus-4-5".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    fn test_get_model_name_none_when_unset() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset both env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_name();
        assert_eq!(result, None);

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_get_model_name_empty_string_returns_none() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set empty string
        std::env::set_var("CHANT_MODEL", "");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_name();
        assert_eq!(result, None);

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }
}
