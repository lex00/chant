//! Initialize chant in a project directory
//!
//! This module handles project initialization, including:
//! - Creating .chant directory structure
//! - Writing config.md with project settings
//! - Setting up prompt templates
//! - Configuring agent files (CLAUDE.md, .cursorrules, etc.)
//! - Setting up MCP configuration
//! - Configuring git merge drivers

use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::templates;

/// Parse provider string from user input
fn parse_provider_string(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "claude" | "claude-cli" => Some("claude"),
        "ollama" | "local" => Some("ollama"),
        "openai" | "gpt" => Some("openai"),
        "kirocli" | "kiro-cli-chat" | "kiro" => Some("kirocli"),
        _ => None,
    }
}

/// Infer provider from agent names if any agent name matches a known provider
fn infer_provider_from_agents(agents: &[String]) -> Option<String> {
    for agent in agents {
        if let Some(provider) = parse_provider_string(agent) {
            return Some(provider.to_string());
        }
    }
    None
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
    force_overwrite: bool,
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
    if target_path.exists() && !force_overwrite {
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
                "--force-overwrite".cyan()
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

    if target_path.exists() && force_overwrite {
        Ok(AgentFileResult::Updated)
    } else {
        Ok(AgentFileResult::Created)
    }
}

/// Handle updating only agent configuration files (used for re-running init with --agent)
fn handle_agent_update(chant_dir: &Path, agents: &[String], force_overwrite: bool) -> Result<()> {
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
            ".kiro/rules.md" => {
                std::fs::create_dir_all(".kiro")?;
                PathBuf::from(".kiro/rules.md")
            }
            filename => PathBuf::from(filename),
        };

        // Write the agent config file using the helper
        let result =
            write_agent_config_file(provider, &template, &target_path, force_overwrite, has_mcp)?;

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

    // Write chant skill to each provider's skills directory (Agent Skills open standard)
    let skill_content = templates::get_chant_skill();
    for provider in &parsed_agents {
        if let Some(skills_dir) = provider.skills_dir() {
            let skill_dir = PathBuf::from(skills_dir).join("chant");
            let skill_path = skill_dir.join("SKILL.md");

            if !skill_path.exists() || force_overwrite {
                std::fs::create_dir_all(&skill_dir)?;
                std::fs::write(&skill_path, skill_content)?;
                created_agents.push((skill_path, "skill"));
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
            if !mcp_path.exists() || force_overwrite {
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

/// Create a global CLAUDE.md next to the chant binary
///
/// This allows Claude Code to discover chant instructions system-wide,
/// even in projects without their own CLAUDE.md.
///
/// The file is written to the directory containing the chant binary
/// (typically ~/.cargo/bin/ for cargo installs).
fn create_global_claude_md(has_mcp: bool) -> Result<()> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Could not determine executable path: {}", e))?;

    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not determine executable directory"))?;

    let global_claude_path = exe_dir.join("CLAUDE.md");

    // Use the same injection logic to preserve any existing content
    let existing_content = if global_claude_path.exists() {
        Some(std::fs::read_to_string(&global_claude_path)?)
    } else {
        None
    };

    let result = templates::inject_chant_section(existing_content.as_deref(), has_mcp);

    match result {
        templates::InjectionResult::Created(content) => {
            std::fs::write(&global_claude_path, content)?;
            println!(
                "{} {} (global)",
                "Created".green(),
                global_claude_path.display()
            );
        }
        templates::InjectionResult::Appended(content)
        | templates::InjectionResult::Replaced(content) => {
            std::fs::write(&global_claude_path, content)?;
            println!(
                "{} {} (global)",
                "Updated".green(),
                global_claude_path.display()
            );
        }
        templates::InjectionResult::Unchanged => {
            // Already up-to-date, no action needed
        }
    }

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

/// Update the global Claude MCP config at ~/.claude/settings.json
///
/// This function:
/// - Creates ~/.claude/ directory if it doesn't exist
/// - Creates a new settings.json if it doesn't exist
/// - Merges with existing settings.json without overwriting other servers
/// - Creates a backup if the existing file has invalid JSON
fn update_claude_mcp_config() -> Result<McpConfigResult> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let global_mcp_path = home_dir.join(".claude").join("settings.json");

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
                let backup_path = home_dir.join(".claude").join("settings.json.backup");
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

/// Configure kiro-cli-chat MCP server with chant
fn configure_kirocli_mcp() -> Result<()> {
    // Check if kiro-cli-chat is installed
    let kiro_check = std::process::Command::new("which")
        .arg("kiro-cli-chat")
        .output();

    if let Ok(output) = kiro_check {
        if !output.status.success() {
            anyhow::bail!("kiro-cli-chat not found. Please install it first.");
        }
    } else {
        anyhow::bail!("Failed to check for kiro-cli-chat installation");
    }

    // Get chant binary path
    let chant_path = std::env::current_exe()
        .ok()
        .or_else(|| {
            std::process::Command::new("which")
                .arg("chant")
                .output()
                .ok()
                .and_then(|out| {
                    if out.status.success() {
                        String::from_utf8(out.stdout)
                            .ok()
                            .map(|s| PathBuf::from(s.trim()))
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| PathBuf::from("chant"));

    // Run kiro-cli-chat mcp add
    let output = std::process::Command::new("kiro-cli-chat")
        .args([
            "mcp",
            "add",
            "--name",
            "chant",
            "--command",
            chant_path.to_str().unwrap_or("chant"),
            "--args",
            "mcp",
            "--scope",
            "global",
            "--force",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Invalid") || stderr.contains("Grant") || stderr.contains("auth") {
            anyhow::bail!(
                "Kiro CLI authentication error. Try running 'kiro-cli-chat auth login' first.\nError: {}",
                stderr.trim()
            );
        }
        anyhow::bail!("Failed to configure Kiro CLI MCP server: {}", stderr.trim());
    }

    println!(
        "{} Configured kiro-cli-chat MCP server for chant",
        "✓".green()
    );
    println!(
        "{} Restart kiro-cli-chat to activate MCP integration",
        "ℹ".cyan()
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_init(
    subcommand: Option<&str>,
    name: Option<String>,
    silent: bool,
    force_overwrite: bool,
    minimal: bool,
    agents: Vec<String>,
    provider: Option<String>,
    model: Option<String>,
    _merge_driver: bool,
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
    if already_initialized && !force_overwrite {
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
                    anyhow::anyhow!(
                        "Invalid provider: {}. Use claude, ollama, openai, or kirocli.",
                        prov
                    )
                })?;

                // Special handling for kirocli: configure MCP instead of updating config
                if normalized == "kirocli" {
                    return configure_kirocli_mcp();
                }

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
            return handle_agent_update(&chant_dir, &agents, force_overwrite);
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
                        "Kiro (.kiro/rules.md)",
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
                        2 => vec!["kiro".to_string()],
                        3 => vec!["generic".to_string()],
                        4 => vec!["all".to_string()],
                        _ => vec![],
                    };

                    return handle_agent_update(&chant_dir, &selected_agents, force_overwrite);
                }
                1 => {
                    // Change default model provider
                    let provider_options = vec![
                        "Claude CLI (recommended)",
                        "Ollama (local)",
                        "OpenAI API",
                        "Kiro CLI",
                    ];

                    let provider_selection = dialoguer::Select::new()
                        .with_prompt("Default model provider?")
                        .items(&provider_options)
                        .default(0)
                        .interact()?;

                    let selected_provider = match provider_selection {
                        0 => "claude",
                        1 => "ollama",
                        2 => "openai",
                        3 => "kirocli",
                        _ => "claude",
                    };

                    update_config_field(&config_path, "provider", selected_provider)?;
                    println!(
                        "{} Updated provider to: {}",
                        "✓".green(),
                        selected_provider.cyan()
                    );

                    // Configure MCP for kirocli
                    if selected_provider == "kirocli" {
                        if let Err(e) = configure_kirocli_mcp() {
                            eprintln!("{} Failed to configure Kiro CLI MCP: {}", "✗".red(), e);
                        }
                    }
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
                        0 => "opus".to_string(),
                        1 => "sonnet".to_string(),
                        2 => "haiku".to_string(),
                        3 => dialoguer::Input::new()
                            .with_prompt("Custom model name")
                            .interact_text()?,
                        _ => "sonnet".to_string(),
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
        if !silent {
            println!("{}", "Chant already initialized.".yellow());
        }
        return Ok(());
    }

    // Detect if we're in wizard mode (no flags provided for fresh init)
    let is_wizard_mode = name.is_none()
        && !silent
        && !force_overwrite
        && !minimal
        && agents.is_empty()
        && provider.is_none()
        && model.is_none();

    // Gather parameters - either from wizard or from flags
    let (
        final_name,
        final_silent,
        final_minimal,
        final_agents,
        final_provider,
        final_model,
        final_setup_kirocli,
    ) = if is_wizard_mode && atty::is(atty::Stream::Stdin) {
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
        let provider_options = vec![
            "Claude CLI (recommended)",
            "Ollama (local)",
            "OpenAI API",
            "Kiro CLI",
        ];

        let provider_selection = dialoguer::Select::new()
            .with_prompt("Default model provider?")
            .items(&provider_options)
            .default(0)
            .interact()?;

        let selected_provider = match provider_selection {
            0 => Some("claude".to_string()),
            1 => Some("ollama".to_string()),
            2 => Some("openai".to_string()),
            3 => Some("kirocli".to_string()),
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
            0 => Some("opus".to_string()),
            1 => Some("sonnet".to_string()),
            2 => Some("haiku".to_string()),
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
            "Kiro (.kiro/rules.md)",
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
            3 => vec!["kiro".to_string()],
            4 => vec!["generic".to_string()],
            5 => vec!["all".to_string()],
            _ => vec![],
        };

        // Setup kiro MCP if kirocli provider selected, or prompt if kiro-cli-chat is installed
        let setup_kirocli = if selected_provider.as_deref() == Some("kirocli") {
            // Automatically setup MCP when kirocli provider is selected
            true
        } else if std::process::Command::new("which")
            .arg("kiro-cli-chat")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            // Ask about MCP setup if kiro-cli-chat is installed but not selected as provider
            dialoguer::Confirm::new()
                .with_prompt("Configure Kiro CLI MCP server for chant?")
                .default(true)
                .interact()?
        } else {
            false
        };

        (
            project_name,
            enable_silent,
            !include_templates, // invert: minimal is "no templates"
            selected_agents,
            selected_provider,
            selected_model,
            setup_kirocli,
        )
    } else {
        // Direct mode: use provided values
        let project_name = name
            .unwrap_or_else(|| detect_project_name().unwrap_or_else(|| "my-project".to_string()));

        // Validate provider if specified, otherwise infer from agent
        let validated_provider = if let Some(ref p) = provider {
            let normalized = parse_provider_string(p).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid provider: {}. Use claude, ollama, openai, or kirocli.",
                    p
                )
            })?;
            Some(normalized.to_string())
        } else {
            // Infer provider from agent if no explicit provider given
            infer_provider_from_agents(&agents)
        };

        // Check if we should configure kirocli in direct mode
        let setup_kirocli = validated_provider.as_deref() == Some("kirocli");

        (
            project_name,
            silent,
            minimal,
            agents,
            validated_provider,
            model,
            setup_kirocli,
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

    if chant_dir.exists() && force_overwrite {
        // force_overwrite flag: do full reinitialization (preserve specs/config)
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
        std::fs::create_dir_all(chant_dir.join("logs"))?;
        std::fs::create_dir_all(chant_dir.join("processes"))?;
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

    // Create directory structure (only if not already created during force_overwrite/restore)
    std::fs::create_dir_all(chant_dir.join("specs"))?;
    std::fs::create_dir_all(chant_dir.join("prompts"))?;
    std::fs::create_dir_all(chant_dir.join("logs"))?;
    std::fs::create_dir_all(chant_dir.join("processes"))?;
    std::fs::create_dir_all(chant_dir.join(".locks"))?;
    std::fs::create_dir_all(chant_dir.join(".store"))?;

    // Create config.md only if it doesn't exist (preserve during --force-overwrite)
    let config_path = chant_dir.join("config.md");
    if !config_path.exists() {
        // Build defaults section with optional provider and model
        let mut defaults_lines = vec!["  prompt: standard".to_string()];
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
        let gitignore_content = "# Local state (not shared)\n.locks/\n.store/\nstore/\nlogs/\nprocesses/\n\n# Agent configuration (contains API keys, not shared)\nagents.md\n";
        std::fs::write(&gitignore_path, gitignore_content)?;
    }

    // Set up the merge driver for spec files (handles .gitattributes and git config)
    // This ensures branch mode works correctly by auto-resolving frontmatter conflicts
    let merge_driver_result = chant::merge_driver::setup_merge_driver();
    let merge_driver_warning = match &merge_driver_result {
        Ok(result) => result.warning.clone(),
        Err(e) => Some(format!("Failed to set up merge driver: {}", e)),
    };
    let merge_driver_result_opt = Some(merge_driver_result);

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
                ".kiro/rules.md" => {
                    // Create .kiro directory in root
                    std::fs::create_dir_all(".kiro")?;
                    PathBuf::from(".kiro/rules.md")
                }
                filename => {
                    // Other providers: write to root
                    PathBuf::from(filename)
                }
            };

            // Write the agent config file using the helper
            let result = write_agent_config_file(
                provider,
                &template,
                &target_path,
                force_overwrite,
                will_have_mcp,
            )?;

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

        // Write chant skill to each provider's skills directory (Agent Skills open standard)
        let skill_content = templates::get_chant_skill();
        for provider in &parsed_agents {
            if let Some(skills_dir) = provider.skills_dir() {
                let skill_dir = PathBuf::from(skills_dir).join("chant");
                let skill_path = skill_dir.join("SKILL.md");

                if !skill_path.exists() || force_overwrite {
                    std::fs::create_dir_all(&skill_dir)?;
                    std::fs::write(&skill_path, skill_content)?;
                    println!("{} {}", "Created".green(), skill_path.display());
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

                // Create project-local .claude/settings.json with cwd set
                // This ensures the MCP server runs from the correct project directory
                let claude_dir = PathBuf::from(".claude");
                let claude_settings_path = claude_dir.join("settings.json");
                if !claude_settings_path.exists() || force_overwrite {
                    if let Err(e) = std::fs::create_dir_all(&claude_dir) {
                        eprintln!("{} Could not create .claude directory: {}", "•".yellow(), e);
                    } else {
                        // Get absolute path for cwd
                        let project_cwd = std::env::current_dir()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| ".".to_string());

                        let claude_settings = format!(
                            r#"{{
  "mcpServers": {{
    "chant": {{
      "type": "stdio",
      "command": "chant",
      "args": ["mcp"],
      "cwd": "{}"
    }}
  }}
}}
"#,
                            project_cwd
                        );
                        if let Err(e) = std::fs::write(&claude_settings_path, claude_settings) {
                            eprintln!(
                                "{} Could not create {}: {}",
                                "•".yellow(),
                                claude_settings_path.display(),
                                e
                            );
                        } else {
                            println!(
                                "{} {} (project-local MCP config)",
                                "Created".green(),
                                claude_settings_path.display()
                            );

                            // Add .claude/ to git exclude (contains machine-specific paths)
                            let exclude_output = std::process::Command::new("git")
                                .args(["rev-parse", "--git-common-dir"])
                                .output();
                            if let Ok(output) = exclude_output {
                                if output.status.success() {
                                    if let Ok(git_dir) = String::from_utf8(output.stdout)
                                        .map(|s| s.trim().to_string())
                                    {
                                        let exclude_path =
                                            PathBuf::from(&git_dir).join("info/exclude");
                                        if let Ok(mut exclude_content) =
                                            std::fs::read_to_string(&exclude_path)
                                                .or_else(|_| Ok::<_, std::io::Error>(String::new()))
                                        {
                                            if !exclude_content.contains(".claude/")
                                                && !exclude_content.contains(".claude")
                                            {
                                                // Create info directory if needed
                                                if let Some(parent) = exclude_path.parent() {
                                                    let _ = std::fs::create_dir_all(parent);
                                                }
                                                if !exclude_content.ends_with('\n')
                                                    && !exclude_content.is_empty()
                                                {
                                                    exclude_content.push('\n');
                                                }
                                                exclude_content.push_str(".claude/\n");
                                                let _ =
                                                    std::fs::write(&exclude_path, exclude_content);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Also create project-local .mcp.json as reference (legacy)
                let mcp_path = PathBuf::from(".mcp.json");
                if !mcp_path.exists() || force_overwrite {
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
                    }
                }

                if mcp_created {
                    println!(
                        "{} Restart Claude Code to activate MCP integration",
                        "ℹ".cyan()
                    );
                }

                // Also create global CLAUDE.md next to binary for system-wide access
                if *provider == templates::AgentProvider::Claude {
                    if let Err(e) = create_global_claude_md(will_have_mcp) {
                        // Non-critical error - log but don't fail
                        eprintln!("{} Could not create global CLAUDE.md: {}", "•".yellow(), e);
                    }
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
            "--force-overwrite".cyan()
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
    } else if let Some(Ok(result)) = merge_driver_result_opt {
        if result.git_config_set {
            println!(
                "{} Merge driver configured (auto-resolves spec file conflicts)",
                "ℹ".cyan()
            );
        }
    }

    // Configure kiro-cli-chat MCP if requested
    if final_setup_kirocli {
        println!(); // Add spacing
        if let Err(e) = configure_kirocli_mcp() {
            eprintln!("{} Failed to configure kiro-cli-chat MCP: {}", "✗".red(), e);
            eprintln!(
                "{} You can manually configure it later with: chant init --provider kirocli",
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

/// Public entry point for the init command
#[allow(clippy::too_many_arguments)]
pub fn run(
    subcommand: Option<&str>,
    name: Option<String>,
    silent: bool,
    force_overwrite: bool,
    minimal: bool,
    agents: Vec<String>,
    provider: Option<String>,
    model: Option<String>,
    merge_driver: bool,
) -> Result<()> {
    cmd_init(
        subcommand,
        name,
        silent,
        force_overwrite,
        minimal,
        agents,
        provider,
        model,
        merge_driver,
    )
}
