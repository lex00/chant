//! Validation and linting functionality for specs
//!
//! Provides validation helpers for spec complexity, coupling, approval,
//! output schema, model usage, and type-specific validation. Also provides
//! the `cmd_lint` command function.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use serde_json;
use std::path::Path;

use chant::config::Config;
use chant::paths::LOGS_DIR;
use chant::spec::{self, ApprovalStatus, Spec, SpecStatus};
use chant::validation;

use std::path::PathBuf;

// ============================================================================
// LINT TYPES
// ============================================================================

/// Output format for lint results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintFormat {
    /// Human-readable text output with colors
    Text,
    /// Machine-readable JSON output
    Json,
}

/// Categories of lint rules for spec validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LintRule {
    /// Spec is too complex (too many criteria, files, or words)
    Complexity,
    /// Spec references other spec IDs in body text (coupling)
    Coupling,
    /// Type-specific validation issues
    Type,
    /// Using expensive model on simple spec
    ModelWaste,
    /// Approval schema inconsistencies
    Approval,
    /// Output schema validation issues
    Output,
    /// Missing or invalid dependency references
    Dependency,
    /// Missing required enterprise fields
    Required,
    /// Missing spec title
    Title,
    /// YAML frontmatter parse errors
    Parse,
}

impl LintRule {
    /// Returns the string representation of the lint rule
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            LintRule::Complexity => "complexity",
            LintRule::Coupling => "coupling",
            LintRule::Type => "type",
            LintRule::ModelWaste => "model_waste",
            LintRule::Approval => "approval",
            LintRule::Output => "output",
            LintRule::Dependency => "dependency",
            LintRule::Required => "required",
            LintRule::Title => "title",
            LintRule::Parse => "parse",
        }
    }
}

/// Severity level for lint diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Error - spec is invalid and must be fixed
    Error,
    /// Warning - spec is valid but could be improved
    Warning,
}

/// A single diagnostic message from linting
#[derive(Debug, Clone, Serialize)]
pub struct LintDiagnostic {
    /// The spec ID this diagnostic applies to
    #[allow(dead_code)]
    pub spec_id: String,
    /// The lint rule that triggered this diagnostic
    pub rule: LintRule,
    /// The severity level
    pub severity: Severity,
    /// The diagnostic message
    pub message: String,
    /// Optional suggestion for fixing the issue
    pub suggestion: Option<String>,
}

impl LintDiagnostic {
    /// Create a new error diagnostic
    pub fn error(spec_id: String, rule: LintRule, message: String) -> Self {
        Self {
            spec_id,
            rule,
            severity: Severity::Error,
            message,
            suggestion: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(spec_id: String, rule: LintRule, message: String) -> Self {
        Self {
            spec_id,
            rule,
            severity: Severity::Warning,
            message,
            suggestion: None,
        }
    }

    /// Add a suggestion to this diagnostic
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// Complete report from linting operation
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct LintReport {
    /// All diagnostics found during linting
    pub diagnostics: Vec<LintDiagnostic>,
    /// Number of specs that passed all checks
    pub passed: usize,
    /// Number of specs with warnings only
    pub warned: usize,
    /// Number of specs with errors
    pub failed: usize,
    /// Total number of specs checked
    pub total: usize,
}

impl LintReport {
    /// Check if the report has any errors
    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Check if the report has any warnings
    #[allow(dead_code)]
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning)
    }
}

// ============================================================================
// VALIDATION THRESHOLDS
// ============================================================================

/// Regex pattern for spec IDs: YYYY-MM-DD-XXX-abc with optional .N suffix
const SPEC_ID_PATTERN: &str = r"\b\d{4}-\d{2}-\d{2}-[0-9a-z]{3}-[0-9a-z]{3}(?:\.\d+)?\b";

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

/// Validate spec complexity and return diagnostics.
/// Detects specs that may be too complex for haiku execution.
pub fn validate_spec_complexity(
    spec: &Spec,
    thresholds: &chant::config::LintThresholds,
) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    // Count total acceptance criteria
    let criteria_count = spec.count_total_checkboxes();
    if criteria_count > thresholds.complexity_criteria {
        diagnostics.push(
            LintDiagnostic::warning(
                spec.id.clone(),
                LintRule::Complexity,
                format!(
                    "Spec has {} acceptance criteria (>{}) - consider splitting for haiku",
                    criteria_count, thresholds.complexity_criteria
                ),
            )
            .with_suggestion(format!("Consider using 'chant split {}'", spec.id)),
        );
    }

    // Count target files
    if let Some(files) = &spec.frontmatter.target_files {
        if files.len() > thresholds.complexity_files {
            diagnostics.push(
                LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Complexity,
                    format!(
                        "Spec touches {} files (>{}) - consider splitting",
                        files.len(),
                        thresholds.complexity_files
                    ),
                )
                .with_suggestion(format!("Consider using 'chant split {}'", spec.id)),
            );
        }
    }

    // Count words in body
    let word_count = spec.body.split_whitespace().count();
    if word_count > thresholds.complexity_words {
        diagnostics.push(
            LintDiagnostic::warning(
                spec.id.clone(),
                LintRule::Complexity,
                format!(
                    "Spec description is {} words (>{}) - may be too complex for haiku",
                    word_count, thresholds.complexity_words
                ),
            )
            .with_suggestion(format!("Consider using 'chant split {}'", spec.id)),
        );
    }

    diagnostics
}

/// Validate spec for coupling - detect references to other spec IDs in body text.
/// Specs should be self-contained; use depends_on for explicit dependencies.
///
/// Rules:
/// - Drivers (type: driver/group): excluded from coupling check entirely
/// - Member specs (.1, .2, etc): warned only for sibling references (same driver, different member)
/// - Regular specs: warned for any spec ID reference
pub fn validate_spec_coupling(spec: &Spec) -> Vec<LintDiagnostic> {
    use regex::Regex;

    let mut diagnostics = Vec::new();

    // Drivers are allowed to reference their members - skip check entirely
    if spec.frontmatter.r#type == "driver" || spec.frontmatter.r#type == "group" {
        return diagnostics;
    }

    // Build regex for spec ID pattern
    let re = match Regex::new(SPEC_ID_PATTERN) {
        Ok(r) => r,
        Err(_) => return diagnostics,
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
            diagnostics.push(
                LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Coupling,
                    format!(
                        "Spec references sibling spec(s): {} - member specs should be independent",
                        ids_str
                    ),
                )
                .with_suggestion(
                    "Use depends_on for dependencies instead of referencing spec IDs in the body"
                        .to_string(),
                ),
            );
        }
    } else {
        // Regular spec - warn on any spec ID reference
        if !referenced_ids.is_empty() {
            let ids_str = referenced_ids.join(", ");
            diagnostics.push(
                LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Coupling,
                    format!(
                        "Spec references other spec ID(s) in body: {} - use depends_on for dependencies",
                        ids_str
                    ),
                )
                .with_suggestion("Use depends_on for dependencies instead of referencing spec IDs in the body".to_string()),
            );
        }
    }

    diagnostics
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
pub fn validate_approval_schema(spec: &Spec) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(ref approval) = spec.frontmatter.approval {
        // If approved or rejected, should have 'by' and 'at' fields
        if approval.status == ApprovalStatus::Approved
            || approval.status == ApprovalStatus::Rejected
        {
            if approval.by.is_none() {
                diagnostics.push(LintDiagnostic::error(
                    spec.id.clone(),
                    LintRule::Approval,
                    format!(
                        "Approval status is {:?} but 'by' field is missing",
                        approval.status
                    ),
                ));
            }
            if approval.at.is_none() {
                diagnostics.push(LintDiagnostic::error(
                    spec.id.clone(),
                    LintRule::Approval,
                    format!(
                        "Approval status is {:?} but 'at' timestamp is missing",
                        approval.status
                    ),
                ));
            }
        }

        // If 'by' is set but status is still pending, that's inconsistent
        if approval.status == ApprovalStatus::Pending && approval.by.is_some() {
            diagnostics.push(LintDiagnostic::error(
                spec.id.clone(),
                LintRule::Approval,
                "Approval has 'by' field set but status is still 'pending'".to_string(),
            ));
        }
    }

    diagnostics
}

/// Validate output schema for completed specs.
/// If a spec has output_schema defined and is completed, check that the agent log
/// contains valid JSON matching the schema.
pub fn validate_output_schema(spec: &Spec) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    // Only validate completed specs with output_schema defined
    if spec.frontmatter.status != SpecStatus::Completed {
        return diagnostics;
    }

    let schema_path_str = match &spec.frontmatter.output_schema {
        Some(path) => path,
        None => return diagnostics,
    };

    let schema_path = Path::new(schema_path_str);

    // Check if schema file exists
    if !schema_path.exists() {
        diagnostics.push(LintDiagnostic::error(
            spec.id.clone(),
            LintRule::Output,
            format!("Output schema file not found: {}", schema_path_str),
        ));
        return diagnostics;
    }

    // Check if log file exists
    let logs_dir = PathBuf::from(LOGS_DIR);
    match validation::validate_spec_output_from_log(&spec.id, schema_path, &logs_dir) {
        Ok(Some(result)) => {
            if !result.is_valid {
                diagnostics.push(LintDiagnostic::error(
                    spec.id.clone(),
                    LintRule::Output,
                    format!("Output validation failed: {}", result.errors.join("; ")),
                ));
            }
        }
        Ok(None) => {
            // No log file - this is expected for specs not yet executed
            // Don't warn since completion may have been set manually or from archive
        }
        Err(e) => {
            diagnostics.push(LintDiagnostic::error(
                spec.id.clone(),
                LintRule::Output,
                format!("Failed to validate output: {}", e),
            ));
        }
    }

    diagnostics
}

/// Validate model usage - warn when expensive models are used on simple specs.
/// Haiku should be used for straightforward specs; opus/sonnet for complex work.
pub fn validate_model_waste(
    spec: &Spec,
    thresholds: &chant::config::LintThresholds,
) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    // Only check if model is explicitly set to opus or sonnet
    let model = match &spec.frontmatter.model {
        Some(m) => m.to_lowercase(),
        None => return diagnostics,
    };

    let is_expensive = model.contains("opus") || model.contains("sonnet");
    if !is_expensive {
        return diagnostics;
    }

    // Don't warn on driver/research specs - they benefit from smarter models
    let spec_type = spec.frontmatter.r#type.as_str();
    if spec_type == "driver" || spec_type == "group" || spec_type == "research" {
        return diagnostics;
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

    let is_simple = criteria_count <= thresholds.simple_criteria
        && file_count <= thresholds.simple_files
        && word_count <= thresholds.simple_words;

    if is_simple {
        diagnostics.push(
            LintDiagnostic::warning(
                spec.id.clone(),
                LintRule::ModelWaste,
                format!(
                    "Spec uses '{}' but appears simple ({} criteria, {} files, {} words) - consider haiku",
                    spec.frontmatter.model.as_ref().unwrap(),
                    criteria_count,
                    file_count,
                    word_count
                ),
            )
            .with_suggestion("Consider using haiku model for simple specs to reduce cost".to_string()),
        );
    }

    diagnostics
}

/// Validate a spec based on its type and return diagnostics.
/// Returns a vector of diagnostics for type-specific validation issues.
pub fn validate_spec_type(spec: &Spec) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    match spec.frontmatter.r#type.as_str() {
        "documentation" => {
            if spec.frontmatter.tracks.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Type,
                    "Documentation spec missing 'tracks' field".to_string(),
                ));
            }
            if spec.frontmatter.target_files.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Type,
                    "Documentation spec missing 'target_files' field".to_string(),
                ));
            }
        }
        "research" => {
            if spec.frontmatter.informed_by.is_none() && spec.frontmatter.origin.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Type,
                    "Research spec missing both 'informed_by' and 'origin' fields".to_string(),
                ));
            }
            if spec.frontmatter.target_files.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    spec.id.clone(),
                    LintRule::Type,
                    "Research spec missing 'target_files' field".to_string(),
                ));
            }
        }
        "driver" | "group" => {
            // Validate members field if present
            if let Some(ref members) = spec.frontmatter.members {
                if members.is_empty() {
                    diagnostics.push(LintDiagnostic::warning(
                        spec.id.clone(),
                        LintRule::Type,
                        "Driver/group spec has empty 'members' array".to_string(),
                    ));
                }
            }
        }
        _ => {}
    }

    diagnostics
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

    // Load config to get lint thresholds
    let config = Config::load().ok();
    let default_thresholds = chant::config::LintThresholds::default();
    let thresholds = config
        .as_ref()
        .map(|c| &c.lint.thresholds)
        .unwrap_or(&default_thresholds);

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
        let mut spec_diagnostics: Vec<LintDiagnostic> = Vec::new();

        // Check for title
        if spec.title.is_none() {
            spec_diagnostics.push(LintDiagnostic::error(
                spec.id.clone(),
                LintRule::Title,
                "Missing title".to_string(),
            ));
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id) {
                    spec_diagnostics.push(LintDiagnostic::error(
                        spec.id.clone(),
                        LintRule::Dependency,
                        format!("Unknown dependency '{}'", dep_id),
                    ));
                }
            }
        }

        // Type-specific validation
        spec_diagnostics.extend(validate_spec_type(spec));

        // Complexity validation
        spec_diagnostics.extend(validate_spec_complexity(spec, thresholds));

        // Coupling validation (spec references other spec IDs)
        spec_diagnostics.extend(validate_spec_coupling(spec));

        // Model waste validation (expensive model on simple spec)
        spec_diagnostics.extend(validate_model_waste(spec, thresholds));

        // Separate errors and warnings
        let errors: Vec<_> = spec_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = spec_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();

        if spec_diagnostics.is_empty() {
            println!("  {} {}", "✓".green(), spec.id);
            passed += 1;
        } else {
            let has_errors = !errors.is_empty();
            let has_warnings = !warnings.is_empty();

            if has_errors {
                for diagnostic in &errors {
                    println!("  {} {}: {}", "✗".red(), spec.id, diagnostic.message);
                }
                failed += 1;
            }

            if has_warnings {
                let has_complexity_warning =
                    warnings.iter().any(|d| d.rule == LintRule::Complexity);
                for diagnostic in &warnings {
                    println!("  {} {}: {}", "⚠".yellow(), spec.id, diagnostic.message);
                    if let Some(ref suggestion) = diagnostic.suggestion {
                        println!("      {} {}", "→".cyan(), suggestion);
                    }
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

pub fn cmd_lint(format: LintFormat) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    if format == LintFormat::Text {
        println!("Linting specs...");
    }

    let mut all_diagnostics: Vec<LintDiagnostic> = Vec::new();
    let mut total_specs = 0;

    // Load config to get enterprise required fields and lint thresholds
    let config = Config::load().ok();
    let default_thresholds = chant::config::LintThresholds::default();
    let thresholds = config
        .as_ref()
        .map(|c| &c.lint.thresholds)
        .unwrap_or(&default_thresholds);

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
                    let diagnostic = LintDiagnostic::error(
                        id.clone(),
                        LintRule::Parse,
                        format!("Invalid YAML frontmatter: {}", e),
                    );
                    if format == LintFormat::Text {
                        println!("{} {}: {}", "✗".red(), id, diagnostic.message);
                    }
                    all_diagnostics.push(diagnostic);
                }
            }
        }
    }

    // Second pass: validate each spec
    for spec in &specs_to_check {
        let mut spec_diagnostics: Vec<LintDiagnostic> = Vec::new();

        // Check for title
        if spec.title.is_none() {
            spec_diagnostics.push(LintDiagnostic::error(
                spec.id.clone(),
                LintRule::Title,
                "Missing title".to_string(),
            ));
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id) {
                    spec_diagnostics.push(LintDiagnostic::error(
                        spec.id.clone(),
                        LintRule::Dependency,
                        format!("Unknown dependency '{}'", dep_id),
                    ));
                }
            }
        }

        // Check members references
        if let Some(members) = &spec.frontmatter.members {
            for member_id in members {
                if !all_spec_ids.contains(member_id) {
                    spec_diagnostics.push(LintDiagnostic::error(
                        spec.id.clone(),
                        LintRule::Dependency,
                        format!("Unknown member spec '{}'", member_id),
                    ));
                }
            }
        }

        // Check required fields from enterprise config
        if let Some(ref cfg) = config {
            if !cfg.enterprise.required.is_empty() {
                for required_field in &cfg.enterprise.required {
                    if !spec.has_frontmatter_field(required_field) {
                        spec_diagnostics.push(LintDiagnostic::error(
                            spec.id.clone(),
                            LintRule::Required,
                            format!("Missing required field '{}'", required_field),
                        ));
                    }
                }
            }
        }

        // Type-specific validation
        spec_diagnostics.extend(validate_spec_type(spec));

        // Complexity validation
        spec_diagnostics.extend(validate_spec_complexity(spec, thresholds));

        // Coupling validation (spec references other spec IDs)
        spec_diagnostics.extend(validate_spec_coupling(spec));

        // Model waste validation (expensive model on simple spec)
        spec_diagnostics.extend(validate_model_waste(spec, thresholds));

        // Approval schema validation
        spec_diagnostics.extend(validate_approval_schema(spec));

        // Output schema validation for completed specs
        spec_diagnostics.extend(validate_output_schema(spec));

        // Separate errors and warnings
        let errors: Vec<_> = spec_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = spec_diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();

        if format == LintFormat::Text {
            if spec_diagnostics.is_empty() {
                println!("{} {}", "✓".green(), spec.id);
            } else {
                for diagnostic in &errors {
                    println!("{} {}: {}", "✗".red(), spec.id, diagnostic.message);
                }
                // Check if there are complexity warnings
                let has_complexity_warning =
                    warnings.iter().any(|d| d.rule == LintRule::Complexity);
                for diagnostic in &warnings {
                    println!("{} {}: {}", "⚠".yellow(), spec.id, diagnostic.message);
                    if let Some(ref suggestion) = diagnostic.suggestion {
                        println!("    {} {}", "→".cyan(), suggestion);
                    }
                }
                // Suggest split if there are complexity warnings
                if has_complexity_warning {
                    println!("    {} Consider: chant split {}", "→".cyan(), spec.id);
                }
            }
        }

        all_diagnostics.extend(spec_diagnostics);
    }

    // Count errors
    let error_count = all_diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();

    // Output results based on format
    match format {
        LintFormat::Text => {
            // Print summary with enterprise policy if configured
            if error_count > 0 {
                println!(
                    "\nFound {} {} in {} specs.",
                    error_count,
                    if error_count == 1 { "error" } else { "errors" },
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
        LintFormat::Json => {
            // Count specs by diagnostic status
            let mut spec_errors = std::collections::HashSet::new();
            let mut spec_warnings = std::collections::HashSet::new();

            for diag in &all_diagnostics {
                if diag.severity == Severity::Error {
                    spec_errors.insert(diag.spec_id.clone());
                } else {
                    spec_warnings.insert(diag.spec_id.clone());
                }
            }

            let failed = spec_errors.len();
            let warned = spec_warnings.difference(&spec_errors).count();
            let passed = total_specs - failed - warned;

            let report = LintReport {
                diagnostics: all_diagnostics,
                passed,
                warned,
                failed,
                total: total_specs,
            };

            let json = serde_json::to_string_pretty(&report)?;
            println!("{}", json);

            if error_count > 0 {
                std::process::exit(1);
            } else {
                Ok(())
            }
        }
    }
}
