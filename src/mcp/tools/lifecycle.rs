//! MCP tools for spec lifecycle transitions

use anyhow::Result;
use serde_json::Value;

use crate::operations;
use crate::spec::{load_all_specs, resolve_spec, SpecStatus};

use super::super::handlers::mcp_ensure_initialized;
use super::super::response::{mcp_error_response, mcp_text_response};

pub fn tool_chant_finalize(arguments: Option<&Value>) -> Result<Value> {
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

    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(mcp_error_response(e.to_string()));
        }
    };

    let spec_id = spec.id.clone();

    // Check if spec is in valid state for finalization
    match spec.frontmatter.status {
        SpecStatus::Completed | SpecStatus::InProgress | SpecStatus::Failed => {
            // Valid for finalization
        }
        _ => {
            return Ok(mcp_error_response(format!("Spec '{}' must be in_progress, completed, or failed to finalize. Current status: {:?}", spec_id, spec.frontmatter.status)));
        }
    }

    // Check for unchecked acceptance criteria
    let unchecked = spec.count_unchecked_checkboxes();
    if unchecked > 0 {
        return Ok(mcp_error_response(format!("Spec '{}' has {} unchecked acceptance criteria. All criteria must be checked before finalization.", spec_id, unchecked)));
    }

    // Load config and all specs for finalization
    let config = match crate::config::Config::load() {
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

    // Use operations module for finalization with full validation
    let spec_repo = crate::repository::spec_repository::FileSpecRepository::new(specs_dir.clone());
    let options = crate::operations::finalize::FinalizeOptions {
        allow_no_commits: false,
        commits: None, // Auto-detect commits
        force,
    };

    match crate::operations::finalize::finalize_spec(
        &mut spec, &spec_repo, &config, &all_specs, options,
    ) {
        Ok(_) => Ok(mcp_text_response(format!("Finalized spec: {}", spec_id))),
        Err(e) => Ok(mcp_error_response(format!(
            "Failed to finalize spec: {}",
            e
        ))),
    }
}

pub fn tool_chant_reset(arguments: Option<&Value>) -> Result<Value> {
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
            return Ok(mcp_error_response(e.to_string()));
        }
    };

    let spec_id = spec.id.clone();
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // Use operations module for reset
    let options = crate::operations::reset::ResetOptions::default();

    match crate::operations::reset::reset_spec(&mut spec, &spec_path, options) {
        Ok(_) => Ok(mcp_text_response(format!(
            "Reset spec '{}' to pending",
            spec_id
        ))),
        Err(e) => Ok(mcp_error_response(format!("Failed to reset spec: {}", e))),
    }
}

pub fn tool_chant_cancel(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let options = operations::CancelOptions::default();

    match operations::cancel_spec(&specs_dir, id, &options) {
        Ok(spec) => Ok(mcp_text_response(format!("Cancelled spec: {}", spec.id))),
        Err(e) => Ok(mcp_error_response(e.to_string())),
    }
}

pub fn tool_chant_archive(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = match mcp_ensure_initialized() {
        Ok(dir) => dir,
        Err(err_response) => return Ok(err_response),
    };

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let options = operations::ArchiveOptions::default();

    match operations::archive_spec(&specs_dir, id, &options) {
        Ok(dest_path) => Ok(mcp_text_response(format!(
            "Archived spec: {} -> {}",
            id,
            dest_path.display()
        ))),
        Err(e) => Ok(mcp_error_response(e.to_string())),
    }
}
