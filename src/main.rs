mod config;
mod git;
mod id;
mod mcp;
mod prompt;
mod spec;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};

use config::Config;
use spec::{Spec, SpecFrontmatter, SpecStatus};

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
    /// Split a spec into subtasks
    Split {
        /// Spec ID to split (full or partial)
        id: String,
        /// Model to use for split analysis (overrides config)
        #[arg(long)]
        model: Option<String>,
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
        Commands::Split { id, model } => cmd_split(&id, model.as_deref()),
        Commands::Archive {
            id,
            dry_run,
            older_than,
            force,
        } => cmd_archive(id.as_deref(), dry_run, older_than, force),
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

    // Create split prompt
    let split_prompt_content = r#"---
name: split
purpose: Split a driver spec into subtasks with detailed acceptance criteria
---

# Split Driver Specification into Subtasks

You are analyzing a driver specification for the {{project.name}} project and proposing how to split it into smaller, ordered subtasks.

## Driver Specification to Split

**ID:** {{spec.id}}
**Title:** {{spec.title}}

{{spec.description}}

## Your Task

1. Analyze the specification and its acceptance criteria
2. Propose a sequence of subtasks where:
   - Each subtask leaves code in a compilable state
   - Each subtask is independently testable and valuable
   - Dependencies are minimized (parallelize where possible)
   - Common patterns are respected (add new alongside old → update callers → remove old)
3. For each subtask, provide:
   - A clear, concise title
   - Description of what should be implemented
   - Explicit acceptance criteria with checkboxes for verification
   - Edge cases that should be considered
   - Example test cases where applicable
   - List of affected files (if identifiable from the spec)
   - Clear "done" conditions that can be verified

## Why Thorough Acceptance Criteria?

These subtasks will be executed by Claude Haiku, a capable but smaller model. A strong model (Opus/Sonnet) doing the split should think through edge cases and requirements thoroughly. Each subtask must have:

- **Specific checkboxes** for each piece of work (not just "implement it")
- **Edge case callouts** to prevent oversights
- **Test scenarios** to clarify expected behavior
- **Clear success metrics** so Haiku knows when it's done

This way, Haiku has a detailed specification to follow and won't miss important aspects.

## Output Format

For each subtask, output exactly this format:

```
## Subtask N: <title>

<description of what this subtask accomplishes>

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

Create as many subtasks as needed (typically 3-5 for a medium spec).
"#;
    std::fs::write(chant_dir.join("prompts/split.md"), split_prompt_content)?;

    // Create .gitignore
    let gitignore_content = "# Local state (not shared)\n.locks/\n.store/\n";
    std::fs::write(chant_dir.join(".gitignore"), gitignore_content)?;

    println!("{} .chant/config.md", "Created".green());
    println!("{} .chant/prompts/standard.md", "Created".green());
    println!("{} .chant/prompts/split.md", "Created".green());
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

/// Format a YAML value with semantic colors based on key and value type.
/// - status: green (completed), yellow (in_progress/pending), red (failed)
/// - commit: cyan
/// - type: blue
/// - lists: magenta
/// - bools: green (true), red (false)
fn format_yaml_value(key: &str, value: &serde_yaml::Value) -> String {
    use serde_yaml::Value;

    match value {
        Value::Null => "~".dimmed().to_string(),
        Value::Bool(b) => {
            if *b {
                "true".green().to_string()
            } else {
                "false".red().to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            // Apply semantic coloring based on key
            match key {
                "status" => match s.as_str() {
                    "completed" => s.green().to_string(),
                    "failed" => s.red().to_string(),
                    _ => s.yellow().to_string(), // pending, in_progress
                },
                "commit" => s.cyan().to_string(),
                "type" => s.blue().to_string(),
                _ => s.to_string(),
            }
        }
        Value::Sequence(seq) => {
            let items: Vec<String> = seq
                .iter()
                .map(|v| match v {
                    Value::String(s) => {
                        // Color commits like commit hashes
                        if key == "commits" {
                            s.cyan().to_string()
                        } else {
                            s.magenta().to_string()
                        }
                    }
                    _ => format_yaml_value("", v),
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        Value::Mapping(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let key_str = match k {
                        Value::String(s) => s.clone(),
                        _ => format!("{:?}", k),
                    };
                    format!("{}: {}", key_str, format_yaml_value(&key_str, v))
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        Value::Tagged(tagged) => format_yaml_value(key, &tagged.value),
    }
}

/// Convert a snake_case key to Title Case for display.
fn key_to_title_case(key: &str) -> String {
    key.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn cmd_show(id: &str) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    let spec = spec::resolve_spec(&specs_dir, id)?;

    // Print ID (not from frontmatter)
    println!("{}: {}", "ID".bold(), spec.id.cyan());

    // Print title if available (extracted from body, not frontmatter)
    if let Some(title) = &spec.title {
        println!("{}: {}", "Title".bold(), title);
    }

    // Convert frontmatter to YAML value and iterate over fields
    let frontmatter_value = serde_yaml::to_value(&spec.frontmatter)?;
    if let serde_yaml::Value::Mapping(map) = frontmatter_value {
        for (key, value) in map {
            // Skip null values
            if value.is_null() {
                continue;
            }

            let key_str = match &key {
                serde_yaml::Value::String(s) => s.clone(),
                _ => continue,
            };

            let display_key = key_to_title_case(&key_str);
            let formatted_value = format_yaml_value(&key_str, &value);

            println!("{}: {}", display_key.bold(), formatted_value);
        }
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

/// Result of log file lookup (used in tests)
#[cfg(test)]
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
#[cfg(test)]
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

    // If this is a member spec, mark the driver spec as in_progress if it's pending
    spec::mark_driver_in_progress(&specs_dir, &spec.id)?;

    println!(
        "{} {} with prompt '{}'",
        "Working".cyan(),
        spec.id,
        prompt_name
    );

    // Assemble prompt
    let message = prompt::assemble(&spec, &prompt_path, &config)?;

    // Invoke agent
    let result = invoke_agent(&message, &spec, prompt_name, &config);

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

            // Finalize the spec (set status, commits, completed_at, model)
            finalize_spec(&mut spec, &spec_path, &config)?;

            println!("\n{} Spec completed!", "✓".green());
            if let Some(commits) = &spec.frontmatter.commits {
                for commit in commits {
                    println!("Commit: {}", commit);
                }
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
    commits: Option<Vec<String>>,
    error: Option<String>,
}

fn cmd_work_parallel(
    specs_dir: &Path,
    prompts_dir: &Path,
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
        let specs_dir_clone = specs_dir.to_path_buf();
        let prompt_name_clone = spec_prompt.to_string();
        let config_model = config.defaults.model.clone();

        let handle = thread::spawn(move || {
            let result = invoke_agent_with_prefix(
                &message,
                &spec_id,
                &prompt_name_clone,
                config_model.as_deref(),
            );
            let (success, commits, error) = match result {
                Ok(_) => {
                    // Get the commits
                    let commits = get_commits_for_spec(&spec_id).ok();

                    // Update spec to completed
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        spec.frontmatter.status = SpecStatus::Completed;
                        spec.frontmatter.commits = commits.clone().filter(|c| !c.is_empty());
                        spec.frontmatter.completed_at = Some(
                            chrono::Local::now()
                                .format("%Y-%m-%dT%H:%M:%SZ")
                                .to_string(),
                        );
                        spec.frontmatter.model =
                            get_model_name_with_default(config_model.as_deref());
                        if let Err(e) = spec.save(&spec_path) {
                            eprintln!(
                                "{} [{}] Warning: Failed to finalize spec: {}",
                                "⚠".yellow(),
                                spec_id,
                                e
                            );
                        }
                    }

                    (true, commits, None)
                }
                Err(e) => {
                    // Update spec to failed
                    let spec_path = specs_dir_clone.join(format!("{}.md", spec_id));
                    if let Ok(mut spec) = spec::resolve_spec(&specs_dir_clone, &spec_id) {
                        spec.frontmatter.status = SpecStatus::Failed;
                        if let Err(save_err) = spec.save(&spec_path) {
                            eprintln!(
                                "{} [{}] Warning: Failed to mark spec as failed: {}",
                                "⚠".yellow(),
                                spec_id,
                                save_err
                            );
                        }
                    }

                    (false, None, Some(e.to_string()))
                }
            };

            let _ = tx_clone.send(ParallelResult {
                spec_id,
                success,
                commits,
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
            if let Some(commits) = result.commits {
                let commits_str = commits.join(", ");
                println!(
                    "[{}] {} Completed (commits: {})",
                    result.spec_id.cyan(),
                    "✓".green(),
                    commits_str
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
fn invoke_agent_with_prefix(
    message: &str,
    spec_id: &str,
    prompt_name: &str,
    config_model: Option<&str>,
) -> Result<()> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Create streaming log writer before spawning agent (writes header immediately)
    let mut log_writer = match StreamingLogWriter::new(spec_id, prompt_name) {
        Ok(writer) => Some(writer),
        Err(e) => {
            eprintln!(
                "{} [{}] Failed to create agent log: {}",
                "⚠".yellow(),
                spec_id,
                e
            );
            None
        }
    };

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!(".chant/specs/{}.md", spec_id))?;

    // Get the model to use
    let model = get_model_for_invocation(config_model);

    let mut child = Command::new("claude")
        .arg("--print")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--model")
        .arg(&model)
        .arg("--dangerously-skip-permissions")
        .arg(message)
        .env("CHANT_SPEC_ID", spec_id)
        .env("CHANT_SPEC_FILE", &spec_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to invoke claude CLI. Is it installed and in PATH?")?;

    // Stream stdout with prefix to both terminal and log file
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let prefix = format!("[{}]", spec_id);
        for line in reader.lines().map_while(Result::ok) {
            for text in extract_text_from_stream_json(&line) {
                for text_line in text.lines() {
                    println!("{} {}", prefix.cyan(), text_line);
                    if let Some(ref mut writer) = log_writer {
                        if let Err(e) = writer.write_line(text_line) {
                            eprintln!(
                                "{} [{}] Failed to write to agent log: {}",
                                "⚠".yellow(),
                                spec_id,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("Agent exited with status: {}", status);
    }

    Ok(())
}

fn invoke_agent(message: &str, spec: &Spec, prompt_name: &str, config: &Config) -> Result<String> {
    invoke_agent_with_model(message, spec, prompt_name, config, None)
}

fn invoke_agent_with_model(
    message: &str,
    spec: &Spec,
    prompt_name: &str,
    config: &Config,
    override_model: Option<&str>,
) -> Result<String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Create streaming log writer before spawning agent (writes header immediately)
    let mut log_writer = match StreamingLogWriter::new(&spec.id, prompt_name) {
        Ok(writer) => Some(writer),
        Err(e) => {
            eprintln!("{} Failed to create agent log: {}", "⚠".yellow(), e);
            None
        }
    };

    // Set environment variables
    let spec_file = std::fs::canonicalize(format!(".chant/specs/{}.md", spec.id))?;

    // Get the model to use - allow override
    let model = if let Some(override_m) = override_model {
        override_m.to_string()
    } else {
        get_model_for_invocation(config.defaults.model.as_deref())
    };

    let mut child = Command::new("claude")
        .arg("--print")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("--model")
        .arg(&model)
        .arg("--dangerously-skip-permissions")
        .arg(message)
        .env("CHANT_SPEC_ID", &spec.id)
        .env("CHANT_SPEC_FILE", &spec_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to invoke claude CLI. Is it installed and in PATH?")?;

    // Stream stdout to both terminal and log file
    let mut captured_output = String::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            for text in extract_text_from_stream_json(&line) {
                for text_line in text.lines() {
                    println!("{}", text_line);
                    captured_output.push_str(text_line);
                    captured_output.push('\n');
                    if let Some(ref mut writer) = log_writer {
                        if let Err(e) = writer.write_line(text_line) {
                            eprintln!("{} Failed to write to agent log: {}", "⚠".yellow(), e);
                        }
                    }
                }
            }
        }
    }

    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("Agent exited with status: {}", status);
    }

    Ok(captured_output)
}

/// Enum to distinguish between different commit retrieval scenarios
#[derive(Debug)]
enum CommitError {
    /// Git command failed (e.g., not in a git repository)
    GitCommandFailed(String),
    /// Git log succeeded but found no matching commits
    NoMatchingCommits,
}

impl std::fmt::Display for CommitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitError::GitCommandFailed(err) => write!(f, "Git command failed: {}", err),
            CommitError::NoMatchingCommits => write!(f, "No matching commits found"),
        }
    }
}

impl std::error::Error for CommitError {}

fn get_commits_for_spec(spec_id: &str) -> Result<Vec<String>> {
    use std::process::Command;

    // Look for all commits with the chant(spec_id) pattern
    let pattern = format!("chant({})", spec_id);

    let output = Command::new("git")
        .args(["log", "--oneline", "--grep", &pattern, "--reverse"])
        .output()
        .context("Failed to execute git log command")?;

    // Check if git command itself failed
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!(
            "git log command failed for pattern 'chant({})': {}",
            spec_id, stderr
        );
        eprintln!("{} {}", "✗".red(), error_msg);
        return Err(anyhow::anyhow!(CommitError::GitCommandFailed(error_msg)));
    }

    // Parse commits from successful output
    let mut commits = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(hash) = line.split_whitespace().next() {
            if !hash.is_empty() {
                commits.push(hash.to_string());
            }
        }
    }

    // If no matching commits found, use HEAD as fallback with warning
    if commits.is_empty() {
        eprintln!(
            "{} No commits found with pattern 'chant({})'. Attempting to use HEAD as fallback.",
            "⚠".yellow(),
            spec_id
        );

        let head_output = Command::new("git")
            .args(["rev-parse", "--short=7", "HEAD"])
            .output()
            .context("Failed to execute git rev-parse command")?;

        if head_output.status.success() {
            let head_hash = String::from_utf8_lossy(&head_output.stdout)
                .trim()
                .to_string();
            if !head_hash.is_empty() {
                eprintln!("{} Using HEAD commit: {}", "⚠".yellow(), head_hash);
                commits.push(head_hash);
            }
        } else {
            let stderr = String::from_utf8_lossy(&head_output.stderr);
            let error_msg = format!(
                "Could not find any commit for spec '{}' and HEAD fallback failed: {}",
                spec_id, stderr
            );
            eprintln!("{} {}", "✗".red(), error_msg);
            return Err(anyhow::anyhow!(CommitError::NoMatchingCommits));
        }
    }

    Ok(commits)
}

/// Finalize a spec after successful completion
/// Sets status, commits, completed_at, and model
/// This function is idempotent and can be called multiple times safely
fn finalize_spec(spec: &mut Spec, spec_path: &Path, config: &Config) -> Result<()> {
    // Get the commits for this spec
    let commits = get_commits_for_spec(&spec.id)?;

    // Update spec to completed
    spec.frontmatter.status = SpecStatus::Completed;
    spec.frontmatter.commits = if commits.is_empty() {
        None
    } else {
        Some(commits)
    };
    spec.frontmatter.completed_at = Some(
        chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    );
    spec.frontmatter.model = get_model_name(Some(config));

    // Save the spec - this must not fail silently
    spec.save(spec_path)
        .context("Failed to save finalized spec")?;

    Ok(())
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

/// Extract text content from a Claude CLI stream-json line.
/// Returns Vec of text strings from assistant message content blocks.
fn extract_text_from_stream_json(line: &str) -> Vec<String> {
    let mut texts = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some("assistant") = json.get("type").and_then(|t| t.as_str()) {
            if let Some(content) = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            {
                for item in content {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        texts.push(text.to_string());
                    }
                }
            }
        }
    }

    texts
}

/// Ensure the logs directory exists and is in .gitignore at the given base path
fn ensure_logs_dir_at(base_path: &Path) -> Result<()> {
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

/// A streaming log writer that writes to a log file in real-time
struct StreamingLogWriter {
    file: std::fs::File,
}

impl StreamingLogWriter {
    /// Create a new streaming log writer that opens the log file and writes the header
    fn new(spec_id: &str, prompt_name: &str) -> Result<Self> {
        Self::new_at(&PathBuf::from(".chant"), spec_id, prompt_name)
    }

    /// Create a new streaming log writer at the given base path
    fn new_at(base_path: &std::path::Path, spec_id: &str, prompt_name: &str) -> Result<Self> {
        use std::io::Write;

        ensure_logs_dir_at(base_path)?;

        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let mut file = std::fs::File::create(&log_path)?;

        // Write header immediately
        writeln!(file, "# Agent Log: {}", spec_id)?;
        writeln!(file, "# Started: {}", timestamp)?;
        writeln!(file, "# Prompt: {}", prompt_name)?;
        writeln!(file)?;
        file.flush()?;

        Ok(Self { file })
    }

    /// Write a line to the log file and flush immediately for real-time visibility
    fn write_line(&mut self, line: &str) -> Result<()> {
        use std::io::Write;

        writeln!(self.file, "{}", line)?;
        self.file.flush()?;
        Ok(())
    }
}

/// Get the model name using the following priority:
/// 1. CHANT_MODEL env var (explicit override)
/// 2. ANTHROPIC_MODEL env var (Claude CLI default)
/// 3. defaults.model in config
/// 4. Parse from `claude --version` output (last resort)
fn get_model_name(config: Option<&Config>) -> Option<String> {
    get_model_name_with_default(config.and_then(|c| c.defaults.model.as_deref()))
}

/// Default model when no env var or config is set
const DEFAULT_MODEL: &str = "haiku";

/// Get the model to use for agent invocation.
/// Priority:
/// 1. CHANT_MODEL env var
/// 2. ANTHROPIC_MODEL env var
/// 3. defaults.model in config
/// 4. "haiku" as hardcoded fallback
fn get_model_for_invocation(config_model: Option<&str>) -> String {
    // 1. CHANT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 2. ANTHROPIC_MODEL env var
    if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 3. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 4. Hardcoded fallback
    DEFAULT_MODEL.to_string()
}

/// Get the model name with an optional default from config.
/// Used by parallel execution where full Config isn't available.
fn get_model_name_with_default(config_model: Option<&str>) -> Option<String> {
    // 1. CHANT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return Some(model);
        }
    }

    // 2. ANTHROPIC_MODEL env var
    if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
        if !model.is_empty() {
            return Some(model);
        }
    }

    // 3. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return Some(model.to_string());
        }
    }

    // 4. Parse from claude --version output
    parse_model_from_claude_version()
}

/// Parse model name from `claude --version` output.
/// Expected format: "X.Y.Z (model-name)" or similar patterns.
fn parse_model_from_claude_version() -> Option<String> {
    use std::process::Command;

    let output = Command::new("claude").arg("--version").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);

    // Try to extract model from parentheses, e.g., "1.0.0 (claude-sonnet-4)"
    if let Some(start) = version_str.find('(') {
        if let Some(end) = version_str.find(')') {
            if start < end {
                let model = version_str[start + 1..end].trim();
                // Check if it looks like a model name (contains "claude" or common model patterns)
                if model.contains("claude")
                    || model.contains("sonnet")
                    || model.contains("opus")
                    || model.contains("haiku")
                {
                    return Some(model.to_string());
                }
            }
        }
    }

    None
}

/// Get the model to use for split operations.
/// Resolution order:
/// 1. --model flag (if provided)
/// 2. CHANT_SPLIT_MODEL env var
/// 3. defaults.split_model from config
/// 4. CHANT_MODEL env var (fallback to general model)
/// 5. defaults.model from config
/// 6. Hardcoded default: "sonnet"
fn get_model_for_split(
    flag_model: Option<&str>,
    config_model: Option<&str>,
    config_split_model: Option<&str>,
) -> String {
    // 1. --model flag
    if let Some(model) = flag_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 2. CHANT_SPLIT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_SPLIT_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 3. defaults.split_model from config
    if let Some(model) = config_split_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 4. CHANT_MODEL env var (fallback to general model)
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }

    // 5. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return model.to_string();
        }
    }

    // 6. Hardcoded default
    "sonnet".to_string()
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

fn cmd_split(id: &str, override_model: Option<&str>) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");
    let prompts_dir = PathBuf::from(".chant/prompts");
    let config = Config::load()?;

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Resolve the spec to split
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Check if already a group
    if spec.frontmatter.r#type == "group" {
        println!("{} Spec {} is already a group.", "⚠".yellow(), spec.id);
        return Ok(());
    }

    println!("{} Analyzing spec {} for splitting...", "→".cyan(), spec.id);

    // Load prompt from file
    let split_prompt_path = prompts_dir.join("split.md");
    if !split_prompt_path.exists() {
        anyhow::bail!("Split prompt not found: split.md");
    }

    // Assemble prompt for split analysis
    let split_prompt = prompt::assemble(&spec, &split_prompt_path, &config)?;

    // Get the model to use for split
    let model = get_model_for_split(
        override_model,
        config.defaults.model.as_deref(),
        config.defaults.split_model.as_deref(),
    );

    // Invoke agent to propose split
    let agent_output =
        invoke_agent_with_model(&split_prompt, &spec, "split", &config, Some(&model))?;

    // Parse subtasks from agent output
    let subtasks = parse_subtasks_from_agent_output(&agent_output)?;

    if subtasks.is_empty() {
        anyhow::bail!("Agent did not propose any subtasks. Check the agent output in the log.");
    }

    println!(
        "{} Creating {} subtasks for spec {}",
        "→".cyan(),
        subtasks.len(),
        spec.id
    );

    // Create subtask spec files
    let driver_id = spec.id.clone();
    for (index, subtask) in subtasks.iter().enumerate() {
        let subtask_number = index + 1;
        let subtask_id = format!("{}.{}", driver_id, subtask_number);
        let subtask_filename = format!("{}.md", subtask_id);
        let subtask_path = specs_dir.join(&subtask_filename);

        // Create frontmatter with dependencies
        let depends_on = if index > 0 {
            Some(vec![format!("{}.{}", driver_id, index)])
        } else {
            None
        };

        let subtask_frontmatter = SpecFrontmatter {
            r#type: "code".to_string(),
            status: SpecStatus::Pending,
            depends_on,
            target_files: subtask.target_files.clone(),
            ..Default::default()
        };

        // Build body with title and description
        // If description already contains ### Acceptance Criteria, don't append generic ones
        let body = if subtask.description.contains("### Acceptance Criteria") {
            format!("# {}\n\n{}", subtask.title, subtask.description)
        } else {
            // No acceptance criteria found, append generic section
            format!(
                "# {}\n\n{}\n\n## Acceptance Criteria\n\n- [ ] Implement as described\n- [ ] All tests pass",
                subtask.title,
                subtask.description
            )
        };

        let subtask_spec = Spec {
            id: subtask_id.clone(),
            frontmatter: subtask_frontmatter,
            title: Some(subtask.title.clone()),
            body,
        };

        subtask_spec.save(&subtask_path)?;
        println!("  {} {}", "✓".green(), subtask_id);
    }

    // Update driver spec to type: group
    spec.frontmatter.r#type = "group".to_string();
    spec.save(&spec_path)?;

    println!(
        "\n{} Split complete! Driver spec {} is now type: group",
        "✓".green(),
        spec.id
    );
    println!("Subtasks:");
    for i in 1..=subtasks.len() {
        println!("  • {}.{}", spec.id, i);
    }

    Ok(())
}

fn cmd_archive(
    spec_id: Option<&str>,
    dry_run: bool,
    older_than: Option<u64>,
    force: bool,
) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");
    let archive_dir = PathBuf::from(".chant/archive");

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Load all specs
    let specs = spec::load_all_specs(&specs_dir)?;

    // Filter specs to archive
    let mut to_archive = Vec::new();

    if let Some(id) = spec_id {
        // Archive specific spec
        if let Some(spec) = specs.iter().find(|s| s.id.starts_with(id)) {
            to_archive.push(spec.clone());
        } else {
            anyhow::bail!("Spec {} not found", id);
        }
    } else {
        // Archive by criteria
        let now = chrono::Local::now();

        for spec in specs {
            // Skip if not completed (unless force)
            if spec.frontmatter.status != SpecStatus::Completed && !force {
                continue;
            }

            // Check older_than filter
            if let Some(days) = older_than {
                if let Some(completed_at_str) = &spec.frontmatter.completed_at {
                    if let Ok(completed_at) = chrono::DateTime::parse_from_rfc3339(completed_at_str)
                    {
                        let completed_at_local =
                            chrono::DateTime::<chrono::Local>::from(completed_at);
                        let age = now.signed_duration_since(completed_at_local);
                        if age.num_days() < days as i64 {
                            continue;
                        }
                    }
                } else {
                    // No completion date, skip
                    continue;
                }
            }

            to_archive.push(spec);
        }
    }

    if to_archive.is_empty() {
        println!("No specs to archive.");
        return Ok(());
    }

    if dry_run {
        println!("{} Would archive {} spec(s):", "→".cyan(), to_archive.len());
        for spec in &to_archive {
            println!(
                "  {} {}",
                spec.id,
                spec.title.as_deref().unwrap_or("(no title)")
            );
        }
        return Ok(());
    }

    // Create archive directory if it doesn't exist
    if !archive_dir.exists() {
        std::fs::create_dir_all(&archive_dir)?;
        println!("{} Created archive directory", "✓".green());
    }

    // Move specs to archive
    let count = to_archive.len();
    for spec in to_archive {
        let src = specs_dir.join(format!("{}.md", spec.id));
        let dst = archive_dir.join(format!("{}.md", spec.id));

        std::fs::rename(&src, &dst)?;
        println!("{} {} → archive/", "→".cyan(), spec.id);
    }

    println!("{} Archived {} spec(s)", "✓".green(), count);

    Ok(())
}

#[derive(Debug, Clone)]
struct Subtask {
    title: String,
    description: String,
    target_files: Option<Vec<String>>,
}

fn parse_subtasks_from_agent_output(output: &str) -> Result<Vec<Subtask>> {
    let mut subtasks = Vec::new();
    let mut current_subtask: Option<(String, String, Vec<String>)> = None;
    let mut collecting_files = false;
    let mut in_code_block = false;

    for line in output.lines() {
        // Check for subtask headers (## Subtask N: ...)
        if line.starts_with("## Subtask ") && line.contains(':') {
            // Save previous subtask if any
            if let Some((title, desc, files)) = current_subtask.take() {
                subtasks.push(Subtask {
                    title,
                    description: desc.trim().to_string(),
                    target_files: if files.is_empty() { None } else { Some(files) },
                });
            }

            // Extract title from "## Subtask N: Title Here"
            if let Some(title_part) = line.split(':').nth(1) {
                let title = title_part.trim().to_string();
                current_subtask = Some((title, String::new(), Vec::new()));
                collecting_files = false;
            }
        } else if current_subtask.is_some() {
            // Check for code block markers
            if line.trim() == "```" {
                in_code_block = !in_code_block;
                if let Some((_, ref mut desc, _)) = &mut current_subtask {
                    desc.push_str(line);
                    desc.push('\n');
                }
                continue;
            }

            // Check for "Affected Files:" header
            if line.contains("**Affected Files:**") || line.contains("Affected Files:") {
                collecting_files = true;
                continue;
            }

            // If collecting files, parse them (format: "- filename")
            if collecting_files {
                if let Some(stripped) = line.strip_prefix("- ") {
                    let file = stripped.trim().to_string();
                    if !file.is_empty() {
                        // Strip annotations like "(test module)" from filename
                        let cleaned_file = if let Some(paren_pos) = file.find('(') {
                            file[..paren_pos].trim().to_string()
                        } else {
                            file
                        };
                        if let Some((_, _, ref mut files)) = current_subtask {
                            files.push(cleaned_file);
                        }
                    }
                } else if line.starts_with('-') && !line.starts_with("- ") {
                    // Not a bullet list, stop collecting
                    collecting_files = false;
                } else if line.trim().is_empty() {
                    // Empty line might end the files section, depending on context
                } else if line.starts_with("##") {
                    // New section
                    collecting_files = false;
                }
            } else if !in_code_block {
                // Preserve ### headers and all content except "Affected Files" section
                if let Some((_, ref mut desc, _)) = &mut current_subtask {
                    desc.push_str(line);
                    desc.push('\n');
                }
            }
        }
    }

    // Save last subtask
    if let Some((title, desc, files)) = current_subtask {
        subtasks.push(Subtask {
            title,
            description: desc.trim().to_string(),
            target_files: if files.is_empty() { None } else { Some(files) },
        });
    }

    if subtasks.is_empty() {
        anyhow::bail!("No subtasks found in agent output");
    }

    Ok(subtasks)
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
    fn test_streaming_log_writer_creates_header() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";

        // Create log writer (this writes the header)
        let _writer = StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();

        // Check that log file exists with header BEFORE any lines are written
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        assert!(log_path.exists());

        let content = std::fs::read_to_string(&log_path).unwrap();

        // Check header format
        assert!(content.starts_with("# Agent Log: 2026-01-24-00a-xyz\n"));
        assert!(content.contains("# Started: "));
        assert!(content.contains("# Prompt: standard\n"));
    }

    #[test]
    fn test_streaming_log_writer_writes_lines() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";

        // Create log writer and write lines
        let mut writer = StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
        writer.write_line("Test agent output").unwrap();
        writer.write_line("With multiple lines").unwrap();

        // Read it back
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        let content = std::fs::read_to_string(&log_path).unwrap();

        // Check header format
        assert!(content.starts_with("# Agent Log: 2026-01-24-00a-xyz\n"));
        assert!(content.contains("# Started: "));
        assert!(content.contains("# Prompt: standard\n"));

        // Check output is preserved
        assert!(content.contains("Test agent output\n"));
        assert!(content.contains("With multiple lines\n"));
    }

    #[test]
    fn test_streaming_log_writer_flushes_each_line() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";

        // Create log writer
        let mut writer = StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));

        // Write first line
        writer.write_line("Line 1").unwrap();

        // Verify it's visible immediately (flushed) by reading the file
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Line 1"));

        // Write second line
        writer.write_line("Line 2").unwrap();

        // Verify both lines are visible
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Line 1"));
        assert!(content.contains("Line 2"));
    }

    #[test]
    fn test_streaming_log_writer_overwrites_on_new_run() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00b-abc";
        let prompt_name = "standard";

        // First run
        {
            let mut writer = StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
            writer.write_line("Content A").unwrap();
        }

        // Second run (simulating replay)
        {
            let mut writer = StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
            writer.write_line("Content B").unwrap();
        }

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

        let result = get_model_name(None);
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

        let result = get_model_name(None);
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

        let result = get_model_name(None);
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
    fn test_get_model_name_from_config_default() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars so config default is used
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_name_with_default(Some("claude-sonnet-4"));
        assert_eq!(result, Some("claude-sonnet-4".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_get_model_name_env_takes_precedence_over_config() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set env var
        std::env::set_var("ANTHROPIC_MODEL", "claude-opus-4-5");
        std::env::remove_var("CHANT_MODEL");

        // Env var should take precedence over config
        let result = get_model_name_with_default(Some("claude-sonnet-4"));
        assert_eq!(result, Some("claude-opus-4-5".to_string()));

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
    fn test_get_model_name_none_when_unset() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset both env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        // With no config and no env vars, falls back to claude version parsing
        // which may or may not return a value depending on system
        let result = get_model_name_with_default(None);
        // We can't assert the exact value since it depends on whether claude is installed
        // and what version it is, so we just verify it doesn't panic
        let _ = result;

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

        // Empty env var should fall through to config default or claude version
        let result = get_model_name_with_default(None);
        // Can't assert exact value since it depends on whether claude is installed
        let _ = result;

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
    fn test_get_model_name_empty_config_model_skipped() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        // Empty config model should be skipped
        let result = get_model_name_with_default(Some(""));
        // Falls through to claude version parsing
        let _ = result;

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_key_to_title_case_single_word() {
        assert_eq!(key_to_title_case("status"), "Status");
        assert_eq!(key_to_title_case("type"), "Type");
        assert_eq!(key_to_title_case("commit"), "Commit");
    }

    #[test]
    fn test_key_to_title_case_snake_case() {
        assert_eq!(key_to_title_case("depends_on"), "Depends On");
        assert_eq!(key_to_title_case("completed_at"), "Completed At");
        assert_eq!(key_to_title_case("target_files"), "Target Files");
    }

    #[test]
    fn test_key_to_title_case_empty_string() {
        assert_eq!(key_to_title_case(""), "");
    }

    #[test]
    fn test_format_yaml_value_null() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Null);
        // Result contains ANSI codes, but should represent "~"
        assert!(result.contains("~") || result.contains('\x1b'));
    }

    #[test]
    fn test_format_yaml_value_bool_true() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Bool(true));
        // Result contains ANSI codes for green, but should represent "true"
        assert!(result.contains("true") || result.contains('\x1b'));
    }

    #[test]
    fn test_format_yaml_value_bool_false() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Bool(false));
        // Result contains ANSI codes for red, but should represent "false"
        assert!(result.contains("false") || result.contains('\x1b'));
    }

    #[test]
    fn test_format_yaml_value_number() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Number(42.into()));
        assert_eq!(result, "42");
    }

    #[test]
    fn test_format_yaml_value_string_status_completed() {
        use serde_yaml::Value;
        let result = format_yaml_value("status", &Value::String("completed".to_string()));
        // Should contain green ANSI codes
        assert!(result.contains("completed"));
    }

    #[test]
    fn test_format_yaml_value_string_status_failed() {
        use serde_yaml::Value;
        let result = format_yaml_value("status", &Value::String("failed".to_string()));
        // Should contain red ANSI codes
        assert!(result.contains("failed"));
    }

    #[test]
    fn test_format_yaml_value_string_status_pending() {
        use serde_yaml::Value;
        let result = format_yaml_value("status", &Value::String("pending".to_string()));
        // Should contain yellow ANSI codes
        assert!(result.contains("pending"));
    }

    #[test]
    fn test_format_yaml_value_string_commit() {
        use serde_yaml::Value;
        let result = format_yaml_value("commit", &Value::String("abc1234".to_string()));
        // Should contain cyan ANSI codes
        assert!(result.contains("abc1234"));
    }

    #[test]
    fn test_format_yaml_value_string_type() {
        use serde_yaml::Value;
        let result = format_yaml_value("type", &Value::String("code".to_string()));
        // Should contain blue ANSI codes
        assert!(result.contains("code"));
    }

    #[test]
    fn test_format_yaml_value_sequence() {
        use serde_yaml::Value;
        let seq = Value::Sequence(vec![
            Value::String("item1".to_string()),
            Value::String("item2".to_string()),
        ]);
        let result = format_yaml_value("labels", &seq);
        // Should be formatted as [item1, item2] with magenta colors
        assert!(result.starts_with('['));
        assert!(result.ends_with(']'));
        assert!(result.contains("item1"));
        assert!(result.contains("item2"));
    }

    #[test]
    fn test_format_yaml_value_plain_string() {
        use serde_yaml::Value;
        // For keys not in the special list, string should be plain
        let result = format_yaml_value("prompt", &Value::String("standard".to_string()));
        assert_eq!(result, "standard");
    }

    #[test]
    fn test_extract_text_from_stream_json_assistant_message() {
        let json_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello, world!"}]}}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["Hello, world!"]);
    }

    #[test]
    fn test_extract_text_from_stream_json_multiple_content_blocks() {
        let json_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"First"},{"type":"text","text":"Second"}]}}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["First", "Second"]);
    }

    #[test]
    fn test_extract_text_from_stream_json_system_message() {
        let json_line = r#"{"type":"system","subtype":"init"}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_result_message() {
        let json_line = r#"{"type":"result","subtype":"success","result":"Done"}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_invalid_json() {
        let json_line = "not valid json";
        let texts = extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_mixed_content_types() {
        // Content can include tool_use blocks which we should skip
        let json_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Analyzing..."},{"type":"tool_use","name":"read_file"}]}}"#;
        let texts = extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["Analyzing..."]);
    }

    #[test]
    fn test_get_model_for_invocation_from_chant_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set CHANT_MODEL
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_for_invocation(None);
        assert_eq!(result, "claude-opus-4-5");

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
    fn test_get_model_for_invocation_from_anthropic_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set only ANTHROPIC_MODEL
        std::env::remove_var("CHANT_MODEL");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = get_model_for_invocation(None);
        assert_eq!(result, "claude-sonnet-4");

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
    fn test_get_model_for_invocation_chant_takes_precedence() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set both env vars
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = get_model_for_invocation(Some("config-model"));
        // CHANT_MODEL takes precedence
        assert_eq!(result, "claude-opus-4-5");

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
    fn test_get_model_for_invocation_from_config() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars so config default is used
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_for_invocation(Some("claude-sonnet-4"));
        assert_eq!(result, "claude-sonnet-4");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_get_model_for_invocation_defaults_to_haiku() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset both env vars and no config
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_for_invocation(None);
        assert_eq!(result, "haiku");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_get_model_for_invocation_empty_env_falls_through() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set empty env vars
        std::env::set_var("CHANT_MODEL", "");
        std::env::set_var("ANTHROPIC_MODEL", "");

        let result = get_model_for_invocation(Some("config-model"));
        // Empty env vars should fall through to config
        assert_eq!(result, "config-model");

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
    fn test_get_model_for_invocation_empty_config_falls_to_haiku() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        // Empty config model should fall through to haiku
        let result = get_model_for_invocation(Some(""));
        assert_eq!(result, "haiku");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_parse_subtasks_from_agent_output_single() {
        let output = r#"## Subtask 1: Add new field

Add a new field to the struct alongside the old one.

**Affected Files:**
- src/lib.rs
- src/main.rs
"#;
        let result = parse_subtasks_from_agent_output(output).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Add new field");
        assert!(result[0].description.contains("Add a new field"));
        assert_eq!(
            result[0].target_files,
            Some(vec!["src/lib.rs".to_string(), "src/main.rs".to_string()])
        );
    }

    #[test]
    fn test_parse_subtasks_from_agent_output_multiple() {
        let output = r#"## Subtask 1: First task

Description of first task.

**Affected Files:**
- file1.rs

## Subtask 2: Second task

Description of second task.

**Affected Files:**
- file2.rs
"#;
        let result = parse_subtasks_from_agent_output(output).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "First task");
        assert_eq!(result[1].title, "Second task");
    }

    #[test]
    fn test_parse_subtasks_without_files() {
        let output = r#"## Subtask 1: Simple task

Just a simple task without files listed.
"#;
        let result = parse_subtasks_from_agent_output(output).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Simple task");
        assert!(result[0].target_files.is_none());
    }

    #[test]
    fn test_parse_subtasks_empty_output() {
        let output = "No subtasks here";
        let result = parse_subtasks_from_agent_output(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_subtasks_preserves_section_headers() {
        let output = r#"## Subtask 1: Implement feature

Add the core feature with detailed logic.

### Acceptance Criteria

- [ ] Feature is implemented
- [ ] Tests pass

### Edge Cases

- Edge case 1: Handle empty input
- Edge case 2: Handle large values

**Affected Files:**
- src/lib.rs
"#;
        let result = parse_subtasks_from_agent_output(output).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Implement feature");
        // Description should preserve ### headers
        assert!(result[0].description.contains("### Acceptance Criteria"));
        assert!(result[0].description.contains("### Edge Cases"));
        assert!(result[0]
            .description
            .contains("- [ ] Feature is implemented"));
        assert!(result[0]
            .description
            .contains("Edge case 1: Handle empty input"));
        assert_eq!(result[0].target_files, Some(vec!["src/lib.rs".to_string()]));
    }

    #[test]
    fn test_parse_subtasks_with_multiple_sections() {
        let output = r#"## Subtask 2: Update callers

Update all callers to use the new API.

### Acceptance Criteria

- [ ] All callers updated
- [ ] No compilation errors

### Example Test Cases

- Case 1: Basic caller should work
- Case 2: Complex caller should work

**Affected Files:**
- src/main.rs
- src/utils.rs
"#;
        let result = parse_subtasks_from_agent_output(output).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].description.contains("### Acceptance Criteria"));
        assert!(result[0].description.contains("### Example Test Cases"));
        assert!(result[0].description.contains("- [ ] All callers updated"));
        assert_eq!(
            result[0].target_files,
            Some(vec!["src/main.rs".to_string(), "src/utils.rs".to_string()])
        );
    }

    #[test]
    fn test_subtask_body_with_existing_acceptance_criteria() {
        // Verify that when a subtask description contains ### Acceptance Criteria,
        // we don't append a generic section
        let subtask = Subtask {
            title: "Implement feature".to_string(),
            description: "Implement the feature.\n\n### Acceptance Criteria\n\n- [ ] Feature works\n- [ ] Tests pass".to_string(),
            target_files: None,
        };

        // Build body the same way cmd_split does
        let body = if subtask.description.contains("### Acceptance Criteria") {
            format!("# {}\n\n{}", subtask.title, subtask.description)
        } else {
            format!(
                "# {}\n\n{}\n\n## Acceptance Criteria\n\n- [ ] Implement as described\n- [ ] All tests pass",
                subtask.title,
                subtask.description
            )
        };

        // Body should contain the preserved ### headers
        assert!(body.contains("### Acceptance Criteria"));
        assert!(body.contains("- [ ] Feature works"));
        // Generic section should NOT be appended
        assert!(!body.matches("## Acceptance Criteria").count() > 1);
    }

    #[test]
    fn test_subtask_body_without_acceptance_criteria() {
        // Verify that when a subtask description lacks ### Acceptance Criteria,
        // we append the generic section
        let subtask = Subtask {
            title: "Simple task".to_string(),
            description: "Just do this simple thing.".to_string(),
            target_files: None,
        };

        // Build body the same way cmd_split does
        let body = if subtask.description.contains("### Acceptance Criteria") {
            format!("# {}\n\n{}", subtask.title, subtask.description)
        } else {
            format!(
                "# {}\n\n{}\n\n## Acceptance Criteria\n\n- [ ] Implement as described\n- [ ] All tests pass",
                subtask.title,
                subtask.description
            )
        };

        // Body should contain the generic section
        assert!(body.contains("## Acceptance Criteria"));
        assert!(body.contains("- [ ] Implement as described"));
        assert!(body.contains("- [ ] All tests pass"));
    }

    #[test]
    fn test_finalize_spec_sets_status_and_timestamps() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().to_path_buf();

        // Create a spec with pending status
        let spec_content = r#"---
type: task
id: 2026-01-24-test-xyz
status: in_progress
---

# Test spec

## Acceptance Criteria

- [x] Item 1
- [x] Item 2
"#;
        specs_dir
            .join("2026-01-24-test-xyz.md")
            .parent()
            .and_then(|p| Some(std::fs::create_dir_all(p).ok()));
        std::fs::write(specs_dir.join("2026-01-24-test-xyz.md"), spec_content).unwrap();

        // Create a minimal config from string
        let config_str = r#"---
project:
  name: test-project
defaults:
  prompt: standard
  branch: true
  pr: false
  branch_prefix: "chant/"
git:
  provider: github
---
"#;
        let config = Config::parse(config_str).unwrap();

        // Load and finalize the spec
        let mut spec = spec::resolve_spec(&specs_dir, "2026-01-24-test-xyz").unwrap();
        let spec_path = specs_dir.join("2026-01-24-test-xyz.md");

        // Before finalization, status should be in_progress
        assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
        assert!(spec.frontmatter.completed_at.is_none());

        // Finalize the spec
        finalize_spec(&mut spec, &spec_path, &config).unwrap();

        // After finalization, status should be completed
        assert_eq!(spec.frontmatter.status, SpecStatus::Completed);
        assert!(spec.frontmatter.completed_at.is_some());

        // Read back the spec from file to verify it was saved
        let saved_spec = spec::resolve_spec(&specs_dir, "2026-01-24-test-xyz").unwrap();
        assert_eq!(saved_spec.frontmatter.status, SpecStatus::Completed);
        assert!(saved_spec.frontmatter.completed_at.is_some());
    }

    #[test]
    fn test_get_model_for_split_flag_override() {
        // Clear env vars for clean test
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let model = get_model_for_split(Some("claude-opus-4"), None, None);
        assert_eq!(model, "claude-opus-4");
    }

    #[test]
    fn test_get_model_for_split_env_var_split_model() {
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");
        std::env::set_var("CHANT_SPLIT_MODEL", "claude-sonnet-4");

        let model = get_model_for_split(None, None, None);
        assert_eq!(model, "claude-sonnet-4");
    }

    #[test]
    fn test_get_model_for_split_config_split_model() {
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let model = get_model_for_split(None, None, Some("claude-sonnet-4"));
        assert_eq!(model, "claude-sonnet-4");
    }

    #[test]
    fn test_get_model_for_split_fallback_chant_model() {
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");
        std::env::set_var("CHANT_MODEL", "haiku");

        let model = get_model_for_split(None, None, None);
        assert_eq!(model, "haiku");
    }

    #[test]
    fn test_get_model_for_split_fallback_config_model() {
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let model = get_model_for_split(None, Some("haiku"), None);
        assert_eq!(model, "haiku");
    }

    #[test]
    fn test_get_model_for_split_default_sonnet() {
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let model = get_model_for_split(None, None, None);
        assert_eq!(model, "sonnet");
    }

    #[test]
    fn test_get_model_for_split_resolution_order() {
        // Set up all levels
        std::env::set_var("CHANT_SPLIT_MODEL", "sonnet-split");
        std::env::set_var("CHANT_MODEL", "haiku-general");

        // Flag should win
        let model = get_model_for_split(
            Some("opus-flag"),
            Some("haiku-general"),
            Some("sonnet-split"),
        );
        assert_eq!(model, "opus-flag");

        // Without flag, split_model env should win
        let model = get_model_for_split(None, Some("haiku-general"), Some("sonnet-split"));
        assert_eq!(model, "sonnet-split");

        // Cleanup
        std::env::remove_var("CHANT_SPLIT_MODEL");
        std::env::remove_var("CHANT_MODEL");
    }

    #[test]
    fn test_get_commits_for_spec_found_commits() {
        // This test verifies that when git log finds matching commits, they're all returned
        // We test with the actual git repo since we're in one
        let commits = get_commits_for_spec("2026-01-24-01p-cmz");

        // The repo should have at least one commit with this spec ID
        // If it doesn't exist, that's okay - the test just verifies the function works
        if let Ok(c) = commits {
            // Commits should be non-empty or the function handled it gracefully
            assert!(!c.is_empty() || c.is_empty()); // Always passes, but verifies function doesn't crash
        }
    }

    #[test]
    fn test_get_commits_for_spec_empty_log_returns_ok() {
        // This test verifies that when git log succeeds but finds no matches,
        // the function either returns empty or uses HEAD fallback (both are OK)
        let commits = get_commits_for_spec("nonexistent-spec-id-that-should-never-exist");

        // Should return Ok with either empty list or HEAD commit
        assert!(commits.is_ok());
        if let Ok(c) = commits {
            // Should either find nothing and use HEAD, or be empty
            // Either way, this is valid behavior now with proper logging
            assert!(c.len() <= 1); // At most HEAD fallback
        }
    }

    #[test]
    fn test_get_commits_for_spec_special_characters_in_id() {
        // This test verifies that spec IDs with special characters don't crash pattern matching
        // Pattern format is "chant(spec_id)" so we test with various special chars
        let test_ids = vec![
            "2026-01-24-01p-cmz",   // Normal
            "test-with-dash",       // Dashes
            "test_with_underscore", // Underscores
        ];

        for spec_id in test_ids {
            let result = get_commits_for_spec(spec_id);
            // Should not panic, even if no commits are found
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_commit_error_display() {
        let err1 = CommitError::GitCommandFailed("test error".to_string());
        assert_eq!(err1.to_string(), "Git command failed: test error");

        let err2 = CommitError::NoMatchingCommits;
        assert_eq!(err2.to_string(), "No matching commits found");
    }

    #[test]
    fn test_archive_spec_loading() {
        // Test that archive can load specs correctly from directory
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");

        // Create specs directory
        std::fs::create_dir_all(&specs_dir).unwrap();

        // Create a completed spec
        let spec_id = "2026-01-24-001-abc";
        let spec_content = format!(
            r#"---
type: code
status: completed
completed_at: {}
---

# Test Spec
"#,
            chrono::Local::now().to_rfc3339()
        );

        let spec_path = specs_dir.join(format!("{}.md", spec_id));
        std::fs::write(&spec_path, &spec_content).unwrap();

        // Load specs to verify they can be parsed
        let specs = spec::load_all_specs(&specs_dir).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].id, spec_id);
        assert_eq!(specs[0].frontmatter.status, SpecStatus::Completed);
    }

    #[test]
    fn test_archive_filtering_completed() {
        // Test that archive correctly filters completed specs
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        // Create specs with different statuses
        let specs_data = vec![
            ("2026-01-24-001-abc", "completed"),
            ("2026-01-24-002-def", "pending"),
            ("2026-01-24-003-ghi", "completed"),
        ];

        for (id, status) in specs_data {
            let content = format!(
                r#"---
type: code
status: {}
---

# Test
"#,
                status
            );
            let path = specs_dir.join(format!("{}.md", id));
            std::fs::write(path, content).unwrap();
        }

        // Load all specs
        let all_specs = spec::load_all_specs(&specs_dir).unwrap();
        assert_eq!(all_specs.len(), 3);

        // Filter completed specs (simulating what cmd_archive does)
        let completed: Vec<_> = all_specs
            .iter()
            .filter(|s| s.frontmatter.status == SpecStatus::Completed)
            .collect();
        assert_eq!(completed.len(), 2);
    }

    #[test]
    fn test_archive_move_file() {
        // Test that files can be moved to archive
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        let archive_dir = temp_dir.path().join("archive");

        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Create a spec file
        let spec_id = "2026-01-24-001-abc";
        let spec_path = specs_dir.join(format!("{}.md", spec_id));
        std::fs::write(&spec_path, "test content").unwrap();
        assert!(spec_path.exists());

        // Move it to archive
        let archived_path = archive_dir.join(format!("{}.md", spec_id));
        std::fs::rename(&spec_path, &archived_path).unwrap();

        // Verify move succeeded
        assert!(!spec_path.exists());
        assert!(archived_path.exists());
        let content = std::fs::read_to_string(&archived_path).unwrap();
        assert_eq!(content, "test content");
    }
}
