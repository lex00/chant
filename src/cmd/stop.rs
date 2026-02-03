//! Stop command for terminating running work processes

use anyhow::Result;
use colored::Colorize;

use chant::pid;
use chant::spec;

/// Stop a running work process for a spec
pub fn cmd_stop(id: &str, force: bool) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Resolve the spec ID
    let spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_id = &spec.id;

    println!("{} Stopping work for spec {}", "→".cyan(), spec_id.cyan());

    // Check if there's a PID file
    let pid = pid::read_pid_file(spec_id)?;

    if let Some(pid) = pid {
        if pid::is_process_running(pid) {
            // Process is running, stop it
            println!("  {} Process {} is running", "•".cyan(), pid);

            if force {
                println!("  {} Sending SIGTERM to process {}", "•".cyan(), pid);
                pid::stop_process(pid)?;
                pid::remove_pid_file(spec_id)?;
                println!("{} Stopped work for spec {}", "✓".green(), spec_id);
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
            pid::remove_pid_file(spec_id)?;
            println!(
                "{} Cleaned up stale PID file for spec {}",
                "✓".green(),
                spec_id
            );
        }
    } else {
        println!(
            "{} No work process found for spec {}",
            "⚠".yellow(),
            spec_id
        );
    }

    Ok(())
}
