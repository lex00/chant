//! Validation and linting functionality for specs
//!
//! Provides validation helpers for spec complexity, coupling, approval,
//! output schema, model usage, and type-specific validation. Also provides
//! the `cmd_lint` command function.

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use chant::config::Config;
use chant::paths::LOGS_DIR;
use chant::spec::{self, ApprovalStatus, Spec, SpecStatus};
use chant::validation;

use std::path::PathBuf;

// ============================================================================
// VALIDATION THRESHOLDS
// ============================================================================

/// Thresholds for spec complexity warnings
const COMPLEXITY_THRESHOLD_CRITERIA: usize = 5;
const COMPLEXITY_THRESHOLD_FILES: usize = 5;
const COMPLEXITY_THRESHOLD_WORDS: usize = 500;

/// Regex pattern for spec IDs: YYYY-MM-DD-XXX-abc with optional .N suffix
const SPEC_ID_PATTERN: &str = r"\b\d{4}-\d{2}-\d{2}-[0-9a-z]{3}-[0-9a-z]{3}(?:\.\d+)?\b";

/// Thresholds for "simple" spec detection (model waste)
const SIMPLE_THRESHOLD_CRITERIA: usize = 3;
const SIMPLE_THRESHOLD_FILES: usize = 2;
const SIMPLE_THRESHOLD_WORDS: usize = 200;

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

/// Validate spec complexity and return warnings.
/// Detects specs that may be too complex for haiku execution.
pub fn validate_spec_complexity(spec: &Spec) -> Vec<String> {
    let mut warnings = Vec::new();

    // Count total acceptance criteria
    let criteria_count = spec.count_total_checkboxes();
    if criteria_count > COMPLEXITY_THRESHOLD_CRITERIA {
        warnings.push(format!(
            "Spec has {} acceptance criteria (>{}) - consider splitting for haiku",
            criteria_count, COMPLEXITY_THRESHOLD_CRITERIA
        ));
    }

    // Count target files
    if let Some(files) = &spec.frontmatter.target_files {
        if files.len() > COMPLEXITY_THRESHOLD_FILES {
            warnings.push(format!(
                "Spec touches {} files (>{}) - consider splitting",
                files.len(),
                COMPLEXITY_THRESHOLD_FILES
            ));
        }
    }

    // Count words in body
    let word_count = spec.body.split_whitespace().count();
    if word_count > COMPLEXITY_THRESHOLD_WORDS {
        warnings.push(format!(
            "Spec description is {} words (>{}) - may be too complex for haiku",
            word_count, COMPLEXITY_THRESHOLD_WORDS
        ));
    }

    warnings
}

/// Validate spec for coupling - detect references to other spec IDs in body text.
/// Specs should be self-contained; use depends_on for explicit dependencies.
///
/// Rules:
/// - Drivers (type: driver/group): excluded from coupling check entirely
/// - Member specs (.1, .2, etc): warned only for sibling references (same driver, different member)
/// - Regular specs: warned for any spec ID reference
pub fn validate_spec_coupling(spec: &Spec) -> Vec<String> {
    use regex::Regex;

    let mut warnings = Vec::new();

    // Drivers are allowed to reference their members - skip check entirely
    if spec.frontmatter.r#type == "driver" || spec.frontmatter.r#type == "group" {
        return warnings;
    }

    // Build regex for spec ID pattern
    let re = match Regex::new(SPEC_ID_PATTERN) {
        Ok(r) => r,
        Err(_) => return warnings,
    };

    // Remove code blocks from body before searching
    let body_without_code = remove_code_blocks(&spec.body);

    // Find all spec IDs in the body (excluding code blocks)
    let mut referenced_ids: Vec<String> = re
        .find_iter(&body_without_code)
        .map(|m| m.as_str().to_string())
        .filter(|id| {
            // Exclude the spec's own ID
            !id.starts_with(&spec.id) && !spec.id.starts_with(id)
        })
        .collect();

    // Deduplicate
    referenced_ids.sort();
    referenced_ids.dedup();

    // Check if this is a member spec
    if let Some(driver_id) = spec::extract_driver_id(&spec.id) {
        // This is a member spec - only warn for sibling references
        let sibling_refs: Vec<String> = referenced_ids
            .into_iter()
            .filter(|ref_id| {
                // Check if referenced spec is a sibling (same driver, different member)
                if let Some(ref_driver_id) = spec::extract_driver_id(ref_id) {
                    if ref_driver_id == driver_id {
                        // Same driver, so it's a sibling
                        return true;
                    }
                }
                false
            })
            .collect();

        if !sibling_refs.is_empty() {
            let ids_str = sibling_refs.join(", ");
            warnings.push(format!(
                "Spec references sibling spec(s): {} - member specs should be independent",
                ids_str
            ));
        }
    } else {
        // Regular spec - warn on any spec ID reference
        if !referenced_ids.is_empty() {
            let ids_str = referenced_ids.join(", ");
            warnings.push(format!(
                "Spec references other spec ID(s) in body: {} - use depends_on for dependencies",
                ids_str
            ));
        }
    }

    warnings
}

/// Remove code blocks from text (content between ``` markers)
fn remove_code_blocks(text: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if !in_code_block {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Validate approval schema - check for consistency in approval fields.
pub fn validate_approval_schema(spec: &Spec) -> Vec<String> {
    let mut warnings = Vec::new();

    if let Some(ref approval) = spec.frontmatter.approval {
        // If approved or rejected, should have 'by' and 'at' fields
        if approval.status == ApprovalStatus::Approved
            || approval.status == ApprovalStatus::Rejected
        {
            if approval.by.is_none() {
                warnings.push(format!(
                    "Approval status is {:?} but 'by' field is missing",
                    approval.status
                ));
            }
            if approval.at.is_none() {
                warnings.push(format!(
                    "Approval status is {:?} but 'at' timestamp is missing",
                    approval.status
                ));
            }
        }

        // If 'by' is set but status is still pending, that's inconsistent
        if approval.status == ApprovalStatus::Pending && approval.by.is_some() {
            warnings.push("Approval has 'by' field set but status is still 'pending'".to_string());
        }
    }

    warnings
}

/// Validate output schema for completed specs.
/// If a spec has output_schema defined and is completed, check that the agent log
/// contains valid JSON matching the schema.
pub fn validate_output_schema(spec: &Spec) -> Vec<String> {
    let mut warnings = Vec::new();

    // Only validate completed specs with output_schema defined
    if spec.frontmatter.status != SpecStatus::Completed {
        return warnings;
    }

    let schema_path_str = match &spec.frontmatter.output_schema {
        Some(path) => path,
        None => return warnings,
    };

    let schema_path = Path::new(schema_path_str);

    // Check if schema file exists
    if !schema_path.exists() {
        warnings.push(format!("Output schema file not found: {}", schema_path_str));
        return warnings;
    }

    // Check if log file exists
    let logs_dir = PathBuf::from(LOGS_DIR);
    match validation::validate_spec_output_from_log(&spec.id, schema_path, &logs_dir) {
        Ok(Some(result)) => {
            if !result.is_valid {
                warnings.push(format!(
                    "Output validation failed: {}",
                    result.errors.join("; ")
                ));
            }
        }
        Ok(None) => {
            // No log file - this is expected for specs not yet executed
            // Don't warn since completion may have been set manually or from archive
        }
        Err(e) => {
            warnings.push(format!("Failed to validate output: {}", e));
        }
    }

    warnings
}

/// Validate model usage - warn when expensive models are used on simple specs.
/// Haiku should be used for straightforward specs; opus/sonnet for complex work.
pub fn validate_model_waste(spec: &Spec) -> Vec<String> {
    let mut warnings = Vec::new();

    // Only check if model is explicitly set to opus or sonnet
    let model = match &spec.frontmatter.model {
        Some(m) => m.to_lowercase(),
        None => return warnings,
    };

    let is_expensive = model.contains("opus") || model.contains("sonnet");
    if !is_expensive {
        return warnings;
    }

    // Don't warn on driver/research specs - they benefit from smarter models
    let spec_type = spec.frontmatter.r#type.as_str();
    if spec_type == "driver" || spec_type == "group" || spec_type == "research" {
        return warnings;
    }

    // Check if spec looks simple
    let criteria_count = spec.count_total_checkboxes();
    let file_count = spec
        .frontmatter
        .target_files
        .as_ref()
        .map(|f| f.len())
        .unwrap_or(0);
    let word_count = spec.body.split_whitespace().count();

    let is_simple = criteria_count <= SIMPLE_THRESHOLD_CRITERIA
        && file_count <= SIMPLE_THRESHOLD_FILES
        && word_count <= SIMPLE_THRESHOLD_WORDS;

    if is_simple {
        warnings.push(format!(
            "Spec uses '{}' but appears simple ({} criteria, {} files, {} words) - consider haiku",
            spec.frontmatter.model.as_ref().unwrap(),
            criteria_count,
            file_count,
            word_count
        ));
    }

    warnings
}

/// Validate a spec based on its type and return warnings.
/// Returns a vector of warning messages for type-specific validation issues.
pub fn validate_spec_type(spec: &Spec) -> Vec<String> {
    let mut warnings = Vec::new();

    match spec.frontmatter.r#type.as_str() {
        "documentation" => {
            if spec.frontmatter.tracks.is_none() {
                warnings.push("Documentation spec missing 'tracks' field".to_string());
            }
            if spec.frontmatter.target_files.is_none() {
                warnings.push("Documentation spec missing 'target_files' field".to_string());
            }
        }
        "research" => {
            if spec.frontmatter.informed_by.is_none() && spec.frontmatter.origin.is_none() {
                warnings.push(
                    "Research spec missing both 'informed_by' and 'origin' fields".to_string(),
                );
            }
            if spec.frontmatter.target_files.is_none() {
                warnings.push("Research spec missing 'target_files' field".to_string());
            }
        }
        "driver" | "group" => {
            // Validate members field if present
            if let Some(ref members) = spec.frontmatter.members {
                if members.is_empty() {
                    warnings.push("Driver/group spec has empty 'members' array".to_string());
                }
            }
        }
        _ => {}
    }

    warnings
}

// ============================================================================
// LINT COMMAND FUNCTIONS
// ============================================================================

/// Internal result type for linting operations
pub struct LintResult {
    pub passed: usize,
    pub warned: usize,
    pub failed: usize,
}

/// Lint specific specs (by ID) and return a summary.
/// Useful for linting a subset of specs, like member specs after split.
pub fn lint_specific_specs(specs_dir: &std::path::Path, spec_ids: &[String]) -> Result<LintResult> {
    let mut all_spec_ids: Vec<String> = Vec::new();
    let mut specs_to_check: Vec<Spec> = Vec::new();

    // Load all specs to validate dependencies
    for entry in std::fs::read_dir(specs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(spec) = Spec::load(&path) {
                all_spec_ids.push(spec.id.clone());
            }
        }
    }

    // Load only the specs we want to check
    for spec_id in spec_ids {
        let spec_path = specs_dir.join(format!("{}.md", spec_id));
        if spec_path.exists() {
            match Spec::load(&spec_path) {
                Ok(spec) => {
                    specs_to_check.push(spec);
                }
                Err(e) => {
                    eprintln!("{} {}: Invalid YAML frontmatter: {}", "✗".red(), spec_id, e);
                    return Ok(LintResult {
                        passed: 0,
                        warned: 0,
                        failed: 1,
                    });
                }
            }
        }
    }

    let mut passed = 0;
    let mut warned = 0;
    let mut failed = 0;

    // Validate each spec
    for spec in &specs_to_check {
        let mut spec_issues: Vec<String> = Vec::new();

        // Check for title
        if spec.title.is_none() {
            spec_issues.push("Missing title".to_string());
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id) {
                    spec_issues.push(format!("Unknown dependency '{}'", dep_id));
                }
            }
        }

        // Type-specific validation
        let type_warnings = validate_spec_type(spec);

        // Complexity validation
        let complexity_warnings = validate_spec_complexity(spec);

        // Coupling validation (spec references other spec IDs)
        let coupling_warnings = validate_spec_coupling(spec);

        // Model waste validation (expensive model on simple spec)
        let model_warnings = validate_model_waste(spec);

        // Combine all warnings
        let mut spec_warnings = type_warnings;
        spec_warnings.extend(complexity_warnings);
        spec_warnings.extend(coupling_warnings);
        spec_warnings.extend(model_warnings);

        if spec_issues.is_empty() && spec_warnings.is_empty() {
            println!("  {} {}", "✓".green(), spec.id);
            passed += 1;
        } else {
            let has_errors = !spec_issues.is_empty();
            let has_warnings = !spec_warnings.is_empty();

            if has_errors {
                for issue in &spec_issues {
                    println!("  {} {}: {}", "✗".red(), spec.id, issue);
                }
                failed += 1;
            }

            if has_warnings {
                let has_complexity_warning = spec_warnings.iter().any(|w| {
                    w.contains("complexity")
                        || w.contains("criteria")
                        || w.contains("files")
                        || w.contains("words")
                });
                for warning in &spec_warnings {
                    println!("  {} {}: {}", "⚠".yellow(), spec.id, warning);
                }
                // Suggest split if there are complexity warnings
                if has_complexity_warning {
                    println!("      {} Consider: chant split {}", "→".cyan(), spec.id);
                }
                if !has_errors {
                    warned += 1;
                }
            }
        }
    }

    Ok(LintResult {
        passed,
        warned,
        failed,
    })
}

pub fn cmd_lint() -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    println!("Linting specs...");

    let mut issues: Vec<(String, String)> = Vec::new();
    let mut total_specs = 0;

    // Load config to get enterprise required fields
    let config = Config::load().ok();

    // First pass: collect all spec IDs and check for parse errors
    let mut all_spec_ids: Vec<String> = Vec::new();
    let mut specs_to_check: Vec<Spec> = Vec::new();

    for entry in std::fs::read_dir(&specs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            total_specs += 1;
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            match Spec::load(&path) {
                Ok(spec) => {
                    all_spec_ids.push(spec.id.clone());
                    specs_to_check.push(spec);
                }
                Err(e) => {
                    let issue = format!("Invalid YAML frontmatter: {}", e);
                    println!("{} {}: {}", "✗".red(), id, issue);
                    issues.push((id, issue));
                }
            }
        }
    }

    // Second pass: validate each spec
    for spec in &specs_to_check {
        let mut spec_issues: Vec<String> = Vec::new();

        // Check for title
        if spec.title.is_none() {
            spec_issues.push("Missing title".to_string());
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id) {
                    spec_issues.push(format!("Unknown dependency '{}'", dep_id));
                }
            }
        }

        // Check members references
        if let Some(members) = &spec.frontmatter.members {
            for member_id in members {
                if !all_spec_ids.contains(member_id) {
                    spec_issues.push(format!("Unknown member spec '{}'", member_id));
                }
            }
        }

        // Check required fields from enterprise config
        if let Some(ref cfg) = config {
            if !cfg.enterprise.required.is_empty() {
                for required_field in &cfg.enterprise.required {
                    if !spec.has_frontmatter_field(required_field) {
                        spec_issues.push(format!("Missing required field '{}'", required_field));
                    }
                }
            }
        }

        // Type-specific validation
        let type_warnings = validate_spec_type(spec);

        // Complexity validation
        let complexity_warnings = validate_spec_complexity(spec);

        // Coupling validation (spec references other spec IDs)
        let coupling_warnings = validate_spec_coupling(spec);

        // Model waste validation (expensive model on simple spec)
        let model_warnings = validate_model_waste(spec);

        // Approval schema validation
        let approval_warnings = validate_approval_schema(spec);

        // Output schema validation for completed specs
        let output_warnings = validate_output_schema(spec);

        // Combine all warnings
        let mut spec_warnings = type_warnings;
        spec_warnings.extend(complexity_warnings);
        spec_warnings.extend(coupling_warnings);
        spec_warnings.extend(model_warnings);
        spec_warnings.extend(approval_warnings);
        spec_warnings.extend(output_warnings);

        if spec_issues.is_empty() && spec_warnings.is_empty() {
            println!("{} {}", "✓".green(), spec.id);
        } else {
            for issue in spec_issues {
                println!("{} {}: {}", "✗".red(), spec.id, issue);
                issues.push((spec.id.clone(), issue));
            }
            // Check if there are complexity warnings before iterating
            let has_complexity_warning = spec_warnings.iter().any(|w| {
                w.contains("complexity")
                    || w.contains("criteria")
                    || w.contains("files")
                    || w.contains("words")
            });
            for warning in spec_warnings {
                println!("{} {}: {}", "⚠".yellow(), spec.id, warning);
            }
            // Suggest split if there are complexity warnings
            if has_complexity_warning {
                println!("    {} Consider: chant split {}", "→".cyan(), spec.id);
            }
        }
    }

    // Print summary with enterprise policy if configured
    if !issues.is_empty() {
        println!(
            "\nFound {} {} in {} specs.",
            issues.len(),
            if issues.len() == 1 { "issue" } else { "issues" },
            total_specs
        );

        // Show enterprise policy if required fields are configured
        if let Some(cfg) = &config {
            if !cfg.enterprise.required.is_empty() {
                println!(
                    "\n{} Enterprise policy requires: {}",
                    "ℹ".cyan(),
                    cfg.enterprise.required.join(", ")
                );
            }
        }

        std::process::exit(1);
    } else {
        println!("\nAll {} specs valid.", total_specs);
        Ok(())
    }
}
