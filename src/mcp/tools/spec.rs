//! MCP tools for spec query and management

use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::str::FromStr;

use crate::diagnose;
use crate::paths::LOGS_DIR;
use crate::spec::{load_all_specs, resolve_spec, SpecStatus, SpecType};
use crate::spec_group;

use super::super::handlers::mcp_ensure_initialized;

pub fn tool_chant_spec_list(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let mut specs = load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Filter by status if provided
    if let Some(args) = arguments {
        if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
            let filter_status = SpecStatus::from_str(status_str).ok();

            if let Some(status) = filter_status {
                specs.retain(|s| s.frontmatter.status == status);
            }
        } else {
            // No status filter provided - filter out cancelled specs by default
            specs.retain(|s| s.frontmatter.status != SpecStatus::Cancelled);
        }
    } else {
        // No arguments provided - filter out cancelled specs by default
        specs.retain(|s| s.frontmatter.status != SpecStatus::Cancelled);
    }

    // Get limit (default 50)
    let limit = arguments
        .and_then(|a| a.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let total = specs.len();
    let limited_specs: Vec<_> = specs.into_iter().take(limit).collect();

    let specs_json: Vec<Value> = limited_specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type,
                "depends_on": s.frontmatter.depends_on,
                "labels": s.frontmatter.labels
            })
        })
        .collect();

    let response = json!({
        "specs": specs_json,
        "total": total,
        "limit": limit,
        "returned": limited_specs.len()
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)?
            }
        ]
    }))
}

pub fn tool_chant_spec_get(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let id = arguments
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_json = json!({
        "id": spec.id,
        "title": spec.title,
        "status": format!("{:?}", spec.frontmatter.status).to_lowercase(),
        "type": spec.frontmatter.r#type,
        "depends_on": spec.frontmatter.depends_on,
        "labels": spec.frontmatter.labels,
        "target_files": spec.frontmatter.target_files,
        "context": spec.frontmatter.context,
        "prompt": spec.frontmatter.prompt,
        "branch": spec.frontmatter.branch,
        "commits": spec.frontmatter.commits,
        "completed_at": spec.frontmatter.completed_at,
        "model": spec.frontmatter.model,
        "body": spec.body
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&spec_json)?
            }
        ]
    }))
}

pub fn tool_chant_ready(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let mut specs = load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Filter to ready specs only
    let all_specs = specs.clone();
    specs.retain(|s| s.is_ready(&all_specs));
    // Filter out group specs - they are containers, not actionable work
    specs.retain(|s| s.frontmatter.r#type != SpecType::Group);

    // Get limit (default 50)
    let limit = arguments
        .and_then(|a| a.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let total = specs.len();
    let limited_specs: Vec<_> = specs.into_iter().take(limit).collect();

    let specs_json: Vec<Value> = limited_specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type,
                "depends_on": s.frontmatter.depends_on,
                "labels": s.frontmatter.labels
            })
        })
        .collect();

    let response = json!({
        "specs": specs_json,
        "total": total,
        "limit": limit,
        "returned": limited_specs.len()
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)?
            }
        ]
    }))
}

pub fn tool_chant_spec_update(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Parse status if provided
    let status = if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
        match SpecStatus::from_str(status_str) {
            Ok(s) => Some(s),
            Err(e) => {
                return Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("{}", e)
                        }
                    ],
                    "isError": true
                }));
            }
        }
    } else {
        None
    };

    // Parse other fields
    let depends_on = args
        .get("depends_on")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let labels = args.get("labels").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    let target_files = args
        .get("target_files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

    let model = args.get("model").and_then(|v| v.as_str()).map(String::from);

    let output = args
        .get("output")
        .and_then(|v| v.as_str())
        .map(String::from);

    let replace_body = args
        .get("replace_body")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Parse force parameter if provided
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

    // Use operations module for update
    let options = crate::operations::update::UpdateOptions {
        status,
        depends_on,
        labels,
        target_files,
        model,
        output,
        replace_body,
        force,
    };

    match crate::operations::update::update_spec(&mut spec, &spec_path, options) {
        Ok(_) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Updated spec: {}", spec_id)
                }
            ]
        })),
        Err(e) => Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("Failed to update spec: {}", e)
                }
            ],
            "isError": true
        })),
    }
}

pub fn tool_chant_status(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let specs = load_all_specs(&specs_dir)?;

    // Parse options
    let brief = arguments
        .and_then(|a| a.get("brief"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_activity = arguments
        .and_then(|a| a.get("include_activity"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Count by status
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut blocked = 0;
    let mut cancelled = 0;
    let mut needs_attention = 0;

    for spec in &specs {
        match spec.frontmatter.status {
            SpecStatus::Pending => pending += 1,
            SpecStatus::InProgress => in_progress += 1,
            SpecStatus::Paused => in_progress += 1, // Count paused as in_progress for summary
            SpecStatus::Completed => completed += 1,
            SpecStatus::Failed => failed += 1,
            SpecStatus::Ready => pending += 1, // Ready is computed, treat as pending
            SpecStatus::Blocked => blocked += 1,
            SpecStatus::Cancelled => cancelled += 1,
            SpecStatus::NeedsAttention => needs_attention += 1,
        }
    }

    // Brief output mode
    if brief {
        let mut parts = vec![];
        if pending > 0 {
            parts.push(format!("{} pending", pending));
        }
        if in_progress > 0 {
            parts.push(format!("{} in_progress", in_progress));
        }
        if completed > 0 {
            parts.push(format!("{} completed", completed));
        }
        if failed > 0 {
            parts.push(format!("{} failed", failed));
        }
        if blocked > 0 {
            parts.push(format!("{} blocked", blocked));
        }
        if cancelled > 0 {
            parts.push(format!("{} cancelled", cancelled));
        }
        if needs_attention > 0 {
            parts.push(format!("{} needs_attention", needs_attention));
        }
        let brief_text = if parts.is_empty() {
            "No specs".to_string()
        } else {
            parts.join(" | ")
        };
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": brief_text
                }
            ]
        }));
    }

    // Build status response
    let mut status_json = json!({
        "total": specs.len(),
        "pending": pending,
        "in_progress": in_progress,
        "completed": completed,
        "failed": failed,
        "blocked": blocked,
        "cancelled": cancelled,
        "needs_attention": needs_attention
    });

    // Include activity info for in_progress specs
    if include_activity {
        let logs_dir = PathBuf::from(LOGS_DIR);
        let mut activity: Vec<Value> = vec![];

        for spec in &specs {
            if spec.frontmatter.status != SpecStatus::InProgress {
                continue;
            }

            let spec_path = specs_dir.join(format!("{}.md", spec.id));
            let log_path = logs_dir.join(format!("{}.log", spec.id));

            // Get spec file modification time
            let spec_mtime = std::fs::metadata(&spec_path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                });

            // Get log file modification time (indicates last agent activity)
            let log_mtime = std::fs::metadata(&log_path)
                .and_then(|m| m.modified())
                .ok()
                .map(|t| {
                    chrono::DateTime::<chrono::Local>::from(t)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                });

            // Check if log file exists
            let has_log = log_path.exists();

            activity.push(json!({
                "id": spec.id,
                "title": spec.title,
                "spec_modified": spec_mtime,
                "log_modified": log_mtime,
                "has_log": has_log
            }));
        }

        status_json["in_progress_activity"] = json!(activity);
    }

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&status_json)?
            }
        ]
    }))
}

pub fn tool_chant_log(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let lines = args
        .get("lines")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let byte_offset = args.get("offset").and_then(|v| v.as_u64());
    let since = args.get("since").and_then(|v| v.as_str());

    // Resolve spec to get full ID
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let logs_dir = PathBuf::from(LOGS_DIR);
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if !log_path.exists() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": format!("No log file found for spec '{}'. Logs are created when a spec is executed with `chant work`.", spec.id)
                }
            ],
            "isError": true
        }));
    }

    // Read log file
    let content = std::fs::read_to_string(&log_path)?;
    let file_byte_len = content.len() as u64;

    // Filter by offset if provided
    let content_after_offset = if let Some(offset) = byte_offset {
        if offset >= file_byte_len {
            // Offset is at or beyond end of file
            String::new()
        } else {
            content[(offset as usize)..].to_string()
        }
    } else {
        content.clone()
    };

    // Filter by timestamp if provided
    let content_after_since = if let Some(since_ts) = since {
        if let Ok(since_time) = chrono::DateTime::parse_from_rfc3339(since_ts) {
            content_after_offset
                .lines()
                .filter(|line| {
                    // Try to parse timestamp from line start
                    // Assumes log format: YYYY-MM-DDTHH:MM:SS.sssZ ...
                    if line.len() >= 24 {
                        if let Ok(line_time) = chrono::DateTime::parse_from_rfc3339(&line[..24]) {
                            return line_time > since_time;
                        }
                    }
                    true // Include lines without parseable timestamps
                })
                .collect::<Vec<&str>>()
                .join("\n")
        } else {
            content_after_offset
        }
    } else {
        content_after_offset
    };

    // Apply lines limit
    let all_lines: Vec<&str> = content_after_since.lines().collect();
    let lines_limit = lines.unwrap_or(100);
    let start = if all_lines.len() > lines_limit {
        all_lines.len() - lines_limit
    } else {
        0
    };
    let log_output = all_lines[start..].join("\n");

    // Calculate new byte offset
    let new_byte_offset = if byte_offset.is_some() {
        file_byte_len
    } else {
        content.len() as u64
    };

    let has_more = all_lines.len() > lines_limit;
    let line_count = all_lines[start..].len();

    let response = json!({
        "content": log_output,
        "byte_offset": new_byte_offset,
        "line_count": line_count,
        "has_more": has_more
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)?
            }
        ]
    }))
}

pub fn tool_chant_search(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?
        .to_lowercase();

    let status_filter = args.get("status").and_then(|v| v.as_str());

    let mut specs = load_all_specs(&specs_dir)?;

    // Filter by query (case-insensitive search in title and body)
    specs.retain(|s| {
        let title_match = s
            .title
            .as_ref()
            .map(|t| t.to_lowercase().contains(&query))
            .unwrap_or(false);
        title_match || s.body.to_lowercase().contains(&query)
    });

    // Filter by status if provided
    if let Some(status_str) = status_filter {
        let filter_status = SpecStatus::from_str(status_str).ok();

        if let Some(status) = filter_status {
            specs.retain(|s| s.frontmatter.status == status);
        }
    }

    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    let specs_json: Vec<Value> = specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type
            })
        })
        .collect();

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&specs_json)?
            }
        ]
    }))
}

pub fn tool_chant_diagnose(arguments: Option<&Value>) -> Result<Value> {
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
    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    // Run diagnostics
    let report = match diagnose::diagnose_spec(&spec.id) {
        Ok(r) => r,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Failed to diagnose spec: {}", e)
                    }
                ],
                "isError": true
            }));
        }
    };

    // Format report as JSON
    let checks_json: Vec<Value> = report
        .checks
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "passed": c.passed,
                "details": c.details
            })
        })
        .collect();

    let report_json = json!({
        "spec_id": report.spec_id,
        "status": format!("{:?}", report.status).to_lowercase(),
        "location": report.location,
        "checks": checks_json,
        "diagnosis": report.diagnosis,
        "suggestion": report.suggestion
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&report_json)?
            }
        ]
    }))
}

pub fn tool_chant_lint(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let spec_id = arguments
        .and_then(|args| args.get("id"))
        .and_then(|v| v.as_str());

    use crate::config::Config;
    use crate::score::traffic_light;
    use crate::scoring::{calculate_spec_score, TrafficLight};
    use crate::spec::Spec;

    // Load config for scoring
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load config: {}", e) }],
                "isError": true
            }));
        }
    };

    // Load all specs (needed for isolation scoring)
    let all_specs = load_all_specs(&specs_dir)?;

    // Collect specs to check
    let specs_to_check: Vec<Spec> = if let Some(id) = spec_id {
        match resolve_spec(&specs_dir, id) {
            Ok(spec) => vec![spec],
            Err(e) => {
                return Ok(json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                }));
            }
        }
    } else {
        all_specs.clone()
    };

    let mut results: Vec<Value> = Vec::new();
    let mut red_count = 0;
    let mut yellow_count = 0;
    let mut green_count = 0;

    // Run full quality assessment on each spec (same as chant work)
    for spec in &specs_to_check {
        let score = calculate_spec_score(spec, &all_specs, &config);
        let suggestions = traffic_light::generate_suggestions(&score);

        let traffic_light_str = match score.traffic_light {
            TrafficLight::Ready => {
                green_count += 1;
                "green"
            }
            TrafficLight::Review => {
                yellow_count += 1;
                "yellow"
            }
            TrafficLight::Refine => {
                red_count += 1;
                "red"
            }
        };

        results.push(json!({
            "id": spec.id,
            "title": spec.title,
            "traffic_light": traffic_light_str,
            "complexity": score.complexity.to_string(),
            "confidence": score.confidence.to_string(),
            "splittability": score.splittability.to_string(),
            "ac_quality": score.ac_quality.to_string(),
            "isolation": score.isolation.map(|i| i.to_string()),
            "suggestions": suggestions
        }));
    }

    let summary = json!({
        "specs_checked": specs_to_check.len(),
        "red": red_count,
        "yellow": yellow_count,
        "green": green_count,
        "results": results
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&summary)?
            }
        ]
    }))
}

pub fn tool_chant_add(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: description"))?;

    let prompt = args.get("prompt").and_then(|v| v.as_str());

    // Load config for derivation
    let config = match crate::config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load config: {}", e) }],
                "isError": true
            }));
        }
    };

    // Use operations module for spec creation
    let options = crate::operations::create::CreateOptions {
        prompt: prompt.map(String::from),
        needs_approval: false,
        auto_commit: false, // MCP doesn't auto-commit
    };

    let (spec, _filepath) = match crate::operations::create::create_spec(
        description,
        &specs_dir,
        &config,
        options,
    ) {
        Ok(result) => result,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to create spec: {}", e) }],
                "isError": true
            }));
        }
    };

    // Load all specs for scoring (needed for isolation)
    let all_specs = match load_all_specs(&specs_dir) {
        Ok(specs) => specs,
        Err(e) => {
            return Ok(json!({
                "content": [{ "type": "text", "text": format!("Failed to load specs: {}", e) }],
                "isError": true
            }));
        }
    };

    // Calculate lint score for the newly created spec
    use crate::score::traffic_light;
    use crate::scoring::calculate_spec_score;

    let score = calculate_spec_score(&spec, &all_specs, &config);
    let suggestions = traffic_light::generate_suggestions(&score);

    // Convert traffic light to string
    let traffic_light_str = match score.traffic_light {
        crate::scoring::TrafficLight::Ready => "green",
        crate::scoring::TrafficLight::Review => "yellow",
        crate::scoring::TrafficLight::Refine => "red",
    };

    // Build response JSON with lint results
    let response = json!({
        "spec_id": spec.id,
        "message": format!("Created spec: {}", spec.id),
        "lint": {
            "traffic_light": traffic_light_str,
            "complexity": score.complexity.to_string(),
            "confidence": score.confidence.to_string(),
            "splittability": score.splittability.to_string(),
            "ac_quality": score.ac_quality.to_string(),
            "isolation": score.isolation.map(|i| i.to_string()),
            "suggestions": suggestions
        }
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)?
            }
        ]
    }))
}

pub fn tool_chant_verify(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_id = spec.id.clone();

    // Count checked and unchecked criteria using operations layer
    let unchecked_count = spec.count_unchecked_checkboxes();

    // Find total checkboxes in Acceptance Criteria section
    let total_count: usize = {
        let acceptance_criteria_marker = "## Acceptance Criteria";
        let mut in_ac_section = false;
        let mut in_code_fence = false;
        let mut count = 0;

        for line in spec.body.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                in_ac_section = true;
                continue;
            }

            if in_ac_section && trimmed.starts_with("## ") && !in_code_fence {
                break;
            }

            if in_ac_section
                && !in_code_fence
                && (trimmed.starts_with("- [x]") || trimmed.starts_with("- [ ]"))
            {
                count += 1;
            }
        }

        count
    };

    let checked_count = total_count.saturating_sub(unchecked_count);
    let verified = unchecked_count == 0 && total_count > 0;

    // Extract unchecked items
    let unchecked_items = if unchecked_count > 0 {
        let acceptance_criteria_marker = "## Acceptance Criteria";
        let mut in_ac_section = false;
        let mut in_code_fence = false;
        let mut items = Vec::new();

        for line in spec.body.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                in_ac_section = true;
                continue;
            }

            if in_ac_section && trimmed.starts_with("## ") && !in_code_fence {
                break;
            }

            if in_ac_section && !in_code_fence && trimmed.starts_with("- [ ]") {
                items.push(trimmed.to_string());
            }
        }

        items
    } else {
        Vec::new()
    };

    let verification_notes = if total_count == 0 {
        "No acceptance criteria found".to_string()
    } else if verified {
        "All acceptance criteria met".to_string()
    } else {
        format!("{} criteria not yet checked", unchecked_count)
    };

    let result = json!({
        "spec_id": spec_id,
        "verified": verified,
        "criteria": {
            "total": total_count,
            "checked": checked_count,
            "unchecked": unchecked_count
        },
        "unchecked_items": unchecked_items,
        "verification_notes": verification_notes
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }
        ]
    }))
}
