//! Silent mode management command

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Enable or disable silent mode for project or globally
pub fn cmd_silent(global: bool, off: bool, status: bool) -> Result<()> {
    if status {
        return show_status(global);
    }

    if off {
        disable_silent_mode(global)
    } else {
        enable_silent_mode(global)
    }
}

fn show_status(global: bool) -> Result<()> {
    if global {
        // Check global config
        if let Some(config_path) = chant::config::global_config_path() {
            if config_path.exists() {
                let silent = check_silent_in_config(&config_path)?;
                if silent {
                    println!("{} Globally enabled", "✓".green());
                } else {
                    println!("{} Globally disabled", "○".dimmed());
                }
            } else {
                println!("{} Globally disabled (no global config)", "○".dimmed());
            }
        } else {
            println!("{} Cannot determine global config path", "✗".red());
        }
    } else {
        // Check project config
        let project_config_path = Path::new(".chant/config.md");
        if !project_config_path.exists() {
            anyhow::bail!("Not in a chant project (no .chant/config.md found)");
        }

        let silent = check_silent_in_config(project_config_path)?;

        // Also check if .chant/ is in git/info/exclude
        let in_git_exclude = is_in_git_exclude()?;

        if silent {
            println!("{} Enabled for this project", "✓".green());
            if in_git_exclude {
                println!("  {} .chant/ is excluded from git", "✓".dimmed());
            } else {
                println!(
                    "  {} .chant/ is NOT excluded from git (inconsistent)",
                    "⚠".yellow()
                );
            }
        } else {
            println!("{} Disabled for this project", "○".dimmed());
            if in_git_exclude {
                println!(
                    "  {} .chant/ is excluded from git (inconsistent)",
                    "⚠".yellow()
                );
            }
        }
    }

    Ok(())
}

fn enable_silent_mode(global: bool) -> Result<()> {
    if global {
        enable_global_silent()?;
        println!("{} Silent mode enabled globally", "✓".green());
        println!("  All new and existing projects will default to silent mode");
    } else {
        enable_project_silent()?;
        println!("{} Silent mode enabled for this project", "✓".green());
        println!("  {} .chant/ added to .git/info/exclude", "✓".dimmed());
        println!("  {} Specs will not be committed", "ℹ".blue());
    }

    Ok(())
}

fn disable_silent_mode(global: bool) -> Result<()> {
    if global {
        disable_global_silent()?;
        println!("{} Silent mode disabled globally", "✓".green());
    } else {
        disable_project_silent()?;
        println!("{} Silent mode disabled for this project", "✓".green());
        println!("  {} .chant/ removed from .git/info/exclude", "✓".dimmed());
        println!("  {} Specs can now be committed", "ℹ".blue());
    }

    Ok(())
}

fn enable_global_silent() -> Result<()> {
    let config_path =
        chant::config::global_config_path().context("Cannot determine global config path")?;

    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    update_config_silent(&config_path, true)?;

    Ok(())
}

fn disable_global_silent() -> Result<()> {
    let config_path =
        chant::config::global_config_path().context("Cannot determine global config path")?;

    if !config_path.exists() {
        anyhow::bail!("No global config found");
    }

    update_config_silent(&config_path, false)?;

    Ok(())
}

fn enable_project_silent() -> Result<()> {
    let config_path = Path::new(".chant/config.md");
    if !config_path.exists() {
        anyhow::bail!("Not in a chant project (no .chant/config.md found)");
    }

    // Update config
    update_config_silent(config_path, true)?;

    // Add to .git/info/exclude
    add_to_git_exclude()?;

    // Remove from .gitignore if present
    remove_from_gitignore()?;

    Ok(())
}

fn disable_project_silent() -> Result<()> {
    let config_path = Path::new(".chant/config.md");
    if !config_path.exists() {
        anyhow::bail!("Not in a chant project (no .chant/config.md found)");
    }

    // Update config
    update_config_silent(config_path, false)?;

    // Remove from .git/info/exclude
    remove_from_git_exclude()?;

    Ok(())
}

fn update_config_silent(config_path: &Path, enable: bool) -> Result<()> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

    let (frontmatter, body) = chant::spec::split_frontmatter(&content);
    let frontmatter = frontmatter.context("Failed to extract frontmatter from config")?;

    // Parse as YAML value to manipulate
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&frontmatter).context("Failed to parse config frontmatter")?;

    // Update silent field
    if let Some(mapping) = yaml.as_mapping_mut() {
        let silent_key = serde_yaml::Value::String("silent".to_string());
        mapping.insert(silent_key, serde_yaml::Value::Bool(enable));
    }

    // Serialize back to YAML
    let updated_frontmatter =
        serde_yaml::to_string(&yaml).context("Failed to serialize updated config")?;

    // Reconstruct the file
    let updated_content = format!("---\n{}---\n{}", updated_frontmatter, body);

    fs::write(config_path, updated_content)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

    Ok(())
}

fn check_silent_in_config(config_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

    let (frontmatter, _body) = chant::spec::split_frontmatter(&content);
    let frontmatter = frontmatter.context("Failed to extract frontmatter from config")?;

    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&frontmatter).context("Failed to parse config frontmatter")?;

    Ok(yaml
        .as_mapping()
        .and_then(|m| m.get(serde_yaml::Value::String("silent".to_string())))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

fn add_to_git_exclude() -> Result<()> {
    let exclude_path = get_git_exclude_path()?;

    // Create info directory if it doesn't exist
    if let Some(parent) = exclude_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing exclude file
    let mut exclude_content = fs::read_to_string(&exclude_path).unwrap_or_default();

    // Add .chant/ if not already present
    if !exclude_content.contains(".chant/") && !exclude_content.contains(".chant") {
        if !exclude_content.ends_with('\n') && !exclude_content.is_empty() {
            exclude_content.push('\n');
        }
        exclude_content.push_str(".chant/\n");
        fs::write(&exclude_path, exclude_content)?;
    }

    Ok(())
}

fn remove_from_git_exclude() -> Result<()> {
    let exclude_path = get_git_exclude_path()?;

    if !exclude_path.exists() {
        return Ok(());
    }

    let exclude_content = fs::read_to_string(&exclude_path)?;
    let updated_content: String = exclude_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed != ".chant/" && trimmed != ".chant"
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Add trailing newline if content is not empty
    let final_content = if updated_content.is_empty() {
        updated_content
    } else {
        format!("{}\n", updated_content)
    };

    fs::write(&exclude_path, final_content)?;

    Ok(())
}

fn remove_from_gitignore() -> Result<()> {
    let gitignore_path = Path::new(".gitignore");

    if !gitignore_path.exists() {
        return Ok(());
    }

    let gitignore_content = fs::read_to_string(gitignore_path)?;
    let updated_content: String = gitignore_content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed != ".chant/" && trimmed != ".chant"
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Add trailing newline if content is not empty
    let final_content = if updated_content.is_empty() {
        updated_content
    } else {
        format!("{}\n", updated_content)
    };

    fs::write(gitignore_path, final_content)?;

    Ok(())
}

fn get_git_exclude_path() -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .context("Failed to run git rev-parse")?;

    if !output.status.success() {
        anyhow::bail!("Not in a git repository");
    }

    let git_dir = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(PathBuf::from(&git_dir).join("info/exclude"))
}

fn is_in_git_exclude() -> Result<bool> {
    let exclude_path = get_git_exclude_path()?;

    if !exclude_path.exists() {
        return Ok(false);
    }

    let exclude_content = fs::read_to_string(&exclude_path)?;
    Ok(exclude_content.contains(".chant/") || exclude_content.contains(".chant"))
}
