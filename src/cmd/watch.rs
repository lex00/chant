//! Watch command for monitoring and managing spec lifecycle
//!
//! Continuously monitors in-progress specs and automatically handles
//! finalization, merging, and failure recovery. Watch mode is monitor-only:
//! it does not spawn agents or create specs, only orchestrates lifecycle operations.

use anyhow::{Context, Result};
use chrono::Local;
use colored::Colorize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chant::config::Config;
use chant::spec::{self, is_completed, is_failed, SpecStatus};
use chant::worktree::status::{read_status, AgentStatus, AgentStatusState};

use crate::cmd;

/// Logger for watch command with structured output and file persistence
pub struct WatchLogger {
    log_file: Option<std::fs::File>,
    log_path: PathBuf,
    #[allow(dead_code)]
    stdout_only: bool,
}

impl WatchLogger {
    /// Initialize the watch logger with log file at `.chant/logs/watch.log`
    pub fn init() -> Result<Self> {
        let log_dir = PathBuf::from(".chant/logs");
        let log_path = log_dir.join("watch.log");

        // Create log directory if it doesn't exist
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir).with_context(|| {
                format!("Failed to create log directory: {}", log_dir.display())
            })?;
        }

        // Try to open log file in append mode
        let (log_file, stdout_only) =
            match OpenOptions::new().create(true).append(true).open(&log_path) {
                Ok(file) => (Some(file), false),
                Err(e) => {
                    // Log file unwritable - fall back to stdout-only mode
                    eprintln!(
                        "Warning: Could not open log file at {}: {}",
                        log_path.display(),
                        e
                    );
                    eprintln!("Continuing with stdout-only logging");
                    (None, true)
                }
            };

        Ok(WatchLogger {
            log_file,
            log_path,
            stdout_only,
        })
    }

    /// Log an event with timestamp to both stdout and file
    pub fn log_event(&mut self, message: &str) -> Result<()> {
        let timestamp = Local::now().format("[%H:%M:%S]");
        let formatted = format!("{} {}", timestamp, message);

        // Write to stdout
        println!("{}", formatted);

        // Write to file if available
        if let Some(ref mut file) = self.log_file {
            writeln!(file, "{}", formatted).with_context(|| {
                format!("Failed to write to log file: {}", self.log_path.display())
            })?;

            // Flush to ensure visibility during long runs
            file.flush().with_context(|| {
                format!("Failed to flush log file: {}", self.log_path.display())
            })?;
        }

        Ok(())
    }

    /// Get the path to the log file
    #[allow(dead_code)]
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Check if logger is in stdout-only mode (file logging failed)
    #[allow(dead_code)]
    pub fn is_stdout_only(&self) -> bool {
        self.stdout_only
    }
}

/// Information about an active worktree
#[derive(Debug, Clone)]
struct WorktreeInfo {
    path: PathBuf,
    spec_id: String,
}

/// Find all active worktrees with branches matching the given prefix
fn find_active_worktrees(branch_prefix: &str) -> Result<Vec<WorktreeInfo>> {
    // Get worktree list from git
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .context("Failed to run git worktree list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree list failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            // Save previous entry if it's a chant worktree
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                if branch.starts_with(branch_prefix) {
                    if let Some(spec_id) = branch.strip_prefix(branch_prefix) {
                        worktrees.push(WorktreeInfo {
                            path,
                            spec_id: spec_id.to_string(),
                        });
                    }
                }
            }
            // Start new entry
            let path = line.strip_prefix("worktree ").unwrap_or("");
            current_path = Some(PathBuf::from(path));
            current_branch = None;
        } else if line.starts_with("branch ") {
            let branch = line.strip_prefix("branch ").unwrap_or("");
            // Strip refs/heads/ prefix if present
            let branch = branch.strip_prefix("refs/heads/").unwrap_or(branch);
            current_branch = Some(branch.to_string());
        }
    }

    // Don't forget the last entry
    if let (Some(path), Some(branch)) = (current_path, current_branch) {
        if branch.starts_with(branch_prefix) {
            if let Some(spec_id) = branch.strip_prefix(branch_prefix) {
                worktrees.push(WorktreeInfo {
                    path,
                    spec_id: spec_id.to_string(),
                });
            }
        }
    }

    Ok(worktrees)
}

/// Check worktree status by reading .chant-status.json
fn check_worktree_status(path: &Path) -> Result<AgentStatus> {
    let status_file = path.join(".chant-status.json");
    read_status(&status_file)
}

/// Read PID from watch PID file
fn read_watch_pid() -> Result<u32> {
    let pid_path = PathBuf::from(".chant/watch.pid");
    let content = fs::read_to_string(&pid_path)
        .with_context(|| format!("Failed to read PID file: {}", pid_path.display()))?;
    let pid: u32 = content
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in file: {}", content))?;
    Ok(pid)
}

/// Write PID to watch PID file
fn write_watch_pid() -> Result<()> {
    let pid_path = PathBuf::from(".chant/watch.pid");
    let pid = std::process::id();
    fs::write(&pid_path, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", pid_path.display()))?;
    Ok(())
}

/// Remove watch PID file
fn remove_watch_pid() -> Result<()> {
    let pid_path = PathBuf::from(".chant/watch.pid");
    if pid_path.exists() {
        fs::remove_file(&pid_path)
            .with_context(|| format!("Failed to remove PID file: {}", pid_path.display()))?;
    }
    Ok(())
}

/// Check if a process is alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        // Signal 0 doesn't send a signal but checks if process exists
        kill(Pid::from_raw(pid as i32), None).is_ok()
    }

    #[cfg(windows)]
    {
        // Use tasklist command as a simple cross-platform approach
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Some(stdout.contains(&pid.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or(false)
    }
}

/// Check if a process is chant watch by examining its command line
fn is_chant_watch_process(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Read /proc/<pid>/cmdline on Linux or use ps on macOS
        #[cfg(target_os = "linux")]
        {
            let cmdline_path = format!("/proc/{}/cmdline", pid);
            if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
                // cmdline is null-separated, check for "chant" and "watch"
                return cmdline.contains("chant") && cmdline.contains("watch");
            }
            false
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Use ps command on macOS and other Unix systems
            let output = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "command="])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let cmdline = String::from_utf8_lossy(&output.stdout);
                    return cmdline.contains("chant") && cmdline.contains("watch");
                }
            }
            false
        }
    }

    #[cfg(windows)]
    {
        // Use WMIC or PowerShell to get command line
        let output = Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("ProcessId={}", pid),
                "get",
                "CommandLine",
                "/format:list",
            ])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let cmdline = String::from_utf8_lossy(&output.stdout);
                return cmdline.contains("chant") && cmdline.contains("watch");
            }
        }
        false
    }
}

/// Check if watch is currently running
///
/// This checks if:
/// 1. PID file exists
/// 2. Process with that PID is alive
/// 3. Process is actually chant watch (not a reused PID)
///
/// If the PID file exists but the process is dead or not chant watch,
/// the stale PID file is automatically cleaned up.
pub fn is_watch_running() -> bool {
    match read_watch_pid() {
        Ok(pid) => {
            // Verify process exists AND is chant watch
            if is_process_alive(pid) && is_chant_watch_process(pid) {
                true
            } else {
                // Stale PID - clean up and report not running
                let _ = remove_watch_pid();
                false
            }
        }
        Err(_) => false, // No file = not running
    }
}

/// Run startup recovery to handle crashed agents and stale worktrees
///
/// This function detects and recovers from:
/// - Status "done" but branch not merged: Queue for merge
/// - Status "working" with timestamp >1 hour old: Mark spec failed, cleanup
/// - Orphaned worktrees (no status file, >1 day old): Cleanup
///
/// Returns the number of recovery actions taken
fn run_startup_recovery(logger: &mut WatchLogger, dry_run: bool, branch_prefix: &str) -> Result<usize> {
    let mut actions = 0;
    let now = chrono::Utc::now();

    // Find all active worktrees
    let active_worktrees = find_active_worktrees(branch_prefix)?;

    for worktree in &active_worktrees {
        let spec_id = &worktree.spec_id;
        let status_file = worktree.path.join(".chant-status.json");

        // Check if status file exists
        if !status_file.exists() {
            // Orphaned worktree - check age
            let worktree_metadata = fs::metadata(&worktree.path)?;
            let modified_time = worktree_metadata.modified()?;
            let age = now
                .signed_duration_since(chrono::DateTime::<chrono::Utc>::from(modified_time))
                .num_hours();

            if age > 24 {
                logger.log_event(&format!(
                    "Found orphaned worktree for spec {} (age: {}h)",
                    spec_id.cyan(),
                    age
                ))?;

                if dry_run {
                    logger.log_event(&format!(
                        "  {} would cleanup orphaned worktree",
                        "→".dimmed()
                    ))?;
                } else {
                    // Cleanup orphaned worktree
                    match chant::worktree::remove_worktree(&worktree.path) {
                        Ok(()) => {
                            logger.log_event(&format!(
                                "  {} cleaned up orphaned worktree",
                                "✓".green()
                            ))?;
                            actions += 1;
                        }
                        Err(e) => {
                            logger.log_event(&format!(
                                "  {} failed to cleanup: {}",
                                "✗".red(),
                                e
                            ))?;
                        }
                    }
                }
            }
            continue;
        }

        // Read status file
        match read_status(&status_file) {
            Ok(status) => {
                match status.status {
                    AgentStatusState::Done => {
                        // Status is done - check if branch has been merged
                        logger.log_event(&format!(
                            "Found completed worktree for spec {} that needs merge",
                            spec_id.cyan()
                        ))?;

                        if dry_run {
                            logger.log_event(&format!(
                                "  {} would finalize and merge spec",
                                "→".dimmed()
                            ))?;
                        } else {
                            // Queue for merge via handle_completed
                            match crate::cmd::lifecycle::handle_completed(spec_id) {
                                Ok(()) => {
                                    logger.log_event(&format!(
                                        "  {} finalized and merged",
                                        "✓".green()
                                    ))?;
                                    actions += 1;
                                }
                                Err(e) => {
                                    logger.log_event(&format!(
                                        "  {} failed to finalize: {}",
                                        "✗".red(),
                                        e
                                    ))?;
                                }
                            }
                        }
                    }
                    AgentStatusState::Working => {
                        // Check if status is stale (>1 hour old)
                        match chrono::DateTime::parse_from_rfc3339(&status.updated_at) {
                            Ok(updated_at) => {
                                let age_hours = now
                                    .signed_duration_since(updated_at.with_timezone(&chrono::Utc))
                                    .num_hours();

                                if age_hours > 1 {
                                    logger.log_event(&format!(
                                        "Found stale working worktree for spec {} (age: {}h)",
                                        spec_id.cyan(),
                                        age_hours
                                    ))?;

                                    if dry_run {
                                        logger.log_event(&format!(
                                            "  {} would mark spec failed and cleanup",
                                            "→".dimmed()
                                        ))?;
                                    } else {
                                        // Mark spec as failed
                                        let specs_dir = PathBuf::from(".chant/specs");
                                        if let Ok(mut spec) =
                                            spec::resolve_spec(&specs_dir, spec_id)
                                        {
                                            spec.frontmatter.status = SpecStatus::Failed;
                                            let spec_path =
                                                specs_dir.join(format!("{}.md", spec_id));
                                            if let Err(e) = spec.save(&spec_path) {
                                                logger.log_event(&format!(
                                                    "  {} failed to mark spec failed: {}",
                                                    "✗".red(),
                                                    e
                                                ))?;
                                            } else {
                                                logger.log_event(&format!(
                                                    "  {} marked spec as failed",
                                                    "✓".green()
                                                ))?;

                                                // Cleanup worktree
                                                match chant::worktree::remove_worktree(
                                                    &worktree.path,
                                                ) {
                                                    Ok(()) => {
                                                        logger.log_event(&format!(
                                                            "  {} cleaned up worktree",
                                                            "✓".green()
                                                        ))?;
                                                        actions += 1;
                                                    }
                                                    Err(e) => {
                                                        logger.log_event(&format!(
                                                            "  {} failed to cleanup: {}",
                                                            "✗".red(),
                                                            e
                                                        ))?;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                logger.log_event(&format!(
                                    "{} Failed to parse timestamp for spec {}: {}",
                                    "⚠".yellow(),
                                    spec_id,
                                    e
                                ))?;
                            }
                        }
                    }
                    AgentStatusState::Failed => {
                        // Already marked as failed, just log
                        logger.log_event(&format!(
                            "Found failed worktree for spec {} (no action needed)",
                            spec_id.cyan()
                        ))?;
                    }
                }
            }
            Err(e) => {
                logger.log_event(&format!(
                    "{} Failed to read status for worktree {}: {}",
                    "⚠".yellow(),
                    worktree.path.display(),
                    e
                ))?;
            }
        }
    }

    Ok(actions)
}

/// Main entry point for watch command
pub fn run_watch(once: bool, dry_run: bool, poll_interval: Option<u64>) -> Result<()> {
    let _specs_dir = cmd::ensure_initialized()?;
    let config = Config::load()?;

    // Use command-line override or config value
    let poll_interval_ms = poll_interval.unwrap_or(config.watch.poll_interval_ms);

    // Warn if poll interval is very short
    if poll_interval_ms < 1000 {
        eprintln!(
            "{} Poll interval {}ms is very short (< 1s)",
            "⚠".yellow(),
            poll_interval_ms
        );
    }

    // Write PID file on startup
    write_watch_pid().context("Failed to write watch PID file")?;

    // Initialize logger
    let mut logger = WatchLogger::init()?;
    logger.log_event(&format!(
        "Watch started (poll_interval={}ms, once={}, dry_run={})",
        poll_interval_ms, once, dry_run
    ))?;

    // Run startup recovery
    logger.log_event("Running startup recovery...")?;
    match run_startup_recovery(&mut logger, dry_run, &config.defaults.branch_prefix) {
        Ok(actions) => {
            if actions > 0 {
                logger.log_event(&format!(
                    "{} Recovered from {} stale worktree(s)",
                    "✓".green(),
                    actions
                ))?;
            } else {
                logger.log_event("No recovery actions needed")?;
            }
        }
        Err(e) => {
            logger.log_event(&format!("{} Recovery failed: {}", "⚠".yellow(), e))?;
        }
    }

    // Set up signal handler for graceful shutdown
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .context("Failed to set signal handler")?;

    // Track last activity time for idle timeout
    let mut last_activity = Instant::now();
    let idle_timeout = Duration::from_secs(config.watch.idle_timeout_minutes * 60);

    // Main event loop
    loop {
        // Check for shutdown signal
        if shutdown.load(Ordering::SeqCst) {
            logger.log_event("Shutdown signal received, exiting gracefully")?;
            remove_watch_pid()?;
            break;
        }

        // Query in-progress specs
        let specs = spec::load_all_specs(&PathBuf::from(".chant/specs"))?;
        let in_progress_specs: Vec<_> = specs
            .iter()
            .filter(|s| matches!(s.frontmatter.status, SpecStatus::InProgress))
            .collect();

        // Discover active worktrees
        let active_worktrees = match find_active_worktrees(&config.defaults.branch_prefix) {
            Ok(worktrees) => worktrees,
            Err(e) => {
                logger.log_event(&format!(
                    "{} Failed to discover worktrees: {}",
                    "⚠".yellow(),
                    e
                ))?;
                Vec::new()
            }
        };

        if in_progress_specs.is_empty() && active_worktrees.is_empty() {
            logger.log_event("No in-progress specs or active worktrees, waiting...")?;

            // Check idle timeout
            if last_activity.elapsed() >= idle_timeout && !once {
                logger.log_event(&format!(
                    "Idle for {} minutes, exiting",
                    config.watch.idle_timeout_minutes
                ))?;
                remove_watch_pid()?;
                break;
            }
        } else {
            // Reset activity timer when there's work
            last_activity = Instant::now();

            if !in_progress_specs.is_empty() {
                logger.log_event(&format!(
                    "Checking {} in-progress spec(s)",
                    in_progress_specs.len()
                ))?;

                // Check each spec for completion or failure
                for spec in &in_progress_specs {
                    let spec_id = &spec.id;

                    // Check if completed
                    if is_completed(spec_id)? {
                        logger.log_event(&format!("Spec {} is completed", spec_id.cyan()))?;

                        if dry_run {
                            logger.log_event(&format!(
                                "  {} would finalize {}",
                                "→".dimmed(),
                                spec_id
                            ))?;
                        } else {
                            // Handle completion (finalize + merge)
                            match crate::cmd::lifecycle::handle_completed(spec_id) {
                                Ok(()) => {
                                    logger.log_event(&format!(
                                        "  {} finalized and merged",
                                        "✓".green()
                                    ))?;
                                }
                                Err(e) => {
                                    logger.log_event(&format!(
                                        "  {} failed to finalize: {}",
                                        "✗".red(),
                                        e
                                    ))?;
                                }
                            }
                        }
                        continue;
                    }

                    // Check if failed
                    if is_failed(spec_id)? {
                        logger.log_event(&format!("Spec {} has failed", spec_id.cyan()))?;

                        if dry_run {
                            logger.log_event(&format!(
                                "  {} would handle failure for {}",
                                "→".dimmed(),
                                spec_id
                            ))?;
                        } else {
                            // Handle failure (retry or permanent failure)
                            match crate::cmd::lifecycle::handle_failed(
                                spec_id,
                                &config.watch.failure,
                            ) {
                                Ok(()) => {
                                    logger
                                        .log_event(&format!("  {} failure handled", "✓".green()))?;
                                }
                                Err(e) => {
                                    logger.log_event(&format!(
                                        "  {} failed to handle failure: {}",
                                        "✗".red(),
                                        e
                                    ))?;
                                }
                            }
                        }
                    }
                }
            }

            // Check worktree status files
            if !active_worktrees.is_empty() {
                logger.log_event(&format!(
                    "Checking {} active worktree(s)",
                    active_worktrees.len()
                ))?;

                for worktree in &active_worktrees {
                    let spec_id = &worktree.spec_id;

                    // Read status file
                    match check_worktree_status(&worktree.path) {
                        Ok(status) => {
                            match status.status {
                                AgentStatusState::Done => {
                                    logger.log_event(&format!(
                                        "Worktree for spec {} reports done",
                                        spec_id.cyan()
                                    ))?;

                                    if dry_run {
                                        logger.log_event(&format!(
                                            "  {} would finalize, merge, and cleanup {}",
                                            "→".dimmed(),
                                            spec_id
                                        ))?;
                                    } else {
                                        // Handle completion (finalize + merge + cleanup)
                                        match crate::cmd::lifecycle::handle_completed(spec_id) {
                                            Ok(()) => {
                                                logger.log_event(&format!(
                                                    "  {} finalized and merged",
                                                    "✓".green()
                                                ))?;
                                            }
                                            Err(e) => {
                                                logger.log_event(&format!(
                                                    "  {} failed to finalize: {}",
                                                    "✗".red(),
                                                    e
                                                ))?;
                                            }
                                        }
                                        // Note: cleanup happens via handle_completed -> merge -> worktree removal
                                    }
                                }
                                AgentStatusState::Failed => {
                                    logger.log_event(&format!(
                                        "Worktree for spec {} reports failed",
                                        spec_id.cyan()
                                    ))?;

                                    if dry_run {
                                        logger.log_event(&format!(
                                            "  {} would mark spec failed and cleanup {}",
                                            "→".dimmed(),
                                            spec_id
                                        ))?;
                                    } else {
                                        // Mark spec as failed
                                        let specs_dir = PathBuf::from(".chant/specs");
                                        if let Ok(mut spec) =
                                            spec::resolve_spec(&specs_dir, spec_id)
                                        {
                                            spec.frontmatter.status = SpecStatus::Failed;
                                            let spec_path =
                                                specs_dir.join(format!("{}.md", spec_id));
                                            if let Err(e) = spec.save(&spec_path) {
                                                logger.log_event(&format!(
                                                    "  {} failed to mark spec failed: {}",
                                                    "✗".red(),
                                                    e
                                                ))?;
                                            } else {
                                                logger.log_event(&format!(
                                                    "  {} marked spec as failed",
                                                    "✓".green()
                                                ))?;

                                                // Handle failure (retry or cleanup)
                                                match crate::cmd::lifecycle::handle_failed(
                                                    spec_id,
                                                    &config.watch.failure,
                                                ) {
                                                    Ok(()) => {
                                                        logger.log_event(&format!(
                                                            "  {} failure handled",
                                                            "✓".green()
                                                        ))?;
                                                    }
                                                    Err(e) => {
                                                        logger.log_event(&format!(
                                                            "  {} failed to handle failure: {}",
                                                            "✗".red(),
                                                            e
                                                        ))?;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                AgentStatusState::Working => {
                                    // Agent is still working, nothing to do
                                }
                            }
                        }
                        Err(e) => {
                            let err_msg = e.to_string();
                            if err_msg.contains("not found") {
                                // No status file yet - agent may not have started writing
                                logger.log_event(&format!(
                                    "{} Worktree {} has no status file yet",
                                    "⚠".yellow(),
                                    worktree.path.display()
                                ))?;
                            } else if err_msg.contains("parse") {
                                // Status file is corrupt
                                logger.log_event(&format!(
                                    "{} Worktree {} has corrupt status file: {}",
                                    "⚠".yellow(),
                                    worktree.path.display(),
                                    e
                                ))?;
                            } else {
                                // Other I/O error (worktree deleted, permission denied, etc.)
                                logger.log_event(&format!(
                                    "{} Failed to read status for worktree {}: {}",
                                    "⚠".yellow(),
                                    worktree.path.display(),
                                    e
                                ))?;
                            }
                        }
                    }
                }
            }
        }

        // Exit after one iteration if --once flag is set
        if once {
            logger.log_event("Single pass complete, exiting")?;
            remove_watch_pid()?;
            break;
        }

        // Sleep for poll interval
        std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_logger_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Create .chant directory (but not logs subdirectory)
        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let logger = WatchLogger::init().unwrap();
        assert!(PathBuf::from(".chant/logs").exists());
        assert!(!logger.is_stdout_only());

        std::env::set_current_dir(&original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_logger_writes_to_file() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("Test message").unwrap();

        // Read using relative path while still in tmp directory
        let contents = fs::read_to_string(".chant/logs/watch.log").unwrap();

        std::env::set_current_dir(&original_dir).unwrap();

        assert!(contents.contains("Test message"));
        assert!(contents.contains("["));
        assert!(contents.contains("]"));
    }

    #[test]
    #[serial]
    fn test_logger_multiple_events() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("Event 1").unwrap();
        logger.log_event("Event 2").unwrap();
        logger.log_event("Event 3").unwrap();

        // Read using relative path while still in tmp directory
        let contents = fs::read_to_string(".chant/logs/watch.log").unwrap();

        std::env::set_current_dir(&original_dir).unwrap();

        assert!(contents.contains("Event 1"));
        assert!(contents.contains("Event 2"));
        assert!(contents.contains("Event 3"));

        // Count lines
        let line_count = contents.lines().count();
        assert_eq!(line_count, 3);
    }

    #[test]
    #[serial]
    fn test_logger_appends_to_existing_file() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let logs_dir = tmp.path().join(".chant/logs");
        fs::create_dir_all(&logs_dir).unwrap();
        let log_path = logs_dir.join("watch.log");
        fs::write(&log_path, "Existing content\n").unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("New event").unwrap();

        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("Existing content"));
        assert!(contents.contains("New event"));

        std::env::set_current_dir(&original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_timestamp_format() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("Test").unwrap();

        // Read using relative path while still in tmp directory
        let contents = fs::read_to_string(".chant/logs/watch.log").unwrap();

        std::env::set_current_dir(&original_dir).unwrap();

        // Should match [HH:MM:SS] format
        assert!(contents.starts_with("["));
        let parts: Vec<&str> = contents.split(']').collect();
        assert!(parts.len() >= 2);
        // Timestamp should be in format [HH:MM:SS]
        let timestamp = parts[0].trim_start_matches('[');
        let time_parts: Vec<&str> = timestamp.split(':').collect();
        assert_eq!(time_parts.len(), 3); // HH:MM:SS
    }
}
