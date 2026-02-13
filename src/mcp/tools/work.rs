//! MCP tools for work execution and management

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::paths::LOGS_DIR;
use crate::spec::{load_all_specs, resolve_spec, SpecStatus};

use super::super::handlers::{
    check_for_running_work_processes, find_project_root, mcp_ensure_initialized,
};
use super::super::response::{mcp_error_response, mcp_text_response};

pub fn tool_chant_work_start(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let chain = args.get("chain").and_then(|v| v.as_bool()).unwrap_or(false);
    let parallel = args.get("parallel").and_then(|v| v.as_u64());
    let skip_criteria = args
        .get("skip_criteria")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Guard: prevent concurrent single/chain execution when not in parallel mode
    if parallel.is_none() {
        match check_for_running_work_processes() {
            Ok(Some((running_spec, pid))) => {
                return Ok(mcp_error_response(format!(
                    "Another work process is already running (spec: {}, PID: {}).\n\
                                 Only one single or chain work process can run at a time.\n\
                                 To run specs concurrently, use the parallel parameter:\n  \
                                 chant_work_start(id=\"<spec>\", parallel=<N>)",
                    running_spec, pid
                )));
            }
            Ok(None) => {
                // No running processes, proceed
            }
            Err(e) => {
                // Log warning but don't block - fail open if we can't check
                eprintln!("Warning: failed to check for running processes: {}", e);
            }
        }
    }

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(mcp_error_response(e.to_string()));
        }
    };

    let spec_id = spec.id.clone();

    // Gate: reject specs in invalid states
    match spec.frontmatter.status {
        SpecStatus::Paused => {
            return Ok(mcp_error_response(format!(
                "Spec '{}' is paused. Cannot start work on a paused spec.\n\
                             Resume the spec first or use `chant reset {}` to reset it to pending.",
                spec_id, spec_id
            )));
        }
        SpecStatus::InProgress => {
            return Ok(mcp_error_response(format!(
                            "Spec '{}' is already in progress. Cannot start work on a spec that is already being worked on.\n\
                             Use `chant takeover {}` to take over the running work.",
                            spec_id, spec_id
                        )));
        }
        SpecStatus::Completed => {
            return Ok(mcp_error_response(format!(
                "Spec '{}' is already completed. Cannot start work on a completed spec.",
                spec_id
            )));
        }
        SpecStatus::Cancelled => {
            return Ok(mcp_error_response(format!(
                "Spec '{}' is cancelled. Cannot start work on a cancelled spec.",
                spec_id
            )));
        }
        _ => {
            // Valid states: Pending, Ready, Failed, NeedsAttention, Blocked
        }
    }

    // Calculate spec quality for advisory feedback (not a gate)
    let quality_warning = if !skip_criteria {
        use crate::config::Config;
        use crate::scoring::{calculate_spec_score, TrafficLight};

        let config = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                return Ok(mcp_error_response(format!("Failed to load config: {}", e)));
            }
        };

        let all_specs = match load_all_specs(&specs_dir) {
            Ok(specs) => specs,
            Err(e) => {
                return Ok(mcp_error_response(format!("Failed to load specs: {}", e)));
            }
        };

        let quality_score = calculate_spec_score(&spec, &all_specs, &config);

        if quality_score.traffic_light == TrafficLight::Refine {
            use crate::score::traffic_light;

            let suggestions = traffic_light::generate_suggestions(&quality_score);
            let mut warning_message =
                "Quality advisory (Red/Refine) - work will start but spec may need improvement:\n\n"
                    .to_string();

            warning_message.push_str("Quality Assessment:\n");
            warning_message.push_str(&format!("  Complexity:    {}\n", quality_score.complexity));
            warning_message.push_str(&format!("  Confidence:    {}\n", quality_score.confidence));
            warning_message.push_str(&format!(
                "  Splittability: {}\n",
                quality_score.splittability
            ));
            warning_message.push_str(&format!("  AC Quality:    {}\n", quality_score.ac_quality));
            if let Some(iso) = quality_score.isolation {
                warning_message.push_str(&format!("  Isolation:     {}\n", iso));
            }

            if !suggestions.is_empty() {
                warning_message.push_str("\nSuggestions:\n");
                for suggestion in &suggestions {
                    warning_message.push_str(&format!("  • {}\n", suggestion));
                }
            }

            Some(json!({
                "status": "refine",
                "scores": {
                    "complexity": quality_score.complexity.to_string(),
                    "confidence": quality_score.confidence.to_string(),
                    "splittability": quality_score.splittability.to_string(),
                    "ac_quality": quality_score.ac_quality.to_string(),
                    "isolation": quality_score.isolation.map(|i| i.to_string())
                },
                "suggestions": suggestions,
                "message": warning_message
            }))
        } else {
            None
        }
    } else {
        None
    };

    // Note: Do NOT transition to InProgress here. The spawned `chant work`
    // handles that transition itself (single.rs line 179). Setting it here
    // causes `chant work` to reject the spec with "already in progress",
    // leaving it stuck as in_progress forever (the error goes to /dev/null).

    // Build command based on mode
    let mut cmd = Command::new("chant");
    cmd.arg("work");

    if skip_criteria {
        cmd.arg("--skip-criteria");
    }

    let mode = if let Some(p) = parallel {
        cmd.arg("--parallel").arg(p.to_string());
        format!("parallel({})", p)
    } else if chain {
        cmd.arg("--chain").arg(&spec_id);
        "chain".to_string()
    } else {
        cmd.arg(&spec_id);
        "single".to_string()
    };

    // Spawn as background process
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().context("Failed to spawn chant work process")?;

    let pid = child.id();
    let started_at = chrono::Local::now().to_rfc3339();
    let process_id = format!("{}-{}", spec_id, pid);

    // Spawn a thread to reap the process when it exits (prevents zombies)
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    // Store process info
    let project_root =
        find_project_root().ok_or_else(|| anyhow::anyhow!("Project root not found"))?;
    let processes_dir = project_root.join(".chant/processes");
    std::fs::create_dir_all(&processes_dir)?;

    let mut process_info = json!({
        "process_id": process_id,
        "spec_id": spec_id,
        "pid": pid,
        "started_at": started_at,
        "mode": mode,
        "started": true,
        "monitor": {
            "instruction": "Poll spec status every 60 seconds using chant_spec_get until status changes to completed or failed. Use chant_log with the since parameter to check progress.",
            "poll_tool": "chant_spec_get",
            "poll_interval_seconds": 60,
            "done_statuses": ["completed", "failed"]
        }
    });

    // Include quality warning if present
    if let Some(warning) = quality_warning {
        process_info["quality_warning"] = warning;
    }

    let process_file = processes_dir.join(format!("{}.json", process_id));
    std::fs::write(&process_file, serde_json::to_string_pretty(&process_info)?)?;

    Ok(mcp_text_response(serde_json::to_string_pretty(
        &process_info,
    )?))
}

pub fn tool_chant_work_list(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let include_completed = arguments
        .and_then(|a| a.get("include_completed"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Use PID files to determine running processes (reliable source of truth)
    let active_pids = crate::pid::list_active_pids()?;

    // Load all specs to get metadata
    let all_specs = load_all_specs(&specs_dir)?;
    let spec_map: std::collections::HashMap<String, &crate::spec::Spec> =
        all_specs.iter().map(|s| (s.id.clone(), s)).collect();

    let mut processes: Vec<Value> = Vec::new();
    let mut running = 0;
    let mut stale_count = 0;

    let logs_dir = PathBuf::from(LOGS_DIR);

    // Report processes with active PIDs
    for (spec_id, pid, is_running) in &active_pids {
        if !is_running {
            // Self-healing: clean up stale PID and process files
            let _ = crate::pid::remove_pid_file(spec_id);
            let _ = crate::pid::remove_process_files(spec_id);
            stale_count += 1;
            if !include_completed {
                continue;
            }
        } else {
            running += 1;
        }

        let spec = spec_map.get(spec_id);
        let title = spec.and_then(|s| s.title.as_deref());
        let branch = spec.and_then(|s| s.frontmatter.branch.as_deref());
        let spec_status = spec.map(|s| &s.frontmatter.status);

        let log_path = logs_dir.join(format!("{}.log", spec_id));
        let log_mtime = if log_path.exists() {
            std::fs::metadata(&log_path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                })
        } else {
            None
        };

        let is_dead_worker = !is_running && matches!(spec_status, Some(SpecStatus::InProgress));

        let mut entry = json!({
            "spec_id": spec_id,
            "title": title,
            "pid": pid,
            "status": if *is_running { "running" } else { "stale" },
            "log_modified": log_mtime,
            "branch": branch
        });

        if is_dead_worker {
            entry["warning"] = json!("process_dead");
        }

        processes.push(entry);
    }

    let summary = json!({
        "running": running,
        "stale": stale_count
    });

    let response = json!({
        "processes": processes,
        "summary": summary
    });

    Ok(mcp_text_response(serde_json::to_string_pretty(&response)?))
}

pub fn tool_chant_pause(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    // Resolve spec to get full ID
    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(mcp_error_response(e.to_string()));
        }
    };

    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec_id));

    // Use operations layer (MCP always forces pause)
    let options = crate::operations::PauseOptions { force: true };
    crate::operations::pause_spec(&mut spec, &spec_path, options)?;

    Ok(mcp_text_response(format!(
        "Successfully paused work for spec '{}'",
        spec_id
    )))
}

pub fn tool_chant_takeover(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(mcp_error_response(e.to_string()));
        }
    };

    // Execute takeover
    match crate::takeover::cmd_takeover(&spec.id, force) {
        Ok(result) => {
            let response = json!({
                "spec_id": result.spec_id,
                "analysis": result.analysis,
                "log_tail": result.log_tail,
                "suggestion": result.suggestion,
                "worktree_path": result.worktree_path
            });

            Ok(mcp_text_response(serde_json::to_string_pretty(&response)?))
        }
        Err(e) => Ok(mcp_error_response(format!(
            "Failed to take over spec '{}': {}",
            spec.id, e
        ))),
    }
}

pub fn tool_chant_split(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    let recursive = args
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_depth = args.get("max_depth").and_then(|v| v.as_u64());

    // Resolve spec to validate it exists
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(mcp_error_response(e.to_string()));
        }
    };

    let spec_id = spec.id.clone();

    // Check if spec is in valid state for splitting
    match spec.frontmatter.status {
        SpecStatus::Pending => {
            // Valid for splitting
        }
        _ => {
            return Ok(mcp_error_response(format!(
                "Spec '{}' must be in pending status to split. Current status: {:?}",
                spec_id, spec.frontmatter.status
            )));
        }
    }

    // Build command
    let mut cmd = Command::new("chant");
    cmd.arg("split");
    cmd.arg(&spec_id);

    if force {
        cmd.arg("--force");
    }
    if recursive {
        cmd.arg("--recursive");
    }
    if let Some(depth) = max_depth {
        cmd.arg("--max-depth").arg(depth.to_string());
    }

    // Spawn as background process
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().context("Failed to spawn chant split process")?;

    let pid = child.id();
    let started_at = chrono::Local::now().to_rfc3339();
    let process_id = format!("split-{}-{}", spec_id, pid);

    // Spawn a thread to reap the process when it exits (prevents zombies)
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    // Store process info
    let project_root =
        find_project_root().ok_or_else(|| anyhow::anyhow!("Project root not found"))?;
    let processes_dir = project_root.join(".chant/processes");
    std::fs::create_dir_all(&processes_dir)?;

    let process_info = json!({
        "process_id": process_id,
        "spec_id": spec_id,
        "pid": pid,
        "started_at": started_at,
        "mode": "split",
        "options": {
            "force": force,
            "recursive": recursive,
            "max_depth": max_depth
        }
    });

    let process_file = processes_dir.join(format!("{}.json", process_id));
    std::fs::write(&process_file, serde_json::to_string_pretty(&process_info)?)?;

    Ok(mcp_text_response(serde_json::to_string_pretty(
        &process_info,
    )?))
}
