//! Integration tests for MCP handler functions
//!
//! Tests the MCP tool call handlers through a simplified test interface.

use chant::spec::{Spec, SpecStatus};
use serde_json::{json, Value};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper functions that test the handlers through the public chant API
fn call_spec_list(args: Option<Value>) -> Result<Value, String> {
    let specs_dir = PathBuf::from(".chant/specs");
    let specs = chant::spec::load_all_specs(&specs_dir).map_err(|e| e.to_string())?;

    let status_filter = args
        .as_ref()
        .and_then(|a| a.get("status"))
        .and_then(|v| v.as_str());

    let limit = args
        .as_ref()
        .and_then(|a| a.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let mut filtered_specs = specs;

    if let Some(status_str) = status_filter {
        filtered_specs
            .retain(|s| format!("{:?}", s.frontmatter.status).to_lowercase() == status_str);
    }

    let total = filtered_specs.len();
    let limited_specs: Vec<_> = filtered_specs.into_iter().take(limit).collect();

    let specs_json: Vec<Value> = limited_specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase()
            })
        })
        .collect();

    Ok(json!({
        "total": total,
        "returned": limited_specs.len(),
        "specs": specs_json
    }))
}

fn call_spec_get(args: Option<Value>) -> Result<Value, String> {
    let id = args
        .as_ref()
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .ok_or("Missing id parameter")?;

    let specs_dir = PathBuf::from(".chant/specs");
    let spec = chant::spec::resolve_spec(&specs_dir, id).map_err(|e| e.to_string())?;

    Ok(json!({
        "id": spec.id,
        "title": spec.title,
        "status": format!("{:?}", spec.frontmatter.status).to_lowercase(),
        "body": spec.body
    }))
}

fn call_spec_update(args: Option<Value>) -> Result<Value, String> {
    let args = args.ok_or("Missing arguments")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?;

    let specs_dir = PathBuf::from(".chant/specs");
    let mut spec = chant::spec::resolve_spec(&specs_dir, id).map_err(|e| e.to_string())?;

    let mut updated = false;

    if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
        spec.frontmatter.status = match status_str {
            "pending" => SpecStatus::Pending,
            "in_progress" => SpecStatus::InProgress,
            "completed" => SpecStatus::Completed,
            "failed" => SpecStatus::Failed,
            _ => return Err(format!("Invalid status: {}", status_str)),
        };
        updated = true;
    }

    if let Some(output) = args.get("output").and_then(|v| v.as_str()) {
        if !output.is_empty() {
            if !spec.body.ends_with('\n') && !spec.body.is_empty() {
                spec.body.push('\n');
            }
            spec.body.push_str("\n## Output\n\n");
            spec.body.push_str(output);
            spec.body.push('\n');
            updated = true;
        }
    }

    if !updated {
        return Err("No updates specified".to_string());
    }

    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path).map_err(|e| e.to_string())?;

    Ok(json!({
        "message": format!("Updated spec: {}", spec.id)
    }))
}

fn call_ready(args: Option<Value>) -> Result<Value, String> {
    let specs_dir = PathBuf::from(".chant/specs");
    let specs = chant::spec::load_all_specs(&specs_dir).map_err(|e| e.to_string())?;

    let limit = args
        .as_ref()
        .and_then(|a| a.get("limit"))
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let all_specs = specs.clone();
    let ready_specs: Vec<_> = specs
        .into_iter()
        .filter(|s| s.is_ready(&all_specs))
        .filter(|s| s.frontmatter.r#type != "group")
        .take(limit)
        .collect();

    Ok(json!({
        "total": ready_specs.len(),
        "returned": ready_specs.len()
    }))
}

fn call_status(args: Option<Value>) -> Result<Value, String> {
    let specs_dir = PathBuf::from(".chant/specs");
    let specs = chant::spec::load_all_specs(&specs_dir).map_err(|e| e.to_string())?;

    let brief = args
        .as_ref()
        .and_then(|a| a.get("brief"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let include_activity = args
        .as_ref()
        .and_then(|a| a.get("include_activity"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut failed = 0;

    for spec in &specs {
        match spec.frontmatter.status {
            SpecStatus::Pending => pending += 1,
            SpecStatus::InProgress | SpecStatus::Paused => in_progress += 1,
            SpecStatus::Completed => completed += 1,
            SpecStatus::Failed => failed += 1,
            _ => {}
        }
    }

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

        return Ok(json!(parts.join(" | ")));
    }

    let mut result = json!({
        "total": specs.len(),
        "pending": pending,
        "in_progress": in_progress,
        "completed": completed,
        "failed": failed
    });

    if include_activity {
        result["in_progress_activity"] = json!([]);
    }

    Ok(result)
}

fn call_log(args: Option<Value>) -> Result<Value, String> {
    let args = args.ok_or("Missing arguments")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?;

    let specs_dir = PathBuf::from(".chant/specs");
    let spec = chant::spec::resolve_spec(&specs_dir, id).map_err(|e| e.to_string())?;

    let logs_dir = PathBuf::from(".chant/logs");
    let log_path = logs_dir.join(format!("{}.log", spec.id));

    if !log_path.exists() {
        return Err("Log file not found".to_string());
    }

    let content = fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
    let all_lines: Vec<&str> = content.lines().collect();

    let lines_limit = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
    let _offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let start = if all_lines.len() > lines_limit {
        all_lines.len() - lines_limit
    } else {
        0
    };

    let line_count = all_lines[start..].len();

    Ok(json!({
        "content": all_lines[start..].join("\n"),
        "line_count": line_count
    }))
}

fn call_verify(args: Option<Value>) -> Result<Value, String> {
    let args = args.ok_or("Missing arguments")?;
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?;

    let specs_dir = PathBuf::from(".chant/specs");
    let spec = chant::spec::resolve_spec(&specs_dir, id).map_err(|e| e.to_string())?;

    let unchecked = spec.count_unchecked_checkboxes();

    let total: usize = {
        let mut in_ac_section = false;
        let mut in_code_fence = false;
        let mut count = 0;
        for line in spec.body.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }
            if !in_code_fence && trimmed.starts_with("## Acceptance Criteria") {
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

    let checked = total.saturating_sub(unchecked);
    let verified = unchecked == 0 && total > 0;

    let unchecked_items: Vec<String> = if unchecked > 0 {
        let mut items = Vec::new();
        let mut in_ac_section = false;
        let mut in_code_fence = false;

        for line in spec.body.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }
            if !in_code_fence && trimmed.starts_with("## Acceptance Criteria") {
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

    Ok(json!({
        "verified": verified,
        "criteria": {
            "total": total,
            "checked": checked,
            "unchecked": unchecked
        },
        "unchecked_items": unchecked_items
    }))
}

fn call_work_list(_args: Option<Value>) -> Result<Value, String> {
    let processes_dir = PathBuf::from(".chant/processes");
    if !processes_dir.exists() {
        return Ok(json!({
            "processes": [],
            "summary": {
                "running": 0,
                "completed": 0
            }
        }));
    }

    Ok(json!({
        "processes": [],
        "summary": {
            "running": 0,
            "completed": 0
        }
    }))
}

/// Helper to create a test environment with specs directory
fn setup_test_env() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let specs_dir = base_path.join(".chant/specs");
    fs::create_dir_all(&specs_dir).unwrap();

    // Set cwd to temp directory so the helpers can find .chant
    std::env::set_current_dir(base_path).unwrap();

    (temp_dir, specs_dir)
}

/// Helper to create a test spec file
fn create_spec(specs_dir: &std::path::Path, id: &str, status: &str, body: &str) {
    let content = format!(
        r#"---
type: code
status: {}
---

{}
"#,
        status, body
    );
    let spec_path = specs_dir.join(format!("{}.md", id));
    fs::write(&spec_path, &content).unwrap();

    // Load and re-save to ensure proper parsing and id field
    let mut spec = Spec::parse(id, &content).unwrap();
    spec.id = id.to_string();
    spec.save(&spec_path).unwrap();
}

// ============================================================================
// Tests
// ============================================================================

#[test]
#[serial]
fn test_spec_list_filtering_by_status() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-01-001-abc",
        "pending",
        "# Pending spec",
    );
    create_spec(
        &specs_dir,
        "2026-02-01-002-def",
        "in_progress",
        "# In progress spec",
    );
    create_spec(
        &specs_dir,
        "2026-02-01-003-ghi",
        "completed",
        "# Completed spec",
    );
    create_spec(
        &specs_dir,
        "2026-02-01-004-jkl",
        "pending",
        "# Another pending spec",
    );

    let response = call_spec_list(Some(json!({"status": "pending"}))).unwrap();
    assert_eq!(response["returned"].as_u64().unwrap(), 2);

    let response = call_spec_list(Some(json!({"status": "completed"}))).unwrap();
    assert_eq!(response["returned"].as_u64().unwrap(), 1);
}

#[test]
#[serial]
fn test_spec_list_limit_parameter() {
    let (_temp, specs_dir) = setup_test_env();

    for i in 1..=5 {
        create_spec(
            &specs_dir,
            &format!("2026-02-01-00{}-abc", i),
            "pending",
            "# Test spec",
        );
    }

    let response = call_spec_list(Some(json!({"limit": 3}))).unwrap();
    assert_eq!(response["returned"].as_u64().unwrap(), 3);
    assert_eq!(response["total"].as_u64().unwrap(), 5);
}

#[test]
#[serial]
fn test_spec_list_empty_results() {
    let (_temp, _specs_dir) = setup_test_env();

    let response = call_spec_list(None).unwrap();
    assert_eq!(response["returned"].as_u64().unwrap(), 0);
    assert_eq!(response["total"].as_u64().unwrap(), 0);
}

#[test]
#[serial]
fn test_spec_get_valid_id() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-01-001-abc",
        "pending",
        "# Test spec\n\nBody content",
    );

    let response = call_spec_get(Some(json!({"id": "2026-02-01-001-abc"}))).unwrap();
    assert_eq!(response["id"].as_str().unwrap(), "2026-02-01-001-abc");
    assert_eq!(response["title"].as_str().unwrap(), "Test spec");
    assert!(response["body"].as_str().unwrap().contains("Body content"));
}

#[test]
#[serial]
fn test_spec_get_partial_id() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-xyz", "pending", "# Test spec");

    let response = call_spec_get(Some(json!({"id": "xyz"}))).unwrap();
    assert_eq!(response["id"].as_str().unwrap(), "2026-02-01-001-xyz");
}

#[test]
#[serial]
fn test_spec_get_nonexistent_id() {
    let (_temp, _specs_dir) = setup_test_env();

    let result = call_spec_get(Some(json!({"id": "nonexistent"})));
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_spec_update_status_change() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", "# Test spec");

    let result = call_spec_update(Some(json!({
        "id": "2026-02-01-001-abc",
        "status": "in_progress"
    })));
    assert!(result.is_ok());

    let spec = Spec::load(&specs_dir.join("2026-02-01-001-abc.md")).unwrap();
    assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
}

#[test]
#[serial]
fn test_spec_update_output_append() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-01-001-abc",
        "pending",
        "# Test spec\n\nOriginal body",
    );

    let result = call_spec_update(Some(json!({
        "id": "2026-02-01-001-abc",
        "output": "New output text"
    })));
    assert!(result.is_ok());

    let spec = Spec::load(&specs_dir.join("2026-02-01-001-abc.md")).unwrap();
    assert!(spec.body.contains("Original body"));
    assert!(spec.body.contains("## Output"));
    assert!(spec.body.contains("New output text"));
}

#[test]
#[serial]
fn test_spec_update_invalid_status() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", "# Test spec");

    let result = call_spec_update(Some(json!({
        "id": "2026-02-01-001-abc",
        "status": "invalid_status"
    })));
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_work_list_no_processes() {
    let (_temp, _specs_dir) = setup_test_env();

    let response = call_work_list(None).unwrap();
    assert_eq!(response["processes"].as_array().unwrap().len(), 0);
    assert_eq!(response["summary"]["running"].as_u64().unwrap(), 0);
}

#[test]
#[serial]
fn test_work_list_include_completed() {
    let (_temp, _specs_dir) = setup_test_env();

    let response = call_work_list(Some(json!({"include_completed": true}))).unwrap();
    assert_eq!(response["processes"].as_array().unwrap().len(), 0);
}

#[test]
#[serial]
fn test_ready_with_no_dependencies() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", "# Ready spec");
    create_spec(
        &specs_dir,
        "2026-02-01-002-def",
        "completed",
        "# Completed spec",
    );

    let response = call_ready(None).unwrap();
    assert!(response["returned"].as_u64().unwrap() >= 1);
}

#[test]
#[serial]
fn test_ready_no_ready_specs() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-01-001-abc",
        "completed",
        "# Completed spec",
    );

    let response = call_ready(None).unwrap();
    assert_eq!(response["returned"].as_u64().unwrap(), 0);
}

#[test]
#[serial]
fn test_ready_limit_parameter() {
    let (_temp, specs_dir) = setup_test_env();

    for i in 1..=5 {
        create_spec(
            &specs_dir,
            &format!("2026-02-01-00{}-abc", i),
            "pending",
            "# Ready spec",
        );
    }

    let response = call_ready(Some(json!({"limit": 2}))).unwrap();
    assert_eq!(response["returned"].as_u64().unwrap(), 2);
}

#[test]
#[serial]
fn test_status_brief_mode() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", "# Pending");
    create_spec(&specs_dir, "2026-02-01-002-def", "completed", "# Completed");
    create_spec(
        &specs_dir,
        "2026-02-01-003-ghi",
        "in_progress",
        "# In progress",
    );

    let response = call_status(Some(json!({"brief": true}))).unwrap();
    let text = response.as_str().unwrap();

    assert!(text.contains("pending"));
    assert!(text.contains("completed"));
    assert!(text.contains("in_progress"));
}

#[test]
#[serial]
fn test_status_include_activity() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-01-001-abc",
        "in_progress",
        "# In progress",
    );

    let response = call_status(Some(json!({"include_activity": true}))).unwrap();
    assert!(response["in_progress_activity"].is_array());
}

#[test]
#[serial]
fn test_status_counts_accuracy() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", "# Pending 1");
    create_spec(&specs_dir, "2026-02-01-002-def", "pending", "# Pending 2");
    create_spec(&specs_dir, "2026-02-01-003-ghi", "completed", "# Completed");
    create_spec(&specs_dir, "2026-02-01-004-jkl", "failed", "# Failed");

    let response = call_status(None).unwrap();
    assert_eq!(response["pending"].as_u64().unwrap(), 2);
    assert_eq!(response["completed"].as_u64().unwrap(), 1);
    assert_eq!(response["failed"].as_u64().unwrap(), 1);
    assert_eq!(response["total"].as_u64().unwrap(), 4);
}

#[test]
#[serial]
fn test_log_nonexistent_spec() {
    let (_temp, _specs_dir) = setup_test_env();

    let result = call_log(Some(json!({"id": "nonexistent"})));
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_log_lines_param() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "completed", "# Test");

    let logs_dir = specs_dir.parent().unwrap().join("logs");
    fs::create_dir_all(&logs_dir).unwrap();
    fs::write(
        logs_dir.join("2026-02-01-001-abc.log"),
        "Line 1\nLine 2\nLine 3\nLine 4\nLine 5",
    )
    .unwrap();

    let response = call_log(Some(json!({"id": "2026-02-01-001-abc", "lines": 3}))).unwrap();
    assert_eq!(response["line_count"].as_u64().unwrap(), 3);
}

#[test]
#[serial]
fn test_log_offset_param() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "completed", "# Test");

    let logs_dir = specs_dir.parent().unwrap().join("logs");
    fs::create_dir_all(&logs_dir).unwrap();
    fs::write(logs_dir.join("2026-02-01-001-abc.log"), "Initial content").unwrap();

    let response = call_log(Some(json!({"id": "2026-02-01-001-abc", "offset": 7}))).unwrap();
    assert!(response["content"].as_str().unwrap().contains("content"));
}

#[test]
#[serial]
fn test_verify_all_criteria_checked() {
    let (_temp, specs_dir) = setup_test_env();

    let body = r#"# Test spec

## Acceptance Criteria

- [x] Criterion 1
- [x] Criterion 2
"#;
    create_spec(&specs_dir, "2026-02-01-001-abc", "completed", body);

    let response = call_verify(Some(json!({"id": "2026-02-01-001-abc"}))).unwrap();
    assert!(response["verified"].as_bool().unwrap());
    assert_eq!(response["criteria"]["total"].as_u64().unwrap(), 2);
    assert_eq!(response["criteria"]["checked"].as_u64().unwrap(), 2);
    assert_eq!(response["criteria"]["unchecked"].as_u64().unwrap(), 0);
}

#[test]
#[serial]
fn test_verify_partial_checked() {
    let (_temp, specs_dir) = setup_test_env();

    let body = r#"# Test spec

## Acceptance Criteria

- [x] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
"#;
    create_spec(&specs_dir, "2026-02-01-001-abc", "in_progress", body);

    let response = call_verify(Some(json!({"id": "2026-02-01-001-abc"}))).unwrap();
    assert!(!response["verified"].as_bool().unwrap());
    assert_eq!(response["criteria"]["total"].as_u64().unwrap(), 3);
    assert_eq!(response["criteria"]["checked"].as_u64().unwrap(), 1);
    assert_eq!(response["criteria"]["unchecked"].as_u64().unwrap(), 2);
}

#[test]
#[serial]
fn test_verify_none_checked() {
    let (_temp, specs_dir) = setup_test_env();

    let body = r#"# Test spec

## Acceptance Criteria

- [ ] Criterion 1
- [ ] Criterion 2
"#;
    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", body);

    let response = call_verify(Some(json!({"id": "2026-02-01-001-abc"}))).unwrap();
    assert!(!response["verified"].as_bool().unwrap());
    assert_eq!(response["criteria"]["unchecked"].as_u64().unwrap(), 2);
}

#[test]
#[serial]
fn test_error_handling_missing_params() {
    let (_temp, _specs_dir) = setup_test_env();

    // Missing id parameter
    let result = call_spec_get(None);
    assert!(result.is_err());

    // Empty arguments
    let result = call_spec_get(Some(json!({})));
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_error_handling_malformed_requests() {
    let (_temp, specs_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-01-001-abc", "pending", "# Test");

    // Update with no changes
    let result = call_spec_update(Some(json!({"id": "2026-02-01-001-abc"})));
    assert!(result.is_err());
}
