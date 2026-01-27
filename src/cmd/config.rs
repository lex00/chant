//! Config command for validating chant configuration

use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use chant::config::Config;
use chant::paths::PROMPTS_DIR;

/// Validate config semantically and report issues
pub fn cmd_config_validate() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "Validating chant configuration...".bold());
    println!();

    let mut errors = 0;
    let mut warnings = 0;

    // Validate parallel agents
    errors += validate_agents(&config);

    // Validate prompts
    errors += validate_prompts(&config);

    // Show parallel config (informational only)
    show_parallel_config(&config);

    // Check recommended fields
    warnings += check_recommended_fields(&config);

    // Summary
    println!();
    if errors == 0 && warnings == 0 {
        println!("{} Configuration is valid", "✓".green());
    } else if errors == 0 {
        println!(
            "{} Configuration valid with {} warning(s)",
            "✓".green(),
            warnings
        );
    } else {
        println!(
            "{} Found {} error(s) and {} warning(s)",
            "✗".red(),
            errors,
            warnings
        );
        std::process::exit(1);
    }

    Ok(())
}

/// Validate parallel agent commands exist in PATH
fn validate_agents(config: &Config) -> usize {
    let agents = &config.parallel.agents;
    if agents.is_empty() {
        return 0;
    }

    println!("{}", "Checking parallel agents...".dimmed());

    let mut errors = 0;

    for agent in agents {
        let found = command_exists(&agent.command);
        if found {
            println!(
                "  {} {} ({}) - found in PATH",
                "✓".green(),
                agent.name,
                agent.command.dimmed()
            );
        } else {
            println!(
                "  {} {} ({}) - not found in PATH",
                "✗".red(),
                agent.name,
                agent.command
            );
            errors += 1;
        }
    }

    errors
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Validate referenced prompts exist
fn validate_prompts(config: &Config) -> usize {
    println!("{}", "Checking prompts...".dimmed());

    let prompts_dir = Path::new(PROMPTS_DIR);
    let mut errors = 0;

    // Check default prompt
    let default_prompt = &config.defaults.prompt;
    let default_path = prompts_dir.join(format!("{}.md", default_prompt));
    if default_path.exists() {
        println!(
            "  {} {}.md (defaults.prompt)",
            "✓".green(),
            default_prompt
        );
    } else {
        println!(
            "  {} {}.md not found (defaults.prompt)",
            "✗".red(),
            default_prompt
        );
        errors += 1;
    }

    // Check cleanup prompt
    let cleanup_prompt = &config.parallel.cleanup.prompt;
    let cleanup_path = prompts_dir.join(format!("{}.md", cleanup_prompt));
    if cleanup_path.exists() {
        println!(
            "  {} {}.md (parallel.cleanup.prompt)",
            "✓".green(),
            cleanup_prompt
        );
    } else if config.parallel.cleanup.enabled {
        println!(
            "  {} {}.md not found (parallel.cleanup.prompt)",
            "✗".red(),
            cleanup_prompt
        );
        errors += 1;
    }

    errors
}

/// Display parallel configuration (informational, no warnings)
fn show_parallel_config(config: &Config) {
    let agents = &config.parallel.agents;
    if agents.is_empty() {
        return;
    }

    println!("{}", "Parallel config...".dimmed());

    let total_capacity = config.parallel.total_capacity();

    println!(
        "  {} {} agent(s), total capacity: {}",
        "ℹ".blue(),
        agents.len(),
        total_capacity
    );
}

/// Check for missing recommended fields
fn check_recommended_fields(config: &Config) -> usize {
    println!("{}", "Checking recommended fields...".dimmed());

    let mut warnings = 0;

    if let Some(model) = &config.defaults.model {
        println!("  {} defaults.model: {}", "✓".green(), model);
    } else {
        println!(
            "  {} No defaults.model set - will use haiku",
            "⚠".yellow()
        );
        warnings += 1;
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_exists_finds_common_commands() {
        // 'ls' should exist on all Unix systems
        assert!(command_exists("ls"));
    }

    #[test]
    fn test_command_exists_returns_false_for_nonexistent() {
        assert!(!command_exists("definitely-not-a-real-command-xyz"));
    }
}
