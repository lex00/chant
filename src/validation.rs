//! Output schema validation for spec agent outputs.
//!
//! This module provides JSON Schema validation for agent outputs when specs
//! define an `output_schema` field in their frontmatter.
//!
//! # Doc Audit
//! - audited: 2026-01-29
//! - docs: reference/schema.md
//! - ignore: false

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Result of validating agent output against a JSON schema
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether the validation passed
    pub is_valid: bool,
    /// List of validation errors (empty if valid)
    pub errors: Vec<String>,
    /// The JSON that was extracted and validated (if any)
    pub extracted_json: Option<serde_json::Value>,
}

/// Extract JSON from agent output text.
///
/// Tries multiple strategies:
/// 1. Look for ```json code blocks
/// 2. Look for bare ``` code blocks that contain JSON
/// 3. Try parsing the entire output as JSON
/// 4. Find JSON object/array patterns in the text
pub fn extract_json_from_output(output: &str) -> Option<serde_json::Value> {
    // Strategy 1: Look for ```json code blocks
    if let Some(json) = extract_json_code_block(output, "json") {
        return Some(json);
    }

    // Strategy 2: Look for bare ``` code blocks
    if let Some(json) = extract_json_code_block(output, "") {
        return Some(json);
    }

    // Strategy 3: Try parsing the entire output as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output.trim()) {
        return Some(json);
    }

    // Strategy 4: Find JSON object/array patterns
    if let Some(json) = find_json_in_text(output) {
        return Some(json);
    }

    None
}

/// Extract JSON from a fenced code block with optional language specifier
fn extract_json_code_block(output: &str, lang: &str) -> Option<serde_json::Value> {
    let mut in_fence = false;
    let mut fence_content = String::new();
    let mut fence_lang = String::new();

    for line in output.lines() {
        let trimmed = line.trim_start();
        if let Some(after_fence) = trimmed.strip_prefix("```") {
            if in_fence {
                // End of fence
                in_fence = false;
                // Check if this is the right language (or any if lang is empty)
                if lang.is_empty()
                    || fence_lang.is_empty()
                    || fence_lang.to_lowercase() == lang.to_lowercase()
                {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&fence_content) {
                        return Some(json);
                    }
                }
                fence_content.clear();
                fence_lang.clear();
            } else {
                // Start of fence
                in_fence = true;
                fence_lang = after_fence.trim().to_string();
            }
        } else if in_fence {
            if !fence_content.is_empty() {
                fence_content.push('\n');
            }
            fence_content.push_str(line);
        }
    }

    // Handle unclosed fence (try what we have)
    if in_fence
        && !fence_content.is_empty()
        && (lang.is_empty()
            || fence_lang.is_empty()
            || fence_lang.to_lowercase() == lang.to_lowercase())
    {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&fence_content) {
            return Some(json);
        }
    }

    None
}

/// Find JSON object or array patterns in text
fn find_json_in_text(text: &str) -> Option<serde_json::Value> {
    // Look for { ... } patterns (objects)
    let mut brace_depth = 0;
    let mut start_idx = None;

    for (idx, ch) in text.char_indices() {
        match ch {
            '{' => {
                if brace_depth == 0 {
                    start_idx = Some(idx);
                }
                brace_depth += 1;
            }
            '}' => {
                brace_depth -= 1;
                if brace_depth == 0 {
                    if let Some(start) = start_idx {
                        let candidate = &text[start..=idx];
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(candidate) {
                            return Some(json);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Look for [ ... ] patterns (arrays)
    let mut bracket_depth = 0;
    start_idx = None;

    for (idx, ch) in text.char_indices() {
        match ch {
            '[' => {
                if bracket_depth == 0 {
                    start_idx = Some(idx);
                }
                bracket_depth += 1;
            }
            ']' => {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    if let Some(start) = start_idx {
                        let candidate = &text[start..=idx];
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(candidate) {
                            return Some(json);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    None
}

/// Load and compile a JSON schema from a file path
pub fn load_schema(schema_path: &Path) -> Result<jsonschema::Validator> {
    let schema_content = fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema file: {}", schema_path.display()))?;

    let schema: serde_json::Value = serde_json::from_str(&schema_content)
        .with_context(|| format!("Failed to parse schema as JSON: {}", schema_path.display()))?;

    jsonschema::validator_for(&schema)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))
}

/// Validate agent output against a JSON schema file.
///
/// # Arguments
/// * `spec_id` - The spec ID (for error messages)
/// * `schema_path` - Path to the JSON schema file
/// * `agent_output` - The raw output from the agent
///
/// # Returns
/// * `ValidationResult` with validation status and any errors
pub fn validate_agent_output(
    spec_id: &str,
    schema_path: &Path,
    agent_output: &str,
) -> Result<ValidationResult> {
    // Load and compile the schema
    let validator = load_schema(schema_path)?;

    // Extract JSON from the agent output
    let extracted_json = match extract_json_from_output(agent_output) {
        Some(json) => json,
        None => {
            return Ok(ValidationResult {
                is_valid: false,
                errors: vec![format!(
                    "No JSON found in agent output for spec '{}'",
                    spec_id
                )],
                extracted_json: None,
            });
        }
    };

    // Validate the extracted JSON against the schema using iter_errors
    // to collect all validation errors
    let error_iter = validator.iter_errors(&extracted_json);
    let error_messages: Vec<String> = error_iter
        .map(|e| {
            let path = e.instance_path.to_string();
            if path.is_empty() {
                e.to_string()
            } else {
                format!("at '{}': {}", path, e)
            }
        })
        .collect();

    if error_messages.is_empty() {
        Ok(ValidationResult {
            is_valid: true,
            errors: vec![],
            extracted_json: Some(extracted_json),
        })
    } else {
        Ok(ValidationResult {
            is_valid: false,
            errors: error_messages,
            extracted_json: Some(extracted_json),
        })
    }
}

/// Read the agent log file for a spec and validate its output against the schema.
///
/// This is useful for batch validation (e.g., in `chant lint`).
///
/// # Arguments
/// * `spec_id` - The spec ID
/// * `schema_path` - Path to the JSON schema file
/// * `logs_dir` - Path to the logs directory (typically `.chant/logs`)
///
/// # Returns
/// * `Ok(Some(ValidationResult))` if log exists and validation was attempted
/// * `Ok(None)` if no log file exists for this spec
/// * `Err` if there was an error reading the log or schema
pub fn validate_spec_output_from_log(
    spec_id: &str,
    schema_path: &Path,
    logs_dir: &Path,
) -> Result<Option<ValidationResult>> {
    let log_path = logs_dir.join(format!("{}.log", spec_id));

    if !log_path.exists() {
        return Ok(None);
    }

    let log_content = fs::read_to_string(&log_path)
        .with_context(|| format!("Failed to read log file: {}", log_path.display()))?;

    let result = validate_agent_output(spec_id, schema_path, &log_content)?;
    Ok(Some(result))
}

/// Generate an "Output Format" prompt section from a JSON schema.
///
/// This is injected into agent prompts when a spec has an `output_schema` field.
pub fn generate_schema_prompt_section(schema_path: &Path) -> Result<String> {
    let schema_content = fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema file: {}", schema_path.display()))?;

    let schema: serde_json::Value = serde_json::from_str(&schema_content)
        .with_context(|| format!("Failed to parse schema as JSON: {}", schema_path.display()))?;

    let mut section = String::new();
    section.push_str("\n## Output Format\n\n");
    section.push_str("Your output MUST include valid JSON matching this schema:\n\n");
    section.push_str("```json\n");
    section.push_str(&serde_json::to_string_pretty(&schema)?);
    section.push_str("\n```\n\n");

    // Extract required fields if present
    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        let required_fields: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
        if !required_fields.is_empty() {
            section.push_str(&format!(
                "**Required fields:** {}\n\n",
                required_fields.join(", ")
            ));
        }
    }

    // Generate an example if properties are defined
    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        section.push_str("**Example:**\n\n```json\n");
        let example = generate_example_from_properties(properties, &schema);
        section.push_str(&serde_json::to_string_pretty(&example)?);
        section.push_str("\n```\n");
    }

    Ok(section)
}

/// Generate an example JSON object from schema properties
fn generate_example_from_properties(
    properties: &serde_json::Map<String, serde_json::Value>,
    schema: &serde_json::Value,
) -> serde_json::Value {
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut example = serde_json::Map::new();

    for (key, prop_schema) in properties {
        // Only include required fields in example, or first 3 properties
        if !required.contains(&key.as_str()) && example.len() >= 3 {
            continue;
        }

        let value = generate_example_value(prop_schema);
        example.insert(key.clone(), value);
    }

    serde_json::Value::Object(example)
}

/// Generate an example value for a schema property
fn generate_example_value(prop_schema: &serde_json::Value) -> serde_json::Value {
    let prop_type = prop_schema.get("type").and_then(|t| t.as_str());

    match prop_type {
        Some("string") => {
            // Check for pattern or enum
            if let Some(enum_values) = prop_schema.get("enum").and_then(|e| e.as_array()) {
                if let Some(first) = enum_values.first() {
                    return first.clone();
                }
            }
            serde_json::Value::String("...".to_string())
        }
        Some("number") | Some("integer") => serde_json::Value::Number(0.into()),
        Some("boolean") => serde_json::Value::Bool(true),
        Some("array") => {
            let items_schema = prop_schema.get("items");
            let item_example = items_schema
                .map(generate_example_value)
                .unwrap_or(serde_json::Value::String("...".to_string()));
            serde_json::Value::Array(vec![item_example])
        }
        Some("object") => {
            if let Some(props) = prop_schema.get("properties").and_then(|p| p.as_object()) {
                generate_example_from_properties(props, prop_schema)
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            }
        }
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_json_from_code_block() {
        let output = r#"
Here is the analysis:

```json
{
  "spec_id": "C.1.1",
  "findings": ["Found issue A", "Found issue B"],
  "recommendation": "Fix the bug"
}
```

That's my report.
"#;

        let json = extract_json_from_output(output).unwrap();
        assert_eq!(json["spec_id"], "C.1.1");
        assert!(json["findings"].is_array());
    }

    #[test]
    fn test_extract_json_bare_output() {
        let output = r#"{"spec_id": "test", "value": 42}"#;

        let json = extract_json_from_output(output).unwrap();
        assert_eq!(json["spec_id"], "test");
        assert_eq!(json["value"], 42);
    }

    #[test]
    fn test_extract_json_embedded_in_text() {
        let output = r#"
The analysis shows that the result is:
{"status": "success", "count": 5}
End of report.
"#;

        let json = extract_json_from_output(output).unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["count"], 5);
    }

    #[test]
    fn test_extract_json_no_json() {
        let output = "This is just plain text without any JSON content.";
        assert!(extract_json_from_output(output).is_none());
    }

    #[test]
    fn test_validate_valid_output() {
        let tmp = TempDir::new().unwrap();
        let schema_path = tmp.path().join("schema.json");

        let schema = r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["spec_id", "status"],
            "properties": {
                "spec_id": {"type": "string"},
                "status": {"type": "string", "enum": ["success", "failure"]}
            }
        }"#;
        fs::write(&schema_path, schema).unwrap();

        let agent_output = r#"
Here is my report:
```json
{"spec_id": "test-001", "status": "success"}
```
"#;

        let result = validate_agent_output("test-001", &schema_path, agent_output).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_output() {
        let tmp = TempDir::new().unwrap();
        let schema_path = tmp.path().join("schema.json");

        let schema = r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["spec_id"],
            "properties": {
                "spec_id": {"type": "string"}
            }
        }"#;
        fs::write(&schema_path, schema).unwrap();

        // Missing required field
        let agent_output = r#"{"status": "done"}"#;

        let result = validate_agent_output("test-001", &schema_path, agent_output).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_no_json_in_output() {
        let tmp = TempDir::new().unwrap();
        let schema_path = tmp.path().join("schema.json");

        let schema = r#"{
            "type": "object",
            "properties": {"x": {"type": "string"}}
        }"#;
        fs::write(&schema_path, schema).unwrap();

        let agent_output = "Just some plain text, no JSON here.";

        let result = validate_agent_output("test-001", &schema_path, agent_output).unwrap();
        assert!(!result.is_valid);
        assert!(result.errors[0].contains("No JSON found"));
    }

    #[test]
    fn test_generate_schema_prompt_section() {
        let tmp = TempDir::new().unwrap();
        let schema_path = tmp.path().join("schema.json");

        let schema = r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["spec_id", "findings"],
            "properties": {
                "spec_id": {"type": "string"},
                "findings": {"type": "array", "items": {"type": "string"}}
            }
        }"#;
        fs::write(&schema_path, schema).unwrap();

        let section = generate_schema_prompt_section(&schema_path).unwrap();

        assert!(section.contains("## Output Format"));
        assert!(section.contains("spec_id"));
        assert!(section.contains("Required fields"));
        assert!(section.contains("Example"));
    }
}
