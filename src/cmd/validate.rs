//! Unified validation framework for lint/verify/config-validate
//!
//! This module provides a common validation framework that unifies:
//! - Spec linting (structural validation)
//! - Spec verification (acceptance criteria validation)
//! - Config validation (configuration file validation)

#![allow(dead_code)] // Framework is being progressively adopted

use anyhow::Result;
use colored::Colorize;

/// Category of validation operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCategory {
    /// Spec structure and quality
    Lint,
    /// Acceptance criteria verification
    Verify,
    /// Configuration validation
    Config,
}

impl std::fmt::Display for ValidationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lint => write!(f, "Lint"),
            Self::Verify => write!(f, "Verify"),
            Self::Config => write!(f, "Config"),
        }
    }
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning - should be addressed but not critical
    Warning,
    /// Error - must be fixed
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// A single validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Category of validation that found this issue
    pub category: ValidationCategory,
    /// ID of the item being validated (spec ID, config file, etc.)
    pub item_id: String,
    /// Message describing the issue
    pub message: String,
    /// Optional suggestion for fixing the issue
    pub suggestion: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(
        severity: Severity,
        category: ValidationCategory,
        item_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            item_id: item_id.into(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Add a suggestion to this issue
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Display the issue with colored output
    pub fn display(&self) {
        let icon = match self.severity {
            Severity::Info => "ℹ".blue(),
            Severity::Warning => "⚠".yellow(),
            Severity::Error => "✗".red(),
        };

        println!(
            "  {} {} [{}]: {}",
            icon,
            self.item_id.cyan(),
            self.category,
            self.message
        );

        if let Some(ref suggestion) = self.suggestion {
            println!("      {} {}", "→".cyan(), suggestion);
        }
    }
}

/// Result of a validation operation
#[derive(Debug)]
pub struct ValidationResult {
    /// Category of validation performed
    pub category: ValidationCategory,
    /// Total items validated
    pub total: usize,
    /// Items that passed validation
    pub passed: usize,
    /// Items with warnings only
    pub warned: usize,
    /// Items that failed validation
    pub failed: usize,
    /// All issues found during validation
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new(category: ValidationCategory) -> Self {
        Self {
            category,
            total: 0,
            passed: 0,
            warned: 0,
            failed: 0,
            issues: Vec::new(),
        }
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Warning)
    }

    /// Add an issue to the result
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }

    /// Display a summary of the validation result
    pub fn display_summary(&self) {
        println!();
        println!("{}", "━".repeat(60).cyan());

        let status_icon = if self.is_valid() {
            "✓".green()
        } else {
            "✗".red()
        };

        print!("{} {} validation: ", status_icon, self.category);

        if self.total > 0 {
            print!("{} total", self.total);

            if self.passed > 0 {
                print!(", {} {}", self.passed, "passed".green());
            }
            if self.warned > 0 {
                print!(", {} {}", self.warned, "warned".yellow());
            }
            if self.failed > 0 {
                print!(", {} {}", self.failed, "failed".red());
            }
            println!();
        } else {
            println!("no items to validate");
        }

        // Display error/warning counts
        let error_count = self
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let warning_count = self
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count();

        if error_count > 0 || warning_count > 0 {
            print!("  ");
            if error_count > 0 {
                print!(
                    "{} {}",
                    error_count,
                    if error_count == 1 { "error" } else { "errors" }.red()
                );
            }
            if error_count > 0 && warning_count > 0 {
                print!(", ");
            }
            if warning_count > 0 {
                print!(
                    "{} {}",
                    warning_count,
                    if warning_count == 1 {
                        "warning"
                    } else {
                        "warnings"
                    }
                    .yellow()
                );
            }
            println!();
        }

        println!("{}", "━".repeat(60).cyan());
    }

    /// Exit with appropriate code based on validation result
    pub fn exit_if_failed(&self) -> Result<()> {
        if !self.is_valid() {
            std::process::exit(1);
        }
        Ok(())
    }
}

/// Trait for types that can be validated
pub trait Validate {
    /// Perform validation and return issues
    fn validate(&self) -> Vec<ValidationIssue>;
}

/// Convert from lint Severity to unified Severity
impl From<crate::cmd::spec::lint::Severity> for Severity {
    fn from(s: crate::cmd::spec::lint::Severity) -> Self {
        match s {
            crate::cmd::spec::lint::Severity::Error => Severity::Error,
            crate::cmd::spec::lint::Severity::Warning => Severity::Warning,
        }
    }
}

/// Convert from lint LintDiagnostic to ValidationIssue
impl From<crate::cmd::spec::lint::LintDiagnostic> for ValidationIssue {
    fn from(diag: crate::cmd::spec::lint::LintDiagnostic) -> Self {
        let mut issue = ValidationIssue::new(
            diag.severity.into(),
            ValidationCategory::Lint,
            diag.spec_id.clone(),
            diag.message.clone(),
        );
        if let Some(suggestion) = diag.suggestion {
            issue = issue.with_suggestion(suggestion);
        }
        issue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_issue_creation() {
        let issue = ValidationIssue::new(
            Severity::Error,
            ValidationCategory::Lint,
            "test-001",
            "Test error",
        );

        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.category, ValidationCategory::Lint);
        assert_eq!(issue.item_id, "test-001");
        assert_eq!(issue.message, "Test error");
        assert!(issue.suggestion.is_none());
    }

    #[test]
    fn test_validation_issue_with_suggestion() {
        let issue = ValidationIssue::new(
            Severity::Warning,
            ValidationCategory::Verify,
            "test-002",
            "Test warning",
        )
        .with_suggestion("Try this fix");

        assert_eq!(issue.suggestion, Some("Try this fix".to_string()));
    }

    #[test]
    fn test_validation_result_is_valid() {
        let mut result = ValidationResult::new(ValidationCategory::Lint);
        result.total = 3;
        result.passed = 2;
        result.warned = 1;
        result.failed = 0;

        // Add a warning
        result.add_issue(ValidationIssue::new(
            Severity::Warning,
            ValidationCategory::Lint,
            "test",
            "Warning message",
        ));

        assert!(result.is_valid()); // Warnings don't fail validation
        assert!(result.has_warnings());
        assert!(!result.has_errors());
    }

    #[test]
    fn test_validation_result_with_errors() {
        let mut result = ValidationResult::new(ValidationCategory::Config);
        result.total = 2;
        result.passed = 1;
        result.failed = 1;

        result.add_issue(ValidationIssue::new(
            Severity::Error,
            ValidationCategory::Config,
            "config.toml",
            "Invalid configuration",
        ));

        assert!(!result.is_valid());
        assert!(result.has_errors());
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn test_validation_category_display() {
        assert_eq!(ValidationCategory::Lint.to_string(), "Lint");
        assert_eq!(ValidationCategory::Verify.to_string(), "Verify");
        assert_eq!(ValidationCategory::Config.to_string(), "Config");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "INFO");
        assert_eq!(Severity::Warning.to_string(), "WARN");
        assert_eq!(Severity::Error.to_string(), "ERROR");
    }
}
