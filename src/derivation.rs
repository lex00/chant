//! Derivation engine for extracting values from multiple sources.
//!
//! The derivation engine extracts values from:
//! - **Branch name** - Current git branch (e.g., `sprint/2026-Q1-W4/PROJ-123`)
//! - **File path** - Spec file path (e.g., `.chant/specs/teams/platform/...`)
//! - **Environment variables** - Shell environment (e.g., `$TEAM_NAME`)
//! - **Git user** - Git user.name or user.email from config
//!
//! For each source, the engine applies regex patterns to extract the first capture group.
//! If a pattern doesn't match, the engine returns None for that field (graceful failure).

use crate::config::{DerivationSource, DerivedFieldConfig, EnterpriseConfig, ValidationRule};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;

/// Result of validating a derived value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Value is valid
    Valid,
    /// Value is invalid but derivation proceeds with a warning
    Warning(String),
}

/// Context containing all available data sources for derivation
#[derive(Debug, Clone)]
pub struct DerivationContext {
    /// Current git branch name
    pub branch_name: Option<String>,
    /// Spec file path
    pub spec_path: Option<PathBuf>,
    /// Environment variables available for extraction
    pub env_vars: HashMap<String, String>,
    /// Git user.name from config
    pub git_user_name: Option<String>,
    /// Git user.email from config
    pub git_user_email: Option<String>,
}

impl DerivationContext {
    /// Create a new empty derivation context
    pub fn new() -> Self {
        Self {
            branch_name: None,
            spec_path: None,
            env_vars: HashMap::new(),
            git_user_name: None,
            git_user_email: None,
        }
    }

    /// Create a derivation context with environment variables
    pub fn with_env_vars(env_vars: HashMap<String, String>) -> Self {
        Self {
            branch_name: None,
            spec_path: None,
            env_vars,
            git_user_name: None,
            git_user_email: None,
        }
    }
}

impl Default for DerivationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a DerivationContext populated with current environment data.
///
/// This creates a fully populated context with:
/// - Current git branch name
/// - Spec file path (constructed from spec_id and specs_dir)
/// - All environment variables
/// - Git user name and email from config
///
/// This is the canonical way to build a context for derivation operations.
pub fn build_context(spec_id: &str, specs_dir: &std::path::Path) -> DerivationContext {
    use crate::git;

    let mut context = DerivationContext::new();

    // Get current branch
    if let Ok(branch) = git::get_current_branch() {
        context.branch_name = Some(branch);
    }

    // Get spec path
    let spec_path = specs_dir.join(format!("{}.md", spec_id));
    context.spec_path = Some(spec_path);

    // Capture environment variables
    context.env_vars = std::env::vars().collect();

    // Get git user info
    let (name, email) = git::get_git_user_info();
    context.git_user_name = name;
    context.git_user_email = email;

    context
}

/// Engine for deriving field values from configured sources
#[derive(Debug, Clone)]
pub struct DerivationEngine {
    config: EnterpriseConfig,
}

impl DerivationEngine {
    /// Create a new derivation engine with the given configuration
    pub fn new(config: EnterpriseConfig) -> Self {
        Self { config }
    }

    /// Derive all configured fields for a spec
    ///
    /// Returns a HashMap with field names as keys and derived values as values.
    /// Fields that fail to match their pattern are omitted from the result.
    /// If the enterprise config is empty, returns an empty HashMap (fast path).
    pub fn derive_fields(&self, context: &DerivationContext) -> HashMap<String, String> {
        // Fast path: if no derivation config, return empty
        if self.config.derived.is_empty() {
            return HashMap::new();
        }

        let mut result = HashMap::new();

        for (field_name, field_config) in &self.config.derived {
            if let Some(value) = self.derive_field(field_name, field_config, context) {
                result.insert(field_name.clone(), value);
            }
        }

        result
    }

    /// Derive a single field from its source using the configured pattern
    ///
    /// For Branch and Path sources: Extracts the first capture group from the pattern match.
    /// For Env and GitUser sources: Returns the value directly (pattern is the field identifier).
    /// Returns None if the pattern doesn't match or the source is unavailable.
    /// Validates the derived value if a validation rule is configured.
    fn derive_field(
        &self,
        field_name: &str,
        config: &DerivedFieldConfig,
        context: &DerivationContext,
    ) -> Option<String> {
        let value = match config.from {
            DerivationSource::Branch => {
                let source_value = self.extract_from_branch(context)?;
                self.apply_pattern(&config.pattern, &source_value)
                    .or_else(|| {
                        eprintln!(
                            "Warning: derivation pattern for field '{}' did not match source",
                            field_name
                        );
                        None
                    })?
            }
            DerivationSource::Path => {
                let source_value = self.extract_from_path(context)?;
                self.apply_pattern(&config.pattern, &source_value)
                    .or_else(|| {
                        eprintln!(
                            "Warning: derivation pattern for field '{}' did not match source",
                            field_name
                        );
                        None
                    })?
            }
            DerivationSource::Env => {
                // For Env, pattern is the environment variable name
                self.extract_from_env(context, &config.pattern)?
            }
            DerivationSource::GitUser => {
                // For GitUser, pattern is "name" or "email"
                self.extract_from_git_user(context, &config.pattern)?
            }
        };

        // Validate the derived value if a validation rule is configured
        if let Some(validation) = &config.validate {
            match self.validate_derived_value(field_name, &value, validation) {
                ValidationResult::Valid => {
                    // Value is valid, proceed
                }
                ValidationResult::Warning(msg) => {
                    // Log warning but still include the value in results
                    eprintln!("Warning: {}", msg);
                }
            }
        }

        Some(value)
    }

    /// Extract value from branch name source
    fn extract_from_branch(&self, context: &DerivationContext) -> Option<String> {
        context.branch_name.clone()
    }

    /// Extract value from file path source
    fn extract_from_path(&self, context: &DerivationContext) -> Option<String> {
        context
            .spec_path
            .as_ref()
            .and_then(|path| path.to_str().map(|s| s.to_string()))
    }

    /// Extract value from environment variable source
    ///
    /// The pattern parameter is treated as the environment variable name
    fn extract_from_env(&self, context: &DerivationContext, env_name: &str) -> Option<String> {
        context.env_vars.get(env_name).cloned()
    }

    /// Extract value from git user source
    ///
    /// The pattern parameter can be "name" for user.name or "email" for user.email
    fn extract_from_git_user(
        &self,
        context: &DerivationContext,
        field_type: &str,
    ) -> Option<String> {
        match field_type {
            "name" => context.git_user_name.clone(),
            "email" => context.git_user_email.clone(),
            _ => None,
        }
    }

    /// Apply regex pattern to extract the first capture group
    ///
    /// Returns None if pattern is invalid or doesn't match
    fn apply_pattern(&self, pattern: &str, source: &str) -> Option<String> {
        let regex = Regex::new(pattern).ok()?;
        regex
            .captures(source)?
            .get(1)
            .map(|m| m.as_str().to_string())
    }

    /// Validate a derived value against its validation rule
    ///
    /// Returns Valid if the value passes validation, or Warning if it fails.
    /// Does not prevent the value from being included in results.
    fn validate_derived_value(
        &self,
        field_name: &str,
        value: &str,
        validation: &ValidationRule,
    ) -> ValidationResult {
        match validation {
            ValidationRule::Enum { values } => {
                if values.contains(&value.to_string()) {
                    ValidationResult::Valid
                } else {
                    ValidationResult::Warning(format!(
                        "Field '{}' value '{}' is not in allowed enum values: {}",
                        field_name,
                        value,
                        values.join(", ")
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_engine(derived: HashMap<String, DerivedFieldConfig>) -> DerivationEngine {
        DerivationEngine::new(EnterpriseConfig {
            derived,
            required: vec![],
        })
    }

    // =========================================================================
    // TEST HELPERS FOR SOURCE TYPE TESTS
    // =========================================================================

    /// Test helper for derivation source tests
    fn assert_derive_from_source(
        field_name: &str,
        config: DerivedFieldConfig,
        context: DerivationContext,
        expected: Option<&str>,
    ) {
        let mut derived = HashMap::new();
        derived.insert(field_name.to_string(), config);
        let engine = create_test_engine(derived);
        let result = engine.derive_fields(&context);

        match expected {
            Some(val) => assert_eq!(result.get(field_name), Some(&val.to_string())),
            None => assert!(!result.contains_key(field_name)),
        }
    }

    // =========================================================================
    // BRANCH NAME EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_branch_basic() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: r"^(dev|staging|prod)".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.branch_name = Some("prod/feature-123".to_string());

        assert_derive_from_source("env", config, context, Some("prod"));
    }

    #[test]
    fn test_derive_from_branch_with_capture_group() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: r"sprint/.*/(PROJ-\d+)".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.branch_name = Some("sprint/2026-Q1-W4/PROJ-123".to_string());

        assert_derive_from_source("project", config, context, Some("PROJ-123"));
    }

    #[test]
    fn test_derive_from_branch_no_match() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: r"^(dev|staging|prod)".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.branch_name = Some("feature/my-branch".to_string());

        assert_derive_from_source("env", config, context, None);
    }

    #[test]
    fn test_derive_from_branch_missing() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: r"^(dev|staging|prod)".to_string(),
            validate: None,
        };
        let context = DerivationContext::new(); // No branch_name

        assert_derive_from_source("env", config, context, None);
    }

    // =========================================================================
    // FILE PATH EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_path_basic() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Path,
            pattern: r"specs/([a-z]+)/".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/platform/feature.md"));

        assert_derive_from_source("team", config, context, Some("platform"));
    }

    #[test]
    fn test_derive_from_path_with_multiple_captures() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Path,
            pattern: r"specs/([a-z]+)/([A-Z0-9]+)-".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/teams/PROJ-123-feature.md"));

        // Should extract first capture group only
        assert_derive_from_source("project", config, context, Some("teams"));
    }

    #[test]
    fn test_derive_from_path_no_match() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Path,
            pattern: r"specs/([a-z]+)/".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/feature.md"));

        assert_derive_from_source("team", config, context, None);
    }

    #[test]
    fn test_derive_from_path_missing() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Path,
            pattern: r"specs/([a-z]+)/".to_string(),
            validate: None,
        };
        let context = DerivationContext::new(); // No spec_path

        assert_derive_from_source("team", config, context, None);
    }

    // =========================================================================
    // ENVIRONMENT VARIABLE EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_env_basic() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "TEAM_NAME".to_string(),
            validate: None,
        };
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "platform".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        assert_derive_from_source("team", config, context, Some("platform"));
    }

    #[test]
    fn test_derive_from_env_with_pattern_match() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "ENVIRONMENT".to_string(),
            validate: None,
        };
        let mut env_vars = HashMap::new();
        env_vars.insert("ENVIRONMENT".to_string(), "production".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        assert_derive_from_source("env_name", config, context, Some("production"));
    }

    #[test]
    fn test_derive_from_env_missing_variable() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "TEAM_NAME".to_string(),
            validate: None,
        };
        let context = DerivationContext::new(); // No env vars

        assert_derive_from_source("team", config, context, None);
    }

    #[test]
    fn test_derive_from_env_undefined_variable() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "TEAM_NAME".to_string(),
            validate: None,
        };
        let mut env_vars = HashMap::new();
        env_vars.insert("OTHER_VAR".to_string(), "value".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        assert_derive_from_source("team", config, context, None);
    }

    // =========================================================================
    // GIT USER EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_git_user_name() {
        let config = DerivedFieldConfig {
            from: DerivationSource::GitUser,
            pattern: "name".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.git_user_name = Some("John Doe".to_string());

        assert_derive_from_source("author", config, context, Some("John Doe"));
    }

    #[test]
    fn test_derive_from_git_user_email() {
        let config = DerivedFieldConfig {
            from: DerivationSource::GitUser,
            pattern: "email".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.git_user_email = Some("john@example.com".to_string());

        assert_derive_from_source("author_email", config, context, Some("john@example.com"));
    }

    #[test]
    fn test_derive_from_git_user_invalid_field() {
        let config = DerivedFieldConfig {
            from: DerivationSource::GitUser,
            pattern: "invalid".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.git_user_name = Some("John Doe".to_string());

        assert_derive_from_source("author", config, context, None);
    }

    #[test]
    fn test_derive_from_git_user_missing_name() {
        let config = DerivedFieldConfig {
            from: DerivationSource::GitUser,
            pattern: "name".to_string(),
            validate: None,
        };
        let context = DerivationContext::new(); // No git_user_name

        assert_derive_from_source("author", config, context, None);
    }

    // =========================================================================
    // GRACEFUL FAILURE TESTS
    // =========================================================================

    #[test]
    fn test_invalid_regex_pattern() {
        let mut derived = HashMap::new();
        derived.insert(
            "test".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: "[invalid regex".to_string(), // Invalid regex
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("test".to_string());

        let result = engine.derive_fields(&context);
        // Invalid regex should result in None (graceful failure)
        assert!(!result.contains_key("test"));
    }

    // =========================================================================
    // EMPTY CONFIG TEST
    // =========================================================================

    #[test]
    fn test_empty_config_returns_empty_map() {
        let engine = create_test_engine(HashMap::new());
        let mut context = DerivationContext::new();
        context.branch_name = Some("main".to_string());
        context.spec_path = Some(PathBuf::from(".chant/specs/test.md"));
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "value".to_string());
        context.env_vars = env_vars;
        context.git_user_name = Some("Test User".to_string());

        let result = engine.derive_fields(&context);
        assert!(result.is_empty());
    }

    // =========================================================================
    // MULTIPLE FIELDS TEST
    // =========================================================================

    #[test]
    fn test_derive_multiple_fields() {
        let mut derived = HashMap::new();
        derived.insert(
            "env".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "name".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("prod/feature".to_string());
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "platform".to_string());
        context.env_vars = env_vars;
        context.git_user_name = Some("Jane Doe".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.len(), 3);
        assert_eq!(result.get("env"), Some(&"prod".to_string()));
        assert_eq!(result.get("team"), Some(&"platform".to_string()));
        assert_eq!(result.get("author"), Some(&"Jane Doe".to_string()));
    }

    #[test]
    fn test_derive_multiple_fields_partial_success() {
        let mut derived = HashMap::new();
        derived.insert(
            "env".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "MISSING_VAR".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "name".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("prod/feature".to_string());
        context.git_user_name = Some("Jane Doe".to_string());

        let result = engine.derive_fields(&context);
        // Only env and author should be derived
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("env"), Some(&"prod".to_string()));
        assert!(!result.contains_key("team"));
        assert_eq!(result.get("author"), Some(&"Jane Doe".to_string()));
    }

    // =========================================================================
    // VALIDATION TESTS
    // =========================================================================

    #[test]
    fn test_enum_validation_valid_value() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec![
                        "platform".to_string(),
                        "frontend".to_string(),
                        "backend".to_string(),
                    ],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "platform".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        // Field should be included even with validation
        assert_eq!(result.get("team"), Some(&"platform".to_string()));
    }

    #[test]
    fn test_enum_validation_invalid_value() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec![
                        "platform".to_string(),
                        "frontend".to_string(),
                        "backend".to_string(),
                    ],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "invalid-team".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        // Field should still be included even if validation fails
        assert_eq!(result.get("team"), Some(&"invalid-team".to_string()));
    }

    #[test]
    fn test_enum_validation_with_branch_source() {
        let mut derived = HashMap::new();
        derived.insert(
            "environment".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec!["dev".to_string(), "staging".to_string(), "prod".to_string()],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("staging/new-feature".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("environment"), Some(&"staging".to_string()));
    }

    #[test]
    fn test_enum_validation_with_branch_source_invalid() {
        let mut derived = HashMap::new();
        derived.insert(
            "environment".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^([a-z]+)".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec!["dev".to_string(), "staging".to_string(), "prod".to_string()],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("testing/new-feature".to_string());

        let result = engine.derive_fields(&context);
        // Value should still be included even though "testing" is not in enum
        assert_eq!(result.get("environment"), Some(&"testing".to_string()));
    }

    #[test]
    fn test_validation_skipped_when_no_rule_configured() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: None, // No validation rule
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "any-value".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        // Field should be included without validation
        assert_eq!(result.get("team"), Some(&"any-value".to_string()));
    }

    #[test]
    fn test_enum_validation_with_path_source() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Path,
                pattern: r"specs/([a-z]+)/".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec![
                        "platform".to_string(),
                        "frontend".to_string(),
                        "backend".to_string(),
                    ],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/backend/feature.md"));

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("team"), Some(&"backend".to_string()));
    }

    #[test]
    fn test_enum_validation_case_sensitive() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec!["Platform".to_string(), "Frontend".to_string()],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "platform".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        // "platform" does not match "Platform" (case sensitive)
        // Field should still be included
        assert_eq!(result.get("team"), Some(&"platform".to_string()));
    }

    #[test]
    fn test_multiple_fields_with_mixed_validation() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec!["platform".to_string(), "frontend".to_string()],
                }),
            },
        );
        derived.insert(
            "environment".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: None, // No validation
            },
        );
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "name".to_string(),
                validate: Some(ValidationRule::Enum {
                    values: vec!["Alice".to_string(), "Bob".to_string()],
                }),
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "backend".to_string()); // Invalid
        context.env_vars = env_vars;
        context.branch_name = Some("prod/feature".to_string());
        context.git_user_name = Some("Charlie".to_string()); // Invalid

        let result = engine.derive_fields(&context);
        // All three should be included despite validation warnings
        assert_eq!(result.len(), 3);
        assert_eq!(result.get("team"), Some(&"backend".to_string()));
        assert_eq!(result.get("environment"), Some(&"prod".to_string()));
        assert_eq!(result.get("author"), Some(&"Charlie".to_string()));
    }

    // =========================================================================
    // UNICODE HANDLING TESTS
    // =========================================================================

    #[test]
    fn test_branch_with_unicode_characters() {
        let mut derived = HashMap::new();
        derived.insert(
            "project".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: "feature/([^/]+)/".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "description".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: "feature/[^/]+/(.+)".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("feature/项目-123/amélioration".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("project"), Some(&"项目-123".to_string()));
        assert_eq!(result.get("description"), Some(&"amélioration".to_string()));
    }

    #[test]
    fn test_env_value_with_unicode() {
        let mut derived = HashMap::new();
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "AUTHOR_NAME".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "desc".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "DESCRIPTION".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("AUTHOR_NAME".to_string(), "José García".to_string());
        env_vars.insert("TEAM_NAME".to_string(), "Платформа".to_string());
        env_vars.insert("DESCRIPTION".to_string(), "Fix 🐛 in parser".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("author"), Some(&"José García".to_string()));
        assert_eq!(result.get("team"), Some(&"Платформа".to_string()));
        assert_eq!(result.get("desc"), Some(&"Fix 🐛 in parser".to_string()));
    }

    #[test]
    fn test_git_user_with_unicode() {
        let mut derived = HashMap::new();
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "name".to_string(),
                validate: None,
            },
        );
        derived.insert(
            "email".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "email".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.git_user_name = Some("François Müller".to_string());
        context.git_user_email = Some("françois.müller@example.com".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("author"), Some(&"François Müller".to_string()));
        assert_eq!(
            result.get("email"),
            Some(&"françois.müller@example.com".to_string())
        );
    }

    #[test]
    fn test_path_with_unicode_directory_names() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Path,
                pattern: "specs/([^/]+)/".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/平台/文档.md"));

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("team"), Some(&"平台".to_string()));
    }

    // =========================================================================
    // SPECIAL CHARACTERS IN VALUES TESTS
    // =========================================================================

    #[test]
    fn test_special_characters_branch_with_slashes_hyphens_dots() {
        let config1 = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: "([A-Z]+-\\d+)".to_string(),
            validate: None,
        };
        let config2 = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: "feature/(.+)".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.branch_name = Some("feature/ABC-123/user-name.test".to_string());

        assert_derive_from_source("ticket", config1, context.clone(), Some("ABC-123"));
        assert_derive_from_source(
            "full_path",
            config2,
            context,
            Some("ABC-123/user-name.test"),
        );
    }

    #[test]
    fn test_special_characters_env_value_with_spaces_and_quotes() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "Platform Team".to_string());
        env_vars.insert(
            "DESCRIPTION".to_string(),
            "This is a \"test\" value".to_string(),
        );
        env_vars.insert(
            "NOTES".to_string(),
            "Value with 'single' and \"double\" quotes".to_string(),
        );

        let config1 = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "TEAM_NAME".to_string(),
            validate: None,
        };
        let config2 = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "DESCRIPTION".to_string(),
            validate: None,
        };
        let config3 = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "NOTES".to_string(),
            validate: None,
        };
        let context = DerivationContext::with_env_vars(env_vars);

        assert_derive_from_source("team", config1, context.clone(), Some("Platform Team"));
        assert_derive_from_source(
            "desc",
            config2,
            context.clone(),
            Some("This is a \"test\" value"),
        );
        assert_derive_from_source(
            "notes",
            config3,
            context,
            Some("Value with 'single' and \"double\" quotes"),
        );
    }

    #[test]
    fn test_special_characters_path_with_dots_and_hyphens() {
        let config1 = DerivedFieldConfig {
            from: DerivationSource::Path,
            pattern: "specs/([^/]+)/".to_string(),
            validate: None,
        };
        let config2 = DerivedFieldConfig {
            from: DerivationSource::Path,
            pattern: "/([^/]+\\.md)$".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/platform-team/feature.v2.md"));

        assert_derive_from_source("component", config1, context.clone(), Some("platform-team"));
        assert_derive_from_source("filename", config2, context, Some("feature.v2.md"));
    }

    #[test]
    fn test_special_characters_value_with_regex_metacharacters() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: "feature/(.+)".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.branch_name = Some("feature/fix-[bug]-in-(parser)".to_string());

        assert_derive_from_source(
            "description",
            config,
            context,
            Some("fix-[bug]-in-(parser)"),
        );
    }

    #[test]
    fn test_special_characters_env_value_with_commas_and_special_chars() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TAGS".to_string(), "bug,feature,urgent".to_string());
        env_vars.insert("EXPRESSION".to_string(), "value = 1 + 2 * 3".to_string());
        env_vars.insert(
            "PATH_LIKE".to_string(),
            "/usr/bin:/usr/local/bin".to_string(),
        );

        let config1 = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "TAGS".to_string(),
            validate: None,
        };
        let config2 = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "EXPRESSION".to_string(),
            validate: None,
        };
        let config3 = DerivedFieldConfig {
            from: DerivationSource::Env,
            pattern: "PATH_LIKE".to_string(),
            validate: None,
        };
        let context = DerivationContext::with_env_vars(env_vars);

        assert_derive_from_source("tags", config1, context.clone(), Some("bug,feature,urgent"));
        assert_derive_from_source("expr", config2, context.clone(), Some("value = 1 + 2 * 3"));
        assert_derive_from_source("path", config3, context, Some("/usr/bin:/usr/local/bin"));
    }

    #[test]
    fn test_special_characters_branch_with_multiple_regex_metacharacters() {
        let config = DerivedFieldConfig {
            from: DerivationSource::Branch,
            pattern: "fix/(.+)".to_string(),
            validate: None,
        };
        let mut context = DerivationContext::new();
        context.branch_name = Some("fix/handle-$var.and^chars+more*stuff".to_string());

        assert_derive_from_source(
            "desc",
            config,
            context,
            Some("handle-$var.and^chars+more*stuff"),
        );
    }
}
