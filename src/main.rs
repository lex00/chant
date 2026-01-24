mod config;
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
        /// Spec ID (full or partial)
        id: String,
        /// Prompt to use
        #[arg(long)]
        prompt: Option<String>,
        /// Create a feature branch before executing
        #[arg(long)]
        branch: bool,
        /// Create a pull request after spec completes
        #[arg(long)]
        pr: bool,
    },
    /// Start MCP server (Model Context Protocol)
    Mcp,
    /// Show project status summary
    Status,
    /// Show ready specs (shortcut for `list --ready`)
    Ready,
    /// Validate all specs for common issues
    Lint,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(name),
        Commands::Add { description } => cmd_add(&description),
        Commands::List { ready, label } => cmd_list(ready, &label),
        Commands::Show { id } => cmd_show(&id),
        Commands::Work { id, prompt, branch, pr } => cmd_work(&id, prompt.as_deref(), branch, pr),
        Commands::Mcp => mcp::run_server(),
        Commands::Status => cmd_status(),
        Commands::Ready => cmd_list(true, &[]),
        Commands::Lint => cmd_lint(),
    }
}

fn cmd_init(name: Option<String>) -> Result<()> {
    let chant_dir = PathBuf::from(".chant");

    if chant_dir.exists() {
        println!("{}", "Chant already initialized.".yellow());
        return Ok(());
    }

    // Detect project name
    let project_name = name.unwrap_or_else(|| detect_project_name().unwrap_or_else(|| "my-project".to_string()));

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
                    return Some(module.split('/').last().unwrap_or(module).to_string());
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
    println!("{}", "============");
    println!("  {:<12} {}", "Pending:", pending);
    println!("  {:<12} {}", "In Progress:", in_progress);
    println!("  {:<12} {}", "Completed:", completed);
    println!("  {:<12} {}", "Failed:", failed);
    println!("  {}", "─────────────");
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

fn cmd_work(id: &str, prompt_name: Option<&str>, cli_branch: bool, cli_pr: bool) -> Result<()> {
    let specs_dir = PathBuf::from(".chant/specs");
    let prompts_dir = PathBuf::from(".chant/prompts");
    let config = Config::load()?;

    if !specs_dir.exists() {
        anyhow::bail!("Chant not initialized. Run `chant init` first.");
    }

    // Resolve spec
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Check if already completed
    if spec.frontmatter.status == SpecStatus::Completed {
        println!("{} Spec already completed.", "⚠".yellow());
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
                    Some(d) => blocking.push(format!("{} ({:?})", dep_id, d.frontmatter.status).to_lowercase()),
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

    println!("{} {} with prompt '{}'", "Working".cyan(), spec.id, prompt_name);

    // Assemble prompt
    let message = prompt::assemble(&spec, &prompt_path, &config)?;

    // Invoke agent
    let result = invoke_agent(&message, &spec);

    match result {
        Ok(()) => {
            // Get the commit hash
            let commit = get_latest_commit_for_spec(&spec.id)?;

            // Update spec to completed
            let mut spec = spec::resolve_spec(&specs_dir, &spec.id)?;
            spec.frontmatter.status = SpecStatus::Completed;
            spec.frontmatter.commit = commit;
            spec.frontmatter.completed_at = Some(chrono::Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());

            println!("\n{} Spec completed!", "✓".green());
            if let Some(commit) = &spec.frontmatter.commit {
                println!("Commit: {}", commit);
            }

            // Create PR if requested
            if create_pr {
                let branch_name = branch_name.as_ref().expect("branch_name should exist when create_pr is true");
                println!("\n{} Pushing branch to remote...", "→".cyan());
                push_branch(branch_name)?;

                println!("{} Creating pull request...", "→".cyan());
                let pr_title = spec.title.clone().unwrap_or_else(|| spec.id.clone());
                let pr_body = spec.body.clone();
                let pr_url = create_pull_request(&pr_title, &pr_body)?;

                spec.frontmatter.pr = Some(pr_url.clone());
                println!("{} PR created: {}", "✓".green(), pr_url);
            }

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

fn invoke_agent(message: &str, spec: &Spec) -> Result<()> {
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

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }
    }

    let status = child.wait()?;

    if !status.success() {
        anyhow::bail!("Agent exited with status: {}", status);
    }

    Ok(())
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
            return Ok(Some(hash.to_string()));
        }
    }

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
    anyhow::bail!("Failed to create or switch to branch '{}': {}", branch_name, stderr)
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

fn create_pull_request(title: &str, body: &str) -> Result<String> {
    use std::process::Command;

    let output = Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", body])
        .output()
        .context("Failed to run gh pr create. Is gh CLI installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create pull request: {}", stderr);
    }

    let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(pr_url)
}
