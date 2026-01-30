//! Spec template system for creating specs from reusable templates.
//!
//! Templates are markdown files with YAML frontmatter containing variable definitions.
//! They can be stored in `.chant/templates/` (project) or `~/.config/chant/templates/` (global).
//!
//! # Template Format
//!
//! ```markdown
//! ---
//! name: add-feature
//! description: Add a new feature with tests
//! variables:
//!   - name: feature_name
//!     description: Name of the feature
//!     required: true
//!   - name: module
//!     description: Target module
//!     default: core
//! type: code
//! labels:
//!   - feature
//! ---
//!
//! # Add {{feature_name}} feature
//!
//! ## Problem
//!
//! The {{module}} module needs {{feature_name}} functionality.
//! ```

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory name for templates within .chant/
pub const PROJECT_TEMPLATES_DIR: &str = ".chant/templates";

/// Directory name for global templates
pub const GLOBAL_TEMPLATES_DIR: &str = "templates";

/// A variable definition within a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Name of the variable (used in {{name}} placeholders)
    pub name: String,
    /// Description of what this variable is for
    #[serde(default)]
    pub description: String,
    /// Whether this variable must be provided (no default)
    #[serde(default)]
    pub required: bool,
    /// Default value if not provided
    #[serde(default)]
    pub default: Option<String>,
}

/// Template frontmatter containing metadata and variable definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFrontmatter {
    /// Template name (identifier)
    pub name: String,
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Variable definitions
    #[serde(default)]
    pub variables: Vec<TemplateVariable>,
    /// Default spec type to use
    #[serde(default)]
    pub r#type: Option<String>,
    /// Default labels to apply
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Default target files
    #[serde(default)]
    pub target_files: Option<Vec<String>>,
    /// Default context files
    #[serde(default)]
    pub context: Option<Vec<String>>,
    /// Default prompt to use
    #[serde(default)]
    pub prompt: Option<String>,
}

/// A spec template with its metadata and content
#[derive(Debug, Clone)]
pub struct SpecTemplate {
    /// Template name
    pub name: String,
    /// Parsed frontmatter
    pub frontmatter: TemplateFrontmatter,
    /// Template body (with {{variable}} placeholders)
    pub body: String,
    /// Source location (project or global)
    pub source: TemplateSource,
    /// File path where template was loaded from
    pub path: PathBuf,
}

/// Where a template was loaded from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    /// From project's .chant/templates/
    Project,
    /// From ~/.config/chant/templates/
    Global,
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::Project => write!(f, "project"),
            TemplateSource::Global => write!(f, "global"),
        }
    }
}

impl SpecTemplate {
    /// Parse a template from file content.
    pub fn parse(content: &str, path: &Path, source: TemplateSource) -> Result<Self> {
        let (frontmatter_str, body) = split_frontmatter(content);

        let frontmatter: TemplateFrontmatter = if let Some(fm) = frontmatter_str {
            serde_yaml::from_str(&fm).context("Failed to parse template frontmatter")?
        } else {
            anyhow::bail!("Template must have YAML frontmatter with 'name' field");
        };

        if frontmatter.name.is_empty() {
            anyhow::bail!("Template 'name' field is required and cannot be empty");
        }

        Ok(Self {
            name: frontmatter.name.clone(),
            frontmatter,
            body: body.to_string(),
            source,
            path: path.to_path_buf(),
        })
    }

    /// Load a template from a file path.
    pub fn load(path: &Path, source: TemplateSource) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template from {}", path.display()))?;
        Self::parse(&content, path, source)
    }

    /// Get list of required variables that don't have defaults
    pub fn required_variables(&self) -> Vec<&TemplateVariable> {
        self.frontmatter
            .variables
            .iter()
            .filter(|v| v.required && v.default.is_none())
            .collect()
    }

    /// Check if all required variables are provided
    pub fn validate_variables(&self, provided: &HashMap<String, String>) -> Result<()> {
        let missing: Vec<_> = self
            .required_variables()
            .iter()
            .filter(|v| !provided.contains_key(&v.name))
            .map(|v| v.name.as_str())
            .collect();

        if !missing.is_empty() {
            anyhow::bail!("Missing required variable(s): {}", missing.join(", "));
        }

        Ok(())
    }

    /// Substitute variables in a string using {{variable}} syntax
    pub fn substitute(&self, text: &str, variables: &HashMap<String, String>) -> String {
        let re = Regex::new(r"\{\{(\w+)\}\}").unwrap();

        re.replace_all(text, |caps: &regex::Captures| {
            let var_name = &caps[1];

            // First check provided variables
            if let Some(value) = variables.get(var_name) {
                return value.clone();
            }

            // Then check for defaults in template definition
            if let Some(var_def) = self
                .frontmatter
                .variables
                .iter()
                .find(|v| v.name == var_name)
            {
                if let Some(ref default) = var_def.default {
                    return default.clone();
                }
            }

            // Keep the placeholder if no value found
            caps[0].to_string()
        })
        .to_string()
    }

    /// Generate spec content from this template with the given variables
    pub fn render(&self, variables: &HashMap<String, String>) -> Result<String> {
        // Validate required variables are present
        self.validate_variables(variables)?;

        // Build frontmatter for the spec
        let mut fm_lines = vec!["---".to_string()];

        // Type (from template or default to 'code')
        let spec_type = self.frontmatter.r#type.as_deref().unwrap_or("code");
        fm_lines.push(format!("type: {}", spec_type));
        fm_lines.push("status: pending".to_string());

        // Labels
        if let Some(ref labels) = self.frontmatter.labels {
            if !labels.is_empty() {
                fm_lines.push("labels:".to_string());
                for label in labels {
                    let substituted = self.substitute(label, variables);
                    fm_lines.push(format!("  - {}", substituted));
                }
            }
        }

        // Target files
        if let Some(ref target_files) = self.frontmatter.target_files {
            if !target_files.is_empty() {
                fm_lines.push("target_files:".to_string());
                for file in target_files {
                    let substituted = self.substitute(file, variables);
                    fm_lines.push(format!("  - {}", substituted));
                }
            }
        }

        // Context
        if let Some(ref context) = self.frontmatter.context {
            if !context.is_empty() {
                fm_lines.push("context:".to_string());
                for ctx in context {
                    let substituted = self.substitute(ctx, variables);
                    fm_lines.push(format!("  - {}", substituted));
                }
            }
        }

        // Prompt
        if let Some(ref prompt) = self.frontmatter.prompt {
            fm_lines.push(format!("prompt: {}", prompt));
        }

        fm_lines.push("---".to_string());
        fm_lines.push(String::new());

        let frontmatter = fm_lines.join("\n");

        // Substitute variables in body
        let body = self.substitute(&self.body, variables);

        Ok(format!("{}{}", frontmatter, body))
    }
}

/// Split content into frontmatter and body.
/// Returns (Some(frontmatter), body) if frontmatter exists, otherwise (None, full_content).
fn split_frontmatter(content: &str) -> (Option<String>, &str) {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return (None, content);
    }

    // Find the closing ---
    let after_first = &content[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let frontmatter = after_first[..end_pos].trim();
        let body_start = 3 + end_pos + 4; // "---" + frontmatter + "\n---"
        let body = if body_start < content.len() {
            content[body_start..].trim_start_matches('\n')
        } else {
            ""
        };
        (Some(frontmatter.to_string()), body)
    } else {
        (None, content)
    }
}

/// Get the path to the project templates directory
pub fn project_templates_dir() -> PathBuf {
    PathBuf::from(PROJECT_TEMPLATES_DIR)
}

/// Get the path to the global templates directory
pub fn global_templates_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("chant").join(GLOBAL_TEMPLATES_DIR))
}

/// Load all templates from a directory
fn load_templates_from_dir(dir: &Path, source: TemplateSource) -> Vec<SpecTemplate> {
    let mut templates = Vec::new();

    if !dir.exists() {
        return templates;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                match SpecTemplate::load(&path, source.clone()) {
                    Ok(template) => templates.push(template),
                    Err(e) => {
                        eprintln!("Warning: Failed to load template {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    templates
}

/// Load all available templates (project templates override global ones with the same name)
pub fn load_all_templates() -> Vec<SpecTemplate> {
    let mut templates_by_name: HashMap<String, SpecTemplate> = HashMap::new();

    // First load global templates
    if let Some(global_dir) = global_templates_dir() {
        for template in load_templates_from_dir(&global_dir, TemplateSource::Global) {
            templates_by_name.insert(template.name.clone(), template);
        }
    }

    // Then load project templates (overriding global ones with same name)
    let project_dir = project_templates_dir();
    for template in load_templates_from_dir(&project_dir, TemplateSource::Project) {
        templates_by_name.insert(template.name.clone(), template);
    }

    let mut templates: Vec<_> = templates_by_name.into_values().collect();
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    templates
}

/// Find a template by name (project templates take precedence)
pub fn find_template(name: &str) -> Result<SpecTemplate> {
    // Check project templates first
    let project_dir = project_templates_dir();
    let project_path = project_dir.join(format!("{}.md", name));
    if project_path.exists() {
        return SpecTemplate::load(&project_path, TemplateSource::Project);
    }

    // Check global templates
    if let Some(global_dir) = global_templates_dir() {
        let global_path = global_dir.join(format!("{}.md", name));
        if global_path.exists() {
            return SpecTemplate::load(&global_path, TemplateSource::Global);
        }
    }

    anyhow::bail!(
        "Template '{}' not found.\n\
         Searched in:\n  \
         - {}\n  \
         - {}",
        name,
        project_path.display(),
        global_templates_dir()
            .map(|p| p.join(format!("{}.md", name)).display().to_string())
            .unwrap_or_else(|| "~/.config/chant/templates/".to_string())
    );
}

/// Parse a list of "key=value" strings into a HashMap
pub fn parse_var_args(var_args: &[String]) -> Result<HashMap<String, String>> {
    let mut vars = HashMap::new();

    for arg in var_args {
        let parts: Vec<&str> = arg.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid variable format '{}'. Expected 'key=value'.", arg);
        }
        vars.insert(parts[0].to_string(), parts[1].to_string());
    }

    Ok(vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter() {
        let content = "---\nname: test\n---\n\n# Body\n";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_some());
        assert_eq!(fm.unwrap(), "name: test");
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn test_split_frontmatter_no_frontmatter() {
        let content = "# Just body\n";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(body, "# Just body\n");
    }

    #[test]
    fn test_parse_template() {
        let content = r#"---
name: test-template
description: A test template
variables:
  - name: feature
    description: Feature name
    required: true
  - name: module
    description: Module name
    default: core
type: code
labels:
  - feature
---

# Add {{feature}} to {{module}}

## Problem

Need to add {{feature}}.
"#;

        let template = SpecTemplate::parse(content, Path::new("test.md"), TemplateSource::Project)
            .expect("Should parse");
        assert_eq!(template.name, "test-template");
        assert_eq!(template.frontmatter.description, "A test template");
        assert_eq!(template.frontmatter.variables.len(), 2);
        assert!(template.frontmatter.variables[0].required);
        assert_eq!(
            template.frontmatter.variables[1].default,
            Some("core".to_string())
        );
    }

    #[test]
    fn test_substitute_variables() {
        let content = r#"---
name: test
variables:
  - name: x
    required: true
  - name: y
    default: default_y
---

Text with {{x}} and {{y}}.
"#;

        let template = SpecTemplate::parse(content, Path::new("test.md"), TemplateSource::Project)
            .expect("Should parse");

        let mut vars = HashMap::new();
        vars.insert("x".to_string(), "value_x".to_string());

        let result = template.substitute("{{x}} and {{y}}", &vars);
        assert_eq!(result, "value_x and default_y");
    }

    #[test]
    fn test_validate_variables() {
        let content = r#"---
name: test
variables:
  - name: required_var
    required: true
  - name: optional_var
    default: optional
---

Body
"#;

        let template = SpecTemplate::parse(content, Path::new("test.md"), TemplateSource::Project)
            .expect("Should parse");

        // Missing required variable
        let vars = HashMap::new();
        assert!(template.validate_variables(&vars).is_err());

        // With required variable
        let mut vars = HashMap::new();
        vars.insert("required_var".to_string(), "value".to_string());
        assert!(template.validate_variables(&vars).is_ok());
    }

    #[test]
    fn test_render_template() {
        let content = r#"---
name: feature
description: Add a feature
variables:
  - name: feature_name
    required: true
  - name: module
    default: core
type: code
labels:
  - feature
  - "{{module}}"
---

# Add {{feature_name}}

Implement {{feature_name}} in {{module}}.
"#;

        let template = SpecTemplate::parse(content, Path::new("test.md"), TemplateSource::Project)
            .expect("Should parse");

        let mut vars = HashMap::new();
        vars.insert("feature_name".to_string(), "logging".to_string());

        let rendered = template.render(&vars).expect("Should render");

        assert!(rendered.contains("type: code"));
        assert!(rendered.contains("status: pending"));
        assert!(rendered.contains("# Add logging"));
        assert!(rendered.contains("Implement logging in core."));
        assert!(rendered.contains("labels:"));
        assert!(rendered.contains("  - feature"));
        assert!(rendered.contains("  - core"));
    }

    #[test]
    fn test_parse_var_args() {
        let args = vec!["key1=value1".to_string(), "key2=value2".to_string()];
        let vars = parse_var_args(&args).expect("Should parse");
        assert_eq!(vars.get("key1"), Some(&"value1".to_string()));
        assert_eq!(vars.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_parse_var_args_with_equals_in_value() {
        let args = vec!["key=value=with=equals".to_string()];
        let vars = parse_var_args(&args).expect("Should parse");
        assert_eq!(vars.get("key"), Some(&"value=with=equals".to_string()));
    }

    #[test]
    fn test_parse_var_args_invalid() {
        let args = vec!["no_equals_sign".to_string()];
        assert!(parse_var_args(&args).is_err());
    }

    #[test]
    fn test_required_variables() {
        let content = r#"---
name: test
variables:
  - name: req1
    required: true
  - name: req2
    required: true
    default: has_default
  - name: opt1
    required: false
---

Body
"#;

        let template = SpecTemplate::parse(content, Path::new("test.md"), TemplateSource::Project)
            .expect("Should parse");

        let required = template.required_variables();
        // req1 is required with no default
        // req2 has required: true but also has a default, so it shouldn't count
        // opt1 is not required
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].name, "req1");
    }
}
