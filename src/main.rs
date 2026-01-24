mod config;
mod id;
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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(name),
        Commands::Add { description } => cmd_add(&description),
        Commands::List { ready } => cmd_list(ready),
        Commands::Show { id } => cmd_show(&id),
        Commands::Work { id, prompt, branch } => cmd_work(&id, prompt.as_deref(), branch),
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

fn cmd_list(ready_only: bool) -> Result<()> {
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

    if specs.is_empty() {
        if ready_only {
            println!("No ready specs.");
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

fn cmd_work(id: &str, prompt_name: Option<&str>, create_branch: bool) -> Result<()> {
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

    // Handle branch creation/switching if requested
    if create_branch {
        let branch_name = format!("{}{}", config.defaults.branch_prefix, spec.id);
        create_or_switch_branch(&branch_name)?;
        spec.frontmatter.branch = Some(branch_name.clone());
        println!("{} Branch: {}", "→".cyan(), branch_name);
    }

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
            spec.save(&spec_path)?;

            println!("\n{} Spec completed!", "✓".green());
            if let Some(commit) = &spec.frontmatter.commit {
                println!("Commit: {}", commit);
            }
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
