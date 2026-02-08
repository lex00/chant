//! Pause command for pausing running work processes

use anyhow::Result;
use colored::Colorize;

use chant::pid;
use chant::spec::{self, SpecStatus};

/// Pause a running work process for a spec
pub fn cmd_pause(id: &str, force: bool) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec ID
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    println!("{} Pausing work for spec {}", "→".cyan(), spec_id.cyan());

    // Check if there's a PID file for force flag handling
    let pid = pid::read_pid_file(&spec_id)?;

    if let Some(pid_value) = pid {
        if pid::is_process_running(pid_value) {
            println!("  {} Process {} is running", "•".cyan(), pid_value);

            if !force {
                println!("{} Use --force to stop the process", "⚠".yellow());
                anyhow::bail!(
                    "Spec {} has a running process (PID: {}). Use --force to stop it.",
                    spec_id,
                    pid_value
                );
            }

            println!("  {} Sending SIGTERM to process {}", "•".cyan(), pid_value);
        } else {
            println!(
                "  {} Process {} is not running (cleaning up PID file)",
                "•".cyan(),
                pid_value
            );
        }
    } else {
        println!(
            "{} No work process found for spec {}",
            "⚠".yellow(),
            spec_id
        );
    }

    // Use operations layer
    let options = chant::operations::PauseOptions { force };
    let process_stopped = chant::operations::pause_spec(&mut spec, &spec_path, options)?;

    if spec.frontmatter.status == SpecStatus::Paused {
        println!("  {} Status set to: paused", "•".cyan());
    }

    if process_stopped {
        println!("  {} Process stopped", "✓".green());
        println!("{} Paused work for spec {}", "✓".green(), spec_id);
    }

    Ok(())
}
