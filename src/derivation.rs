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

use crate::config::{DerivationSource, DerivedFieldConfig, EnterpriseConfig};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;

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
    fn derive_field(
        &self,
        field_name: &str,
        config: &DerivedFieldConfig,
        context: &DerivationContext,
    ) -> Option<String> {
        match config.from {
            DerivationSource::Branch => {
                let source_value = self.extract_from_branch(context)?;
                self.apply_pattern(&config.pattern, &source_value)
                    .or_else(|| {
                        eprintln!(
                            "Warning: derivation pattern for field '{}' did not match source",
                            field_name
                        );
                        None
                    })
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
                    })
            }
            DerivationSource::Env => {
                // For Env, pattern is the environment variable name
                self.extract_from_env(context, &config.pattern)
            }
            DerivationSource::GitUser => {
                // For GitUser, pattern is "name" or "email"
                self.extract_from_git_user(context, &config.pattern)
            }
        }
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
    // BRANCH NAME EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_branch_basic() {
        let mut derived = HashMap::new();
        derived.insert(
            "env".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("prod/feature-123".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("env"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_derive_from_branch_with_capture_group() {
        let mut derived = HashMap::new();
        derived.insert(
            "project".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"sprint/.*/(PROJ-\d+)".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("sprint/2026-Q1-W4/PROJ-123".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("project"), Some(&"PROJ-123".to_string()));
    }

    #[test]
    fn test_derive_from_branch_no_match() {
        let mut derived = HashMap::new();
        derived.insert(
            "env".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.branch_name = Some("feature/my-branch".to_string());

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("env"));
    }

    #[test]
    fn test_derive_from_branch_missing() {
        let mut derived = HashMap::new();
        derived.insert(
            "env".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Branch,
                pattern: r"^(dev|staging|prod)".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let context = DerivationContext::new(); // No branch_name

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("env"));
    }

    // =========================================================================
    // FILE PATH EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_path_basic() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Path,
                pattern: r"specs/([a-z]+)/".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/platform/feature.md"));

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("team"), Some(&"platform".to_string()));
    }

    #[test]
    fn test_derive_from_path_with_multiple_captures() {
        let mut derived = HashMap::new();
        derived.insert(
            "project".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Path,
                pattern: r"specs/([a-z]+)/([A-Z0-9]+)-".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/teams/PROJ-123-feature.md"));

        let result = engine.derive_fields(&context);
        // Should extract first capture group only
        assert_eq!(result.get("project"), Some(&"teams".to_string()));
    }

    #[test]
    fn test_derive_from_path_no_match() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Path,
                pattern: r"specs/([a-z]+)/".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.spec_path = Some(PathBuf::from(".chant/specs/feature.md"));

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("team"));
    }

    #[test]
    fn test_derive_from_path_missing() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Path,
                pattern: r"specs/([a-z]+)/".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let context = DerivationContext::new(); // No spec_path

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("team"));
    }

    // =========================================================================
    // ENVIRONMENT VARIABLE EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_env_basic() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("TEAM_NAME".to_string(), "platform".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("team"), Some(&"platform".to_string()));
    }

    #[test]
    fn test_derive_from_env_with_pattern_match() {
        let mut derived = HashMap::new();
        derived.insert(
            "env_name".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "ENVIRONMENT".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("ENVIRONMENT".to_string(), "production".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("env_name"), Some(&"production".to_string()));
    }

    #[test]
    fn test_derive_from_env_missing_variable() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let context = DerivationContext::new(); // No env vars

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("team"));
    }

    #[test]
    fn test_derive_from_env_undefined_variable() {
        let mut derived = HashMap::new();
        derived.insert(
            "team".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::Env,
                pattern: "TEAM_NAME".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut env_vars = HashMap::new();
        env_vars.insert("OTHER_VAR".to_string(), "value".to_string());
        let context = DerivationContext::with_env_vars(env_vars);

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("team"));
    }

    // =========================================================================
    // GIT USER EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_derive_from_git_user_name() {
        let mut derived = HashMap::new();
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
        context.git_user_name = Some("John Doe".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(result.get("author"), Some(&"John Doe".to_string()));
    }

    #[test]
    fn test_derive_from_git_user_email() {
        let mut derived = HashMap::new();
        derived.insert(
            "author_email".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "email".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.git_user_email = Some("john@example.com".to_string());

        let result = engine.derive_fields(&context);
        assert_eq!(
            result.get("author_email"),
            Some(&"john@example.com".to_string())
        );
    }

    #[test]
    fn test_derive_from_git_user_invalid_field() {
        let mut derived = HashMap::new();
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "invalid".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let mut context = DerivationContext::new();
        context.git_user_name = Some("John Doe".to_string());

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("author"));
    }

    #[test]
    fn test_derive_from_git_user_missing_name() {
        let mut derived = HashMap::new();
        derived.insert(
            "author".to_string(),
            DerivedFieldConfig {
                from: DerivationSource::GitUser,
                pattern: "name".to_string(),
                validate: None,
            },
        );

        let engine = create_test_engine(derived);
        let context = DerivationContext::new(); // No git_user_name

        let result = engine.derive_fields(&context);
        assert!(!result.contains_key("author"));
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
}
