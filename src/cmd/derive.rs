//! Derivation command for manual field derivation on specs.
//!
//! Allows users to:
//! - Re-derive fields after changing enterprise config
//! - Test derivation patterns before committing to config
//! - Derive fields for existing specs created before enterprise config
//! - Preview derived fields with --dry-run mode

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use chant::config::Config;
use chant::derivation::{DerivationContext, DerivationEngine};
use chant::git;
use chant::spec;

/// Derive fields for one or all specs
pub fn cmd_derive(spec_id: Option<String>, all: bool, dry_run: bool) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;
    let config = Config::load()?;

    // Fast path: no-op if no enterprise config
    if config.enterprise.derived.is_empty() {
        println!("No enterprise derivation configured");
        return Ok(());
    }

    // Determine which specs to process
    let specs = if all {
        spec::load_all_specs(&specs_dir)?
    } else if let Some(id) = spec_id {
        vec![spec::resolve_spec(&specs_dir, &id)?]
    } else {
        anyhow::bail!("Specify --all or provide SPEC_ID");
    };

    // Process each spec
    for spec in specs {
        let spec_path = specs_dir.join(format!("{}.md", spec.id));
        let context = build_derivation_context(&spec.id, &specs_dir)?;
        let engine = DerivationEngine::new(config.enterprise.clone());
        let derived = engine.derive_fields(&context);

        if derived.is_empty() {
            println!("{}: no fields derived", spec.id.cyan());
        } else if dry_run {
            print_derived_fields(&spec.id, &derived);
        } else {
            // Update the spec with derived fields
            let mut updated_spec = spec.clone();
            updated_spec.add_derived_fields(derived.clone());
            updated_spec
                .save(&spec_path)
                .context("Failed to save spec with derived fields")?;

            println!(
                "{}: updated with {} derived field(s)",
                spec.id.green(),
                derived.len()
            );
            for (key, value) in &derived {
                println!("  {} = {}", key.cyan(), value);
            }
        }
    }

    Ok(())
}

/// Build a DerivationContext with all available sources for derivation.
/// Returns a context with branch name, spec path, environment variables, and git user info.
fn build_derivation_context(spec_id: &str, specs_dir: &Path) -> Result<DerivationContext> {
    let mut context = DerivationContext::new();

    // Get current branch
    if let Ok(branch) = git::get_current_branch() {
        context.branch_name = Some(branch);
    }

    // Get spec path
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    context.spec_path = Some(spec_path);

    // Capture environment variables
    context.env_vars = std::env::vars().collect();

    // Get git user.name
    if let Ok(output) = Command::new("git").args(["config", "user.name"]).output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                context.git_user_name = Some(name);
            }
        }
    }

    // Get git user.email
    if let Ok(output) = Command::new("git").args(["config", "user.email"]).output() {
        if output.status.success() {
            let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !email.is_empty() {
                context.git_user_email = Some(email);
            }
        }
    }

    Ok(context)
}

/// Print derived fields to stdout in a human-readable format
fn print_derived_fields(spec_id: &str, fields: &std::collections::HashMap<String, String>) {
    println!("{}: would derive {} field(s)", spec_id.cyan(), fields.len());
    for (key, value) in fields {
        println!("  {} = {}", key.cyan(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[serial_test::serial]
    fn test_derive_no_enterprise_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // Initialize chant
            crate::cmd_init(None, Some("test".to_string()), false, false, false, vec![])
                .expect("Failed to init chant");

            // Run derive with no enterprise config
            let result = cmd_derive(None, true, false);
            assert!(result.is_ok());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_derive_single_spec_dry_run() {
        let temp_dir = tempfile::tempdir().unwrap();
        let orig_dir = std::env::current_dir().unwrap();

        if std::env::set_current_dir(&temp_dir).is_ok() {
            // Initialize chant
            crate::cmd_init(None, Some("test".to_string()), false, false, false, vec![])
                .expect("Failed to init chant");

            // Create a spec
            let specs_dir = temp_dir.path().join(".chant/specs");
            let spec_content = r#"---
type: code
status: pending
---

# Test Spec

Test spec for derivation

## Acceptance Criteria

- [ ] Test completed
"#;
            let spec_path = specs_dir.join("2026-01-27-test-abc.md");
            fs::write(&spec_path, spec_content).expect("Failed to write spec");

            // Add enterprise config with derivation
            let config_path = temp_dir.path().join(".chant/config.md");
            let config_content = r#"---
project:
  name: test-project

defaults:
  prompt: standard

enterprise:
  derived:
    team:
      from: env
      pattern: TEAM_NAME
---

# Chant Configuration
"#;
            fs::write(&config_path, config_content).expect("Failed to write config");

            // Run derive with dry-run (should not fail even without env var set)
            let result = cmd_derive(Some("2026-01-27-test-abc".to_string()), false, true);
            assert!(result.is_ok());

            let _ = std::env::set_current_dir(orig_dir);
        }
    }
}
