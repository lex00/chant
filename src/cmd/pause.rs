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

    // Check if there's a PID file
    let pid = pid::read_pid_file(&spec_id)?;

    let mut process_stopped = false;
    if let Some(pid) = pid {
        if pid::is_process_running(pid) {
            // Process is running, stop it
            println!("  {} Process {} is running", "•".cyan(), pid);

            if force {
                println!("  {} Sending SIGTERM to process {}", "•".cyan(), pid);
                pid::stop_process(pid)?;
                pid::remove_pid_file(&spec_id)?;
                process_stopped = true;
                println!("  {} Process stopped", "✓".green());
            } else {
                println!("{} Use --force to stop the process", "⚠".yellow());
                anyhow::bail!(
                    "Spec {} has a running process (PID: {}). Use --force to stop it.",
                    spec_id,
                    pid
                );
            }
        } else {
            // Process not running, clean up PID file
            println!(
                "  {} Process {} is not running (cleaning up PID file)",
                "•".cyan(),
                pid
            );
            pid::remove_pid_file(&spec_id)?;
        }
    } else {
        println!(
            "{} No work process found for spec {}",
            "⚠".yellow(),
            spec_id
        );
    }

    // Update spec status to paused if it was in_progress
    if spec.frontmatter.status == SpecStatus::InProgress {
        spec.frontmatter.status = SpecStatus::Paused;
        spec.save(&spec_path)?;
        println!("  {} Status set to: paused", "•".cyan());
    }

    if process_stopped {
        println!("{} Paused work for spec {}", "✓".green(), spec_id);
    }

    Ok(())
}
