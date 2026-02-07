//! Takeover command for intervening in running work processes
//!
//! Allows users to stop an autonomous agent and continue with manual guidance.

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::paths::LOGS_DIR;
use crate::pid;
use crate::spec::{self, SpecStatus};
use crate::worktree;

/// Result of a takeover operation
pub struct TakeoverResult {
    pub spec_id: String,
    pub analysis: String,
    pub log_tail: String,
    pub suggestion: String,
    pub worktree_path: Option<String>,
}

/// Takeover a spec that is currently being worked on
pub fn cmd_takeover(id: &str, force: bool) -> Result<TakeoverResult> {
    let specs_dir = PathBuf::from(crate::paths::SPECS_DIR);
    if !specs_dir.exists() {
        anyhow::bail!("Not a chant project (no .chant/ directory found)");
    }

    // Resolve the spec ID
    let mut spec = spec::resolve_spec(&specs_dir, id)?;
    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    println!("{} Taking over spec {}", "→".cyan(), spec_id.cyan());

    // Pause the work (stops process and sets status to paused)
    let pid = pid::read_pid_file(&spec_id)?;
    let was_running = if let Some(pid) = pid {
        if pid::is_process_running(pid) {
            println!("  {} Stopping running process (PID: {})", "•".cyan(), pid);
            pid::stop_process(pid)?;
            pid::remove_pid_file(&spec_id)?;
            println!("  {} Process stopped", "✓".green());
            true
        } else {
            println!("  {} Cleaning up stale PID file", "•".cyan());
            pid::remove_pid_file(&spec_id)?;
            false
        }
    } else {
        if !force {
            anyhow::bail!(
                "Spec {} is not currently running. Use --force to analyze anyway.",
                spec_id
            );
        }
        false
    };

    // Read and analyze the log
    let log_path = PathBuf::from(LOGS_DIR).join(format!("{}.log", spec_id));
    let (log_tail, analysis) = if log_path.exists() {
        let log_content = fs::read_to_string(&log_path)
            .with_context(|| format!("Failed to read log file: {}", log_path.display()))?;

        let tail = get_log_tail(&log_content, 50);
        let analysis = analyze_log(&log_content);

        (tail, analysis)
    } else {
        (
            "No log file found".to_string(),
            "No execution log available for analysis".to_string(),
        )
    };

    // Generate suggestion based on spec status and analysis
    let suggestion = generate_suggestion(&spec, &analysis);

    // Check for worktree path
    let config = Config::load().ok();
    let project_name = config.as_ref().map(|c| c.project.name.as_str());
    let worktree_path = worktree::get_active_worktree(&spec_id, project_name);
    let worktree_exists = worktree_path.is_some();
    let worktree_path_str = worktree_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    // Update spec status to paused if it was in_progress
    if spec.frontmatter.status == SpecStatus::InProgress {
        let _ = spec.set_status(SpecStatus::Paused);
    }

    // Append takeover analysis to spec body
    let worktree_info = if let Some(ref path) = worktree_path_str {
        format!(
            "\n\n### Worktree Location\n\nWork should be done in the isolated worktree:\n```\ncd {}\n```\n",
            path
        )
    } else {
        "\n\n### Worktree Location\n\nWorktree no longer exists (agent may have cleaned up). If you need to continue working, recreate the worktree with `chant work <spec-id>`.\n".to_string()
    };

    let takeover_section = format!(
        "\n\n## Takeover Analysis\n\n{}\n\n### Recent Log Activity\n\n```\n{}\n```\n{}\n### Recommendation\n\n{}\n",
        analysis,
        log_tail,
        worktree_info,
        suggestion
    );

    spec.body.push_str(&takeover_section);
    spec.save(&spec_path)?;

    println!("{} Updated spec with takeover analysis", "✓".green());
    if was_running {
        println!("  {} Status set to: paused", "•".cyan());
    }
    println!("  {} Analysis appended to spec body", "•".cyan());
    if worktree_exists {
        println!(
            "  {} Worktree at: {}",
            "•".cyan(),
            worktree_path_str.as_ref().unwrap()
        );
    } else {
        println!("  {} Worktree no longer exists", "⚠".yellow());
    }

    Ok(TakeoverResult {
        spec_id,
        analysis,
        log_tail,
        suggestion,
        worktree_path: worktree_path_str,
    })
}

/// Get the last N lines from a log
fn get_log_tail(log_content: &str, lines: usize) -> String {
    log_content
        .lines()
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Analyze log content to understand what went wrong
fn analyze_log(log_content: &str) -> String {
    let lines: Vec<&str> = log_content.lines().collect();

    if lines.is_empty() {
        return "Log is empty - no execution activity recorded.".to_string();
    }

    let mut analysis = Vec::new();

    // Check for common error patterns
    let error_indicators = ["error:", "failed:", "ERROR", "FAIL", "exception", "panic"];
    let errors: Vec<&str> = lines
        .iter()
        .filter(|line| {
            error_indicators
                .iter()
                .any(|indicator| line.to_lowercase().contains(indicator))
        })
        .copied()
        .collect();

    if !errors.is_empty() {
        analysis.push(format!("Found {} error indicator(s) in log:", errors.len()));
        for error in errors.iter().take(3) {
            analysis.push(format!("  - {}", error.trim()));
        }
        if errors.len() > 3 {
            analysis.push(format!("  ... and {} more", errors.len() - 3));
        }
    }

    // Check for tool usage patterns
    let tool_indicators = [
        "<function_calls>",
        "tool_name",
        "Bash",
        "Read",
        "Edit",
        "Write",
    ];
    let tool_uses: Vec<&str> = lines
        .iter()
        .filter(|line| {
            tool_indicators
                .iter()
                .any(|indicator| line.contains(indicator))
        })
        .copied()
        .collect();

    if !tool_uses.is_empty() {
        analysis.push(format!("\nAgent made {} tool call(s)", tool_uses.len()));
    }

    // Check for completion indicators
    let completion_indicators = ["completed", "finished", "done", "success"];
    let has_completion = lines.iter().any(|line| {
        completion_indicators
            .iter()
            .any(|indicator| line.to_lowercase().contains(indicator))
    });

    if has_completion {
        analysis.push("\nLog contains completion indicators.".to_string());
    }

    // Estimate progress
    let total_lines = lines.len();
    analysis.push(format!("\nLog contains {} lines of output.", total_lines));

    if errors.is_empty() && !has_completion {
        analysis.push("\nNo errors detected, but work appears incomplete.".to_string());
    }

    if analysis.is_empty() {
        "Agent execution started but no significant activity detected.".to_string()
    } else {
        analysis.join("\n")
    }
}

/// Generate a suggestion based on spec and analysis
fn generate_suggestion(spec: &spec::Spec, analysis: &str) -> String {
    let mut suggestions: Vec<String> = Vec::new();

    // Check if there are errors
    if analysis.to_lowercase().contains("error") {
        suggestions
            .push("Review the errors in the log and address them before resuming.".to_string());
    }

    // Check acceptance criteria
    let unchecked = spec.count_unchecked_checkboxes();
    if unchecked > 0 {
        suggestions.push(format!(
            "{} acceptance criteria remain unchecked.",
            unchecked
        ));
    }

    // General suggestions
    suggestions.push("Continue working on this spec manually or adjust the approach.".to_string());
    suggestions
        .push("When ready to resume automated work, use `chant work <spec-id>`.".to_string());

    suggestions.join("\n")
}
