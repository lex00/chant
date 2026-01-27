//! Export command for exporting specs to various formats
//!
//! Supports JSON, CSV, and Markdown formats with flexible filtering options
//! including status, type, labels, and date ranges.

use anyhow::{Context, Result};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use chant::spec::{self, Spec};

/// Main export command handler
pub fn cmd_export(
    format: &str,
    statuses: &[String],
    type_filter: Option<&str>,
    labels: &[String],
    ready_only: bool,
    from_date: Option<&str>,
    to_date: Option<&str>,
    fields: Option<&str>,
    output_file: Option<&str>,
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load all specs
    let mut specs = spec::load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    // Apply filters
    apply_filters(
        &mut specs,
        statuses,
        type_filter,
        labels,
        ready_only,
        from_date,
        to_date,
    )?;

    if specs.is_empty() {
        let output = "No specs match the specified filters.";
        if let Some(file_path) = output_file {
            let mut file = File::create(file_path).context("Failed to create output file")?;
            writeln!(file, "{}", output).context("Failed to write to output file")?;
        } else {
            println!("{}", output);
        }
        return Ok(());
    }

    // Generate output based on format
    let output = match format.to_lowercase().as_str() {
        "json" => export_json(&specs, fields)?,
        "csv" => export_csv(&specs, fields)?,
        "markdown" | "md" => export_markdown(&specs, fields)?,
        _ => anyhow::bail!(
            "Unknown format: {}. Supported formats: json, csv, markdown",
            format
        ),
    };

    // Write output
    if let Some(file_path) = output_file {
        let mut file = File::create(file_path).context("Failed to create output file")?;
        write!(file, "{}", output).context("Failed to write to output file")?;
        println!("Export written to: {}", file_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

/// Apply all filters to the specs list
fn apply_filters(
    specs: &mut Vec<Spec>,
    statuses: &[String],
    type_filter: Option<&str>,
    labels: &[String],
    ready_only: bool,
    from_date: Option<&str>,
    to_date: Option<&str>,
) -> Result<()> {
    // Filter by status
    if !statuses.is_empty() {
        specs.retain(|s| {
            let status_str = format!("{:?}", s.frontmatter.status).to_lowercase();
            statuses.iter().any(|st| st.to_lowercase() == status_str)
        });
    }

    // Filter by type
    if let Some(type_str) = type_filter {
        specs.retain(|s| s.frontmatter.r#type == type_str);
    }

    // Filter by labels (OR logic)
    if !labels.is_empty() {
        specs.retain(|s| {
            if let Some(spec_labels) = &s.frontmatter.labels {
                labels.iter().any(|l| spec_labels.contains(l))
            } else {
                false
            }
        });
    }

    // Filter by ready status
    if ready_only {
        let all_specs = spec::load_all_specs(&PathBuf::from(".chant/specs"))?;
        specs.retain(|s| s.is_ready(&all_specs));
    }

    // Filter by date range (based on spec ID date component)
    if from_date.is_some() || to_date.is_some() {
        specs.retain(|s| {
            // Extract date from spec ID (format: YYYY-MM-DD-XXX-abc)
            let id_parts: Vec<&str> = s.id.split('-').collect();
            if id_parts.len() < 3 {
                return false;
            }

            let spec_date = format!("{}-{}-{}", id_parts[0], id_parts[1], id_parts[2]);

            let mut matches = true;

            if let Some(from) = from_date {
                matches = matches && spec_date >= from.to_string();
            }

            if let Some(to) = to_date {
                matches = matches && spec_date <= to.to_string();
            }

            matches
        });
    }

    Ok(())
}

/// Get list of field names to export
fn get_field_list(fields: Option<&str>) -> Vec<String> {
    match fields {
        None => vec![
            "id".to_string(),
            "type".to_string(),
            "status".to_string(),
            "title".to_string(),
            "labels".to_string(),
            "model".to_string(),
            "completed_at".to_string(),
        ],
        Some("all") => vec![
            "id".to_string(),
            "type".to_string(),
            "status".to_string(),
            "title".to_string(),
            "labels".to_string(),
            "target_files".to_string(),
            "depends_on".to_string(),
            "model".to_string(),
            "completed_at".to_string(),
            "commits".to_string(),
            "pr".to_string(),
            "tracks".to_string(),
            "informed_by".to_string(),
            "origin".to_string(),
            "schedule".to_string(),
        ],
        Some(field_str) => field_str.split(',').map(|f| f.trim().to_string()).collect(),
    }
}

/// Extract a field value from a spec as a JSON value
fn get_field_value(spec: &Spec, field: &str) -> serde_json::Value {
    match field {
        "id" => json!(spec.id),
        "type" => json!(spec.frontmatter.r#type),
        "status" => json!(format!("{:?}", spec.frontmatter.status).to_lowercase()),
        "title" => json!(spec.title),
        "labels" => json!(spec.frontmatter.labels),
        "target_files" => json!(spec.frontmatter.target_files),
        "depends_on" => json!(spec.frontmatter.depends_on),
        "model" => json!(spec.frontmatter.model),
        "completed_at" => json!(spec.frontmatter.completed_at),
        "commits" => json!(spec.frontmatter.commits),
        "pr" => json!(spec.frontmatter.pr),
        "tracks" => json!(spec.frontmatter.tracks),
        "informed_by" => json!(spec.frontmatter.informed_by),
        "origin" => json!(spec.frontmatter.origin),
        "schedule" => json!(spec.frontmatter.schedule),
        _ => json!(null),
    }
}

/// Export specs as JSON
fn export_json(specs: &[Spec], fields: Option<&str>) -> Result<String> {
    let field_list = get_field_list(fields);
    let mut json_array = Vec::new();

    for spec in specs {
        let mut obj = serde_json::Map::new();

        for field in &field_list {
            let value = get_field_value(spec, field);
            // Only include non-null values
            if !value.is_null() {
                obj.insert(field.clone(), value);
            }
        }

        json_array.push(serde_json::Value::Object(obj));
    }

    Ok(serde_json::to_string_pretty(&json_array)?)
}

/// Export specs as CSV
fn export_csv(specs: &[Spec], fields: Option<&str>) -> Result<String> {
    let field_list = get_field_list(fields);

    let mut output = String::new();

    // Write header
    output.push_str(&field_list.join(","));
    output.push('\n');

    // Write data rows
    for spec in specs {
        let values: Vec<String> = field_list
            .iter()
            .map(|field| {
                let value = get_field_value(spec, field);
                csv_escape(&value.to_string())
            })
            .collect();

        output.push_str(&values.join(","));
        output.push('\n');
    }

    Ok(output)
}

/// Escape a value for CSV output
fn csv_escape(value: &str) -> String {
    // Handle null specially
    if value == "null" {
        return String::new();
    }

    // If value contains comma, quote, or newline, wrap in quotes and escape quotes
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Export specs as Markdown table
fn export_markdown(specs: &[Spec], fields: Option<&str>) -> Result<String> {
    let field_list = get_field_list(fields);

    let mut output = String::new();

    // Write header row
    output.push('|');
    for field in &field_list {
        output.push(' ');
        output.push_str(field);
        output.push_str(" |");
    }
    output.push('\n');

    // Write separator row
    output.push('|');
    for _ in &field_list {
        output.push_str(" --- |");
    }
    output.push('\n');

    // Write data rows
    for spec in specs {
        output.push('|');
        for field in &field_list {
            output.push(' ');
            let value = get_field_value(spec, field);
            let value_str = if value.is_null() {
                "-".to_string()
            } else if value.is_array() || value.is_object() {
                // Truncate long arrays/objects for readability
                let s = value.to_string();
                if s.len() > 30 {
                    format!("{}...", &s[..27])
                } else {
                    s
                }
            } else {
                value.to_string().trim_matches('"').to_string()
            };
            output.push_str(&value_str);
            output.push_str(" |");
        }
        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape() {
        assert_eq!(csv_escape("simple"), "simple");
        assert_eq!(csv_escape("with,comma"), "\"with,comma\"");
        assert_eq!(csv_escape("with\"quote"), "\"with\"\"quote\"");
        assert_eq!(csv_escape("with\nnewline"), "\"with\nnewline\"");
    }

    #[test]
    fn test_get_field_list_default() {
        let fields = get_field_list(None);
        assert!(fields.contains(&"id".to_string()));
        assert!(fields.contains(&"type".to_string()));
        assert!(fields.contains(&"status".to_string()));
    }

    #[test]
    fn test_get_field_list_custom() {
        let fields = get_field_list(Some("id,type,title"));
        assert_eq!(fields.len(), 3);
        assert!(fields.contains(&"id".to_string()));
        assert!(fields.contains(&"type".to_string()));
        assert!(fields.contains(&"title".to_string()));
    }
}
