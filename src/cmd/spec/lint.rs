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
use chant::score::ac_quality::calculate_ac_quality;
use chant::score::confidence::calculate_confidence;
use chant::score::isolation::calculate_isolation;
use chant::score::splittability::calculate_splittability;
use chant::score::traffic_light::{determine_status, generate_suggestions};
use chant::scoring::{calculate_complexity, SpecScore};
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
    pub fn error(spec_id: &str, rule: LintRule, message: String) -> Self {
        Self {
            spec_id: spec_id.to_string(),
            rule,
            severity: Severity::Error,
            message,
            suggestion: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(spec_id: &str, rule: LintRule, message: String) -> Self {
        Self {
            spec_id: spec_id.to_string(),
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
// SCORING HELPERS
// ============================================================================

/// Extract acceptance criteria text from spec body
fn extract_acceptance_criteria(spec: &Spec) -> Vec<String> {
    let acceptance_criteria_marker = "## Acceptance Criteria";
    let mut criteria = Vec::new();
    let mut in_code_fence = false;
    let mut in_ac_section = false;

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

        // Stop if we hit another ## heading
        if in_ac_section && !in_code_fence && trimmed.starts_with("## ") {
            break;
        }

        // Extract checkbox items
        if in_ac_section
            && !in_code_fence
            && (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]"))
        {
            // Extract text after checkbox
            let text = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim()
                .to_string();
            if !text.is_empty() {
                criteria.push(text);
            }
        }
    }

    criteria
}

/// Calculate complete quality score for a spec
fn calculate_spec_score(spec: &Spec, all_specs: &[Spec]) -> SpecScore {
    let complexity = calculate_complexity(spec);

    // Load config for confidence calculation (creates minimal default if load fails)
    let loaded_config = Config::load().ok();
    let minimal_config = if loaded_config.is_none() {
        // Create a minimal config on the fly if load fails
        let toml_str = r#"
[project]
name = "default"
prefix = ""
"#;
        Config::parse(toml_str).ok()
    } else {
        None
    };
    let conf = loaded_config.as_ref().or(minimal_config.as_ref());

    let confidence = if let Some(c) = conf {
        calculate_confidence(spec, c)
    } else {
        // Fallback if config creation fails - use basic confidence calculation
        chant::scoring::ConfidenceGrade::B
    };
    let splittability = calculate_splittability(spec);
    let isolation = calculate_isolation(spec, all_specs);

    // Extract acceptance criteria for AC quality scoring
    let criteria = extract_acceptance_criteria(spec);
    let ac_quality = calculate_ac_quality(&criteria);

    let mut score = SpecScore {
        complexity,
        confidence,
        splittability,
        isolation,
        ac_quality,
        traffic_light: chant::scoring::TrafficLight::Ready, // temporary, will be overwritten
    };

    // Determine final traffic light status
    score.traffic_light = determine_status(&score);

    score
}

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
                &spec.id,
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
                    &spec.id,
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
                &spec.id,
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
                    &spec.id,
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
                    &spec.id,
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
                    &spec.id,
                    LintRule::Approval,
                    format!(
                        "Approval status is {:?} but 'by' field is missing",
                        approval.status
                    ),
                ));
            }
            if approval.at.is_none() {
                diagnostics.push(LintDiagnostic::error(
                    &spec.id,
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
                &spec.id,
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
            &spec.id,
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
                    &spec.id,
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
                &spec.id,
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
                &spec.id,
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
                    &spec.id,
                    LintRule::Type,
                    "Documentation spec missing 'tracks' field".to_string(),
                ));
            }
            if spec.frontmatter.target_files.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    &spec.id,
                    LintRule::Type,
                    "Documentation spec missing 'target_files' field".to_string(),
                ));
            }
        }
        "research" => {
            if spec.frontmatter.informed_by.is_none() && spec.frontmatter.origin.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    &spec.id,
                    LintRule::Type,
                    "Research spec missing both 'informed_by' and 'origin' fields".to_string(),
                ));
            }
            if spec.frontmatter.target_files.is_none() {
                diagnostics.push(LintDiagnostic::warning(
                    &spec.id,
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
                        &spec.id,
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
    let mut all_specs: Vec<Spec> = Vec::new();
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
                all_specs.push(spec);
            }
        }
    }

    // Build set of all spec IDs for dependency validation
    let all_spec_ids: std::collections::HashSet<&str> =
        all_specs.iter().map(|s| s.id.as_str()).collect();

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
                &spec.id,
                LintRule::Title,
                "Missing title".to_string(),
            ));
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id.as_str()) {
                    spec_diagnostics.push(LintDiagnostic::error(
                        &spec.id,
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

pub fn cmd_lint(format: LintFormat, verbose: bool) -> Result<()> {
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

    // First pass: collect all specs and check for parse errors
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
                    specs_to_check.push(spec);
                }
                Err(e) => {
                    let diagnostic = LintDiagnostic::error(
                        &id,
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

    // Build set of all spec IDs for dependency validation
    let all_spec_ids: std::collections::HashSet<&str> =
        specs_to_check.iter().map(|s| s.id.as_str()).collect();

    // Second pass: validate each spec
    for spec in &specs_to_check {
        let mut spec_diagnostics: Vec<LintDiagnostic> = Vec::new();

        // Check for title
        if spec.title.is_none() {
            spec_diagnostics.push(LintDiagnostic::error(
                &spec.id,
                LintRule::Title,
                "Missing title".to_string(),
            ));
        }

        // Check depends_on references
        if let Some(deps) = &spec.frontmatter.depends_on {
            for dep_id in deps {
                if !all_spec_ids.contains(dep_id.as_str()) {
                    spec_diagnostics.push(LintDiagnostic::error(
                        &spec.id,
                        LintRule::Dependency,
                        format!("Unknown dependency '{}'", dep_id),
                    ));
                }
            }
        }

        // Check members references
        if let Some(members) = &spec.frontmatter.members {
            for member_id in members {
                if !all_spec_ids.contains(member_id.as_str()) {
                    spec_diagnostics.push(LintDiagnostic::error(
                        &spec.id,
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
                            &spec.id,
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
            // Calculate quality score for this spec
            let score = calculate_spec_score(spec, &specs_to_check);

            // Build score display string
            let mut score_display = format!(
                "{} | Complexity: {} | Confidence: {} | Splittable: {}",
                score.traffic_light, score.complexity, score.confidence, score.splittability
            );

            // Add isolation grade if present (specs with members)
            if verbose {
                if let Some(isolation) = score.isolation {
                    score_display.push_str(&format!(" | Isolation: {}", isolation));
                }
                score_display.push_str(&format!(" | AC Quality: {}", score.ac_quality));
            }

            if spec_diagnostics.is_empty() {
                println!("{} {}: {}", "✓".green(), spec.id, score_display);
            } else {
                for diagnostic in &errors {
                    println!(
                        "{} {}: {} | {}",
                        "✗".red(),
                        spec.id,
                        score_display,
                        diagnostic.message
                    );
                }
                // Check if there are complexity warnings
                let has_complexity_warning =
                    warnings.iter().any(|d| d.rule == LintRule::Complexity);
                for diagnostic in &warnings {
                    println!(
                        "{} {}: {} | {}",
                        "⚠".yellow(),
                        spec.id,
                        score_display,
                        diagnostic.message
                    );
                    if let Some(ref suggestion) = diagnostic.suggestion {
                        println!("    {} {}", "→".cyan(), suggestion);
                    }
                }
                // Suggest split if there are complexity warnings
                if has_complexity_warning {
                    println!("    {} Consider: chant split {}", "→".cyan(), spec.id);
                }
            }

            // Show quality suggestions for Review/Refine status
            use chant::scoring::TrafficLight;
            if matches!(
                score.traffic_light,
                TrafficLight::Review | TrafficLight::Refine
            ) {
                // Show basic suggestions
                let suggestions = generate_suggestions(&score);
                const MAX_SUGGESTIONS: usize = 3;
                for (i, suggestion) in suggestions.iter().take(MAX_SUGGESTIONS).enumerate() {
                    println!("    {} {}", "→".cyan(), suggestion);
                    if i == MAX_SUGGESTIONS - 1 && suggestions.len() > MAX_SUGGESTIONS {
                        let remaining = suggestions.len() - MAX_SUGGESTIONS;
                        println!("    {} ... and {} more", "→".cyan(), remaining);
                        break;
                    }
                }

                // Show detailed guidance in verbose mode
                if verbose {
                    use chant::score::traffic_light::generate_detailed_guidance;
                    let guidance = generate_detailed_guidance(&score);
                    if !guidance.is_empty() {
                        print!("{}", guidance);
                    }
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
                    spec_errors.insert(&diag.spec_id);
                } else {
                    spec_warnings.insert(&diag.spec_id);
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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chant::config::LintThresholds;
    use chant::spec::{Spec, SpecFrontmatter};

    /// Create a test spec with the specified number of criteria, files, and approximate body words.
    /// Note: The body will contain slightly more words than `body_words` due to the Acceptance Criteria
    /// section, but this helper ensures consistent test data.
    fn create_test_spec(
        id: &str,
        criteria_count: usize,
        file_count: usize,
        body_words: usize,
    ) -> Spec {
        let mut body = String::new();

        // Add body content with approximately the specified word count
        // We add simple filler text to control the word count
        if body_words > 0 {
            for _ in 0..body_words {
                body.push_str("word ");
            }
            body.push('\n');
        }

        // Add Acceptance Criteria section with specified number of checkboxes
        if criteria_count > 0 {
            body.push_str("\n## Acceptance Criteria\n\n");
            for i in 0..criteria_count {
                body.push_str(&format!("- [ ] Criterion {}\n", i + 1));
            }
        }

        let mut frontmatter = SpecFrontmatter::default();
        if file_count > 0 {
            frontmatter.target_files =
                Some((0..file_count).map(|i| format!("file{}.rs", i)).collect());
        }

        Spec {
            id: id.to_string(),
            frontmatter,
            title: Some("Test Spec".to_string()),
            body,
        }
    }

    #[test]
    fn test_validate_spec_complexity_below_thresholds() {
        // Default thresholds: criteria=10, files=5, words=50
        // Create spec well below all thresholds
        let spec = create_test_spec("2026-01-30-001-abc", 5, 3, 20);
        let thresholds = LintThresholds::default();

        let diagnostics = validate_spec_complexity(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should have no diagnostics when below all thresholds"
        );
    }

    #[test]
    fn test_validate_spec_complexity_exceeds_criteria() {
        // Default threshold: 10 criteria, 50 words
        // Create spec with 11 criteria (just above threshold), but within files limit
        // Note: Each criterion adds ~5 words, so 11 criteria ≈ 58 words total
        // This will trigger both criteria and words warnings, which is expected
        let spec = create_test_spec("2026-01-30-002-def", 11, 3, 0);
        let thresholds = LintThresholds::default();

        let diagnostics = validate_spec_complexity(&spec, &thresholds);

        // Find the criteria diagnostic
        let criteria_diag = diagnostics
            .iter()
            .find(|d| d.message.contains("acceptance criteria"))
            .expect("Should have criteria diagnostic");

        assert_eq!(criteria_diag.rule, LintRule::Complexity);
        assert_eq!(criteria_diag.severity, Severity::Warning);
        assert!(
            criteria_diag.message.contains("11 acceptance criteria"),
            "Message should mention the criteria count"
        );
        assert!(
            criteria_diag.message.contains(">10"),
            "Message should mention the threshold"
        );
        assert!(
            criteria_diag.suggestion.is_some(),
            "Should have a suggestion"
        );
    }

    #[test]
    fn test_validate_spec_complexity_exceeds_files() {
        // Default threshold: 5 files
        // Create spec with 8 files, but within criteria and words limits
        let spec = create_test_spec("2026-01-30-003-ghi", 5, 8, 20);
        let thresholds = LintThresholds::default();

        let diagnostics = validate_spec_complexity(&spec, &thresholds);

        assert_eq!(diagnostics.len(), 1, "Should have exactly one diagnostic");
        assert_eq!(diagnostics[0].rule, LintRule::Complexity);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("8 files"),
            "Message should mention the file count"
        );
        assert!(
            diagnostics[0].message.contains(">5"),
            "Message should mention the threshold"
        );
        assert!(
            diagnostics[0].suggestion.is_some(),
            "Should have a suggestion"
        );
    }

    #[test]
    fn test_validate_spec_complexity_exceeds_words() {
        // Default threshold: 50 words
        // Create spec with 100 words, but within criteria and files limits
        let spec = create_test_spec("2026-01-30-004-jkl", 5, 3, 100);
        let thresholds = LintThresholds::default();

        let diagnostics = validate_spec_complexity(&spec, &thresholds);

        assert_eq!(diagnostics.len(), 1, "Should have exactly one diagnostic");
        assert_eq!(diagnostics[0].rule, LintRule::Complexity);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains(" words"),
            "Message should mention words"
        );
        assert!(
            diagnostics[0].message.contains(">50"),
            "Message should mention the threshold"
        );
        assert!(
            diagnostics[0].suggestion.is_some(),
            "Should have a suggestion"
        );
    }

    #[test]
    fn test_validate_spec_complexity_custom_thresholds() {
        // Custom thresholds: criteria=7, files=3, words=35
        // Create spec with: criteria=8, files=4, words=40
        let spec = create_test_spec("2026-01-30-005-mno", 8, 4, 40);
        let custom_thresholds = LintThresholds {
            complexity_criteria: 7,
            complexity_files: 3,
            complexity_words: 35,
            simple_criteria: 1,
            simple_files: 1,
            simple_words: 3,
        };

        let diagnostics = validate_spec_complexity(&spec, &custom_thresholds);

        // Should trigger all three warnings with custom thresholds
        assert_eq!(
            diagnostics.len(),
            3,
            "Should have three diagnostics with custom thresholds"
        );

        // Check that we have warnings for criteria, files, and words
        let has_criteria_warning = diagnostics
            .iter()
            .any(|d| d.message.contains("8 acceptance criteria"));
        let has_files_warning = diagnostics.iter().any(|d| d.message.contains("4 files"));
        let has_words_warning = diagnostics.iter().any(|d| d.message.contains(" words"));

        assert!(has_criteria_warning, "Should have criteria warning");
        assert!(has_files_warning, "Should have files warning");
        assert!(has_words_warning, "Should have words warning");
    }

    #[test]
    fn test_validate_spec_complexity_multiple_thresholds_exceeded() {
        // Default thresholds: criteria=10, files=5, words=50
        // Create spec that exceeds all: criteria=15, files=8, words=100
        let spec = create_test_spec("2026-01-30-006-pqr", 15, 8, 100);
        let thresholds = LintThresholds::default();

        let diagnostics = validate_spec_complexity(&spec, &thresholds);

        // Should trigger all three warnings
        assert_eq!(
            diagnostics.len(),
            3,
            "Should have three diagnostics when all thresholds exceeded"
        );
        assert!(diagnostics.iter().all(|d| d.rule == LintRule::Complexity));
        assert!(diagnostics.iter().all(|d| d.severity == Severity::Warning));
    }

    #[test]
    fn test_validate_spec_coupling_no_references() {
        // Create a regular spec with no spec ID references
        let spec = Spec {
            id: "2026-01-30-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "This is a spec without any spec ID references.".to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should have no diagnostics when there are no spec references"
        );
    }

    #[test]
    fn test_validate_spec_coupling_driver_excluded() {
        // Create a driver spec that references member specs
        let spec = Spec {
            id: "2026-01-30-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "driver".to_string(),
                ..Default::default()
            },
            title: Some("Test Driver".to_string()),
            body: "This driver references 2026-01-30-002-def.1 and 2026-01-30-002-def.2."
                .to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Driver specs should be excluded from coupling check"
        );
    }

    #[test]
    fn test_validate_spec_coupling_group_excluded() {
        // Create a group spec that references other specs
        let spec = Spec {
            id: "2026-01-30-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "group".to_string(),
                ..Default::default()
            },
            title: Some("Test Group".to_string()),
            body: "This group references 2026-01-30-003-ghi.1 and 2026-01-30-004-jkl.".to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Group specs should be excluded from coupling check"
        );
    }

    #[test]
    fn test_validate_spec_coupling_regular_spec_with_reference() {
        // Create a regular spec that references another spec ID
        let spec = Spec {
            id: "2026-01-30-004-jkl".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "This spec depends on 2026-01-30-003-ghi for completion.".to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for spec ID reference"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Coupling);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("2026-01-30-003-ghi"),
            "Message should mention the referenced spec ID"
        );
        assert!(
            diagnostics[0].message.contains("use depends_on"),
            "Message should suggest using depends_on"
        );
    }

    #[test]
    fn test_validate_spec_coupling_self_reference_excluded() {
        // Create a spec that references its own ID
        let spec = Spec {
            id: "2026-01-30-005-mno".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "This spec is 2026-01-30-005-mno and references itself.".to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Self-references should be excluded from coupling check"
        );
    }

    #[test]
    fn test_validate_spec_coupling_code_block_excluded() {
        // Create a spec with spec IDs in code blocks
        let spec = Spec {
            id: "2026-01-30-006-pqr".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: r#"
This spec shows code examples:

```bash
chant work 2026-01-30-003-ghi
```

And another example:
```
2026-01-30-004-jkl
```

No coupling issues here.
"#
            .to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Spec IDs in code blocks should be excluded from coupling check"
        );
    }

    #[test]
    fn test_validate_spec_coupling_member_spec_sibling_warning() {
        // Create a member spec that references a sibling member
        let spec = Spec {
            id: "2026-01-30-007-stu.1".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test Member".to_string()),
            body: "This member depends on 2026-01-30-007-stu.2 being completed.".to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for sibling reference"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Coupling);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("2026-01-30-007-stu.2"),
            "Message should mention the sibling spec ID"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("member specs should be independent"),
            "Message should mention member independence"
        );
    }

    #[test]
    fn test_validate_spec_coupling_member_spec_non_sibling_ok() {
        // Create a member spec that references a non-sibling spec (different driver)
        let spec = Spec {
            id: "2026-01-30-008-vwx.1".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                ..Default::default()
            },
            title: Some("Test Member".to_string()),
            body: "This member references 2026-01-30-009-yza which is not a sibling.".to_string(),
        };

        let diagnostics = validate_spec_coupling(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Member specs should be allowed to reference non-sibling specs"
        );
    }

    #[test]
    fn test_validate_spec_type_documentation_missing_tracks() {
        // Create a documentation spec without tracks field
        let spec = Spec {
            id: "2026-01-30-010-abc".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "documentation".to_string(),
                target_files: Some(vec!["README.md".to_string()]),
                ..Default::default()
            },
            title: Some("Documentation Spec".to_string()),
            body: "Document the API.".to_string(),
        };

        let diagnostics = validate_spec_type(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for missing tracks"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Type);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("missing 'tracks'"),
            "Message should mention missing tracks field"
        );
    }

    #[test]
    fn test_validate_spec_type_documentation_missing_target_files() {
        // Create a documentation spec without target_files field
        let spec = Spec {
            id: "2026-01-30-011-def".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "documentation".to_string(),
                tracks: Some(vec!["2026-01-30-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Documentation Spec".to_string()),
            body: "Document the API.".to_string(),
        };

        let diagnostics = validate_spec_type(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for missing target_files"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Type);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("missing 'target_files'"),
            "Message should mention missing target_files field"
        );
    }

    #[test]
    fn test_validate_spec_type_research_missing_fields() {
        // Create a research spec without informed_by or origin
        let spec = Spec {
            id: "2026-01-30-012-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "research".to_string(),
                target_files: Some(vec!["analysis.md".to_string()]),
                ..Default::default()
            },
            title: Some("Research Spec".to_string()),
            body: "Research the topic.".to_string(),
        };

        let diagnostics = validate_spec_type(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for missing informed_by and origin"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Type);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0]
                .message
                .contains("missing both 'informed_by' and 'origin'"),
            "Message should mention missing informed_by and origin fields"
        );
    }

    #[test]
    fn test_validate_spec_type_driver_empty_members() {
        // Create a driver spec with empty members array
        let spec = Spec {
            id: "2026-01-30-013-jkl".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "driver".to_string(),
                members: Some(vec![]),
                ..Default::default()
            },
            title: Some("Driver Spec".to_string()),
            body: "Driver with no members.".to_string(),
        };

        let diagnostics = validate_spec_type(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for empty members"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Type);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("empty 'members' array"),
            "Message should mention empty members array"
        );
    }

    #[test]
    fn test_validate_spec_type_driver_with_members_ok() {
        // Create a driver spec with non-empty members array
        let spec = Spec {
            id: "2026-01-30-014-mno".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "driver".to_string(),
                members: Some(vec![
                    "2026-01-30-014-mno.1".to_string(),
                    "2026-01-30-014-mno.2".to_string(),
                ]),
                ..Default::default()
            },
            title: Some("Driver Spec".to_string()),
            body: "Driver with members.".to_string(),
        };

        let diagnostics = validate_spec_type(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should have no diagnostics when driver has members"
        );
    }

    #[test]
    fn test_validate_model_waste_no_model_set() {
        // Create a simple spec without model field
        let spec = create_test_spec("2026-01-30-020-abc", 1, 1, 5);
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should have no diagnostics when model is not set"
        );
    }

    #[test]
    fn test_validate_model_waste_haiku_ok() {
        // Create a simple spec with haiku model - should not trigger warning
        let mut spec = create_test_spec("2026-01-30-021-def", 1, 1, 5);
        spec.frontmatter.model = Some("haiku".to_string());
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            0,
            "Haiku model should never trigger model waste warning"
        );
    }

    #[test]
    fn test_validate_model_waste_opus_on_simple_spec() {
        // Create a simple spec with opus model - should trigger warning
        // Default simple thresholds: criteria<=1, files<=1, words<=3
        // Create spec with 0 criteria, 0 files, 0 words to ensure it's simple
        let mut spec = create_test_spec("2026-01-30-022-ghi", 0, 0, 0);
        spec.frontmatter.model = Some("opus".to_string());
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for opus on simple spec"
        );
        assert_eq!(diagnostics[0].rule, LintRule::ModelWaste);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("opus"),
            "Message should mention opus model"
        );
        assert!(
            diagnostics[0].message.contains("simple"),
            "Message should mention spec is simple"
        );
        assert!(
            diagnostics[0].suggestion.is_some(),
            "Should have a suggestion"
        );
    }

    #[test]
    fn test_validate_model_waste_sonnet_on_simple_spec() {
        // Create a simple spec with sonnet model - should trigger warning
        let mut spec = create_test_spec("2026-01-30-023-jkl", 0, 0, 0);
        spec.frontmatter.model = Some("sonnet".to_string());
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            1,
            "Should have one diagnostic for sonnet on simple spec"
        );
        assert_eq!(diagnostics[0].rule, LintRule::ModelWaste);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(
            diagnostics[0].message.contains("sonnet"),
            "Message should mention sonnet model"
        );
        assert!(
            diagnostics[0].message.contains("simple"),
            "Message should mention spec is simple"
        );
    }

    #[test]
    fn test_validate_model_waste_research_excluded() {
        // Create a simple research spec with opus - should not trigger warning
        let mut spec = create_test_spec("2026-01-30-024-mno", 0, 0, 0);
        spec.frontmatter.r#type = "research".to_string();
        spec.frontmatter.model = Some("opus".to_string());
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            0,
            "Research specs should be excluded from model waste check"
        );
    }

    #[test]
    fn test_validate_model_waste_driver_excluded() {
        // Create a simple driver spec with sonnet - should not trigger warning
        let mut spec = create_test_spec("2026-01-30-025-pqr", 0, 0, 0);
        spec.frontmatter.r#type = "driver".to_string();
        spec.frontmatter.model = Some("sonnet".to_string());
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            0,
            "Driver specs should be excluded from model waste check"
        );
    }

    #[test]
    fn test_validate_model_waste_complex_spec_ok() {
        // Create a complex spec with opus - should not trigger warning
        // Default simple thresholds: criteria<=1, files<=1, words<=3
        // This spec exceeds all simple thresholds so it's considered complex
        let mut spec = create_test_spec("2026-01-30-026-stu", 5, 3, 20);
        spec.frontmatter.model = Some("opus".to_string());
        let thresholds = LintThresholds::default();

        let diagnostics = validate_model_waste(&spec, &thresholds);

        assert_eq!(
            diagnostics.len(),
            0,
            "Complex specs should be allowed to use expensive models"
        );
    }

    #[test]
    fn test_validate_approval_schema_no_approval_field() {
        // Create a spec without approval field - should be OK
        let spec = Spec {
            id: "2026-01-30-030-abc".to_string(),
            frontmatter: SpecFrontmatter {
                approval: None,
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "No approval needed.".to_string(),
        };

        let diagnostics = validate_approval_schema(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Spec without approval field should have no diagnostics"
        );
    }

    #[test]
    fn test_validate_approval_schema_pending_ok() {
        // Create a spec with pending approval - should be OK
        let spec = Spec {
            id: "2026-01-30-031-def".to_string(),
            frontmatter: SpecFrontmatter {
                approval: Some(chant::spec::Approval {
                    required: false,
                    status: ApprovalStatus::Pending,
                    by: None,
                    at: None,
                }),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "Pending approval.".to_string(),
        };

        let diagnostics = validate_approval_schema(&spec);

        assert_eq!(
            diagnostics.len(),
            0,
            "Pending approval without 'by' or 'at' should have no diagnostics"
        );
    }

    #[test]
    fn test_validate_approval_schema_approved_missing_by() {
        // Create a spec with approved status but missing 'by' field
        let spec = Spec {
            id: "2026-01-30-032-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                approval: Some(chant::spec::Approval {
                    required: false,
                    status: ApprovalStatus::Approved,
                    by: None,
                    at: Some("2026-01-30T12:00:00Z".to_string()),
                }),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "Approved but missing by.".to_string(),
        };

        let diagnostics = validate_approval_schema(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Approved status without 'by' should have one diagnostic"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Approval);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0].message.contains("'by' field is missing"),
            "Message should mention missing 'by' field"
        );
    }

    #[test]
    fn test_validate_approval_schema_approved_missing_at() {
        // Create a spec with approved status but missing 'at' field
        let spec = Spec {
            id: "2026-01-30-033-jkl".to_string(),
            frontmatter: SpecFrontmatter {
                approval: Some(chant::spec::Approval {
                    required: false,
                    status: ApprovalStatus::Approved,
                    by: Some("alice".to_string()),
                    at: None,
                }),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "Approved but missing at.".to_string(),
        };

        let diagnostics = validate_approval_schema(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Approved status without 'at' should have one diagnostic"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Approval);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0].message.contains("'at' timestamp is missing"),
            "Message should mention missing 'at' timestamp"
        );
    }

    #[test]
    fn test_validate_approval_schema_pending_with_by_inconsistent() {
        // Create a spec with pending status but 'by' field set - inconsistent
        let spec = Spec {
            id: "2026-01-30-034-mno".to_string(),
            frontmatter: SpecFrontmatter {
                approval: Some(chant::spec::Approval {
                    required: false,
                    status: ApprovalStatus::Pending,
                    by: Some("alice".to_string()),
                    at: None,
                }),
                ..Default::default()
            },
            title: Some("Test Spec".to_string()),
            body: "Pending with by field.".to_string(),
        };

        let diagnostics = validate_approval_schema(&spec);

        assert_eq!(
            diagnostics.len(),
            1,
            "Pending status with 'by' field should have one diagnostic"
        );
        assert_eq!(diagnostics[0].rule, LintRule::Approval);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(
            diagnostics[0]
                .message
                .contains("has 'by' field set but status is still 'pending'"),
            "Message should mention inconsistency"
        );
    }
}
