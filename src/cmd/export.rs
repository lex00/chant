//! Export command for exporting specs to various formats
//!
//! Supports JSON, CSV, and Markdown formats with flexible filtering options
//! including status, type, labels, and date ranges.
//!
//! When run without the --format flag, launches an interactive wizard to configure options.

use anyhow::{Context, Result};
use atty;
use dialoguer::Select;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use chant::spec::{self, Spec};
use chant::spec_group;

/// Print usage hint for export command in non-TTY contexts
fn print_export_usage_hint() {
    println!("Usage: chant export --format <FORMAT>\n");
    println!("Formats: json, csv, markdown\n");
    println!("Examples:");
    println!("  chant export --format json");
    println!("  chant export --format csv --output specs.csv");
    println!("  chant export --format markdown --status completed\n");
    println!("Run 'chant export --help' for all options.");
}

/// Holds the result of the interactive wizard
struct WizardOptions {
    format: String,
    statuses: Vec<String>,
    type_filter: Option<String>,
    labels: Vec<String>,
    ready_only: bool,
    output_file: Option<String>,
}

/// Main export command handler
#[allow(clippy::too_many_arguments)]
pub fn cmd_export(
    format: Option<&str>,
    statuses: &[String],
    type_filter: Option<&str>,
    labels: &[String],
    ready_only: bool,
    from_date: Option<&str>,
    to_date: Option<&str>,
    fields: Option<&str>,
    output_file: Option<&str>,
) -> Result<()> {
    // Detect wizard mode: triggered if format is None AND no other filters are set
    let is_wizard_mode = format.is_none()
        && statuses.is_empty()
        && type_filter.is_none()
        && labels.is_empty()
        && !ready_only
        && from_date.is_none()
        && to_date.is_none()
        && output_file.is_none();

    // If wizard mode, check for TTY
    let options = if is_wizard_mode {
        // If not a TTY, print usage hint instead of launching wizard
        if !atty::is(atty::Stream::Stdin) {
            print_export_usage_hint();
            return Ok(());
        }
        run_wizard()?
    } else {
        // Direct mode: use provided values
        WizardOptions {
            format: format.unwrap_or("json").to_string(),
            statuses: statuses.to_vec(),
            type_filter: type_filter.map(|s| s.to_string()),
            labels: labels.to_vec(),
            ready_only,
            output_file: output_file.map(|s| s.to_string()),
        }
    };

    let specs_dir = crate::cmd::ensure_initialized()?;

    // Load all specs
    let mut specs = spec::load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| spec_group::compare_spec_ids(&a.id, &b.id));

    // Apply filters
    apply_filters(
        &mut specs,
        &options.statuses,
        options.type_filter.as_deref(),
        &options.labels,
        options.ready_only,
        from_date,
        to_date,
    )?;

    if specs.is_empty() {
        let output = "No specs match the specified filters.";
        if let Some(file_path) = &options.output_file {
            let mut file = File::create(file_path).context("Failed to create output file")?;
            writeln!(file, "{}", output).context("Failed to write to output file")?;
        } else {
            println!("{}", output);
        }
        return Ok(());
    }

    // Generate output based on format
    let output = match options.format.to_lowercase().as_str() {
        "json" => export_json(&specs, fields)?,
        "csv" => export_csv(&specs, fields)?,
        "markdown" | "md" => export_markdown(&specs, fields)?,
        _ => anyhow::bail!(
            "Unknown format: {}. Supported formats: json, csv, markdown",
            options.format
        ),
    };

    // Write output
    if let Some(file_path) = options.output_file {
        let mut file = File::create(&file_path).context("Failed to create output file")?;
        write!(file, "{}", output).context("Failed to write to output file")?;
        println!("Export written to: {}", file_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

/// Run the interactive wizard to configure export options
fn run_wizard() -> Result<WizardOptions> {
    use dialoguer::{Input, MultiSelect};

    // 1. Ask for format
    let formats = vec!["JSON", "CSV", "Markdown"];
    let format_selection = Select::new()
        .with_prompt("Export format:")
        .items(&formats)
        .default(0)
        .interact()?;
    let format = formats[format_selection].to_lowercase();

    // 2. Ask for status filter
    let status_options = vec![
        "pending",
        "ready",
        "in_progress",
        "completed",
        "blocked",
        "cancelled",
    ];
    let status_selections = MultiSelect::new()
        .with_prompt("Filter by status (space to toggle, enter to confirm):")
        .items(&status_options)
        .defaults(&[false, true, false, false, false, false]) // ready is selected by default
        .interact()?;

    let statuses: Vec<String> = status_options
        .iter()
        .enumerate()
        .filter_map(|(i, &s)| {
            if status_selections.contains(&i) {
                Some(s.to_string())
            } else {
                None
            }
        })
        .collect();

    // 3. Ask for type filter
    let type_options = vec![
        "No filter",
        "code",
        "task",
        "driver",
        "documentation",
        "research",
    ];
    let type_selection = Select::new()
        .with_prompt("Filter by type:")
        .items(&type_options)
        .default(0)
        .interact()?;
    let type_filter = if type_selection == 0 {
        None
    } else {
        Some(type_options[type_selection].to_string())
    };

    // 4. Ask for output destination
    let output_options = vec!["Print to stdout", "Save to file"];
    let output_selection = Select::new()
        .with_prompt("Output destination:")
        .items(&output_options)
        .default(0)
        .interact()?;

    let output_file = if output_selection == 1 {
        let filename = Input::new()
            .with_prompt("Output filename:")
            .interact_text()?;
        Some(filename)
    } else {
        None
    };

    Ok(WizardOptions {
        format,
        statuses,
        type_filter,
        labels: vec![],
        ready_only: false,
        output_file,
    })
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
                matches = matches && spec_date.as_str() >= from;
            }

            if let Some(to) = to_date {
                matches = matches && spec_date.as_str() <= to;
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
            "derived_fields".to_string(),
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
        "tracks" => json!(spec.frontmatter.tracks),
        "informed_by" => json!(spec.frontmatter.informed_by),
        "origin" => json!(spec.frontmatter.origin),
        "schedule" => json!(spec.frontmatter.schedule),
        "derived_fields" => json!(spec.frontmatter.derived_fields),
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

    #[test]
    fn test_wizard_options_default() {
        let options = WizardOptions {
            format: "json".to_string(),
            statuses: vec!["ready".to_string()],
            type_filter: None,
            labels: vec![],
            ready_only: false,
            output_file: None,
        };
        assert_eq!(options.format, "json");
        assert_eq!(options.statuses.len(), 1);
        assert_eq!(options.statuses[0], "ready");
    }

    #[test]
    fn test_wizard_options_with_type_filter() {
        let options = WizardOptions {
            format: "csv".to_string(),
            statuses: vec!["ready".to_string(), "completed".to_string()],
            type_filter: Some("code".to_string()),
            labels: vec![],
            ready_only: false,
            output_file: Some("export.csv".to_string()),
        };
        assert_eq!(options.format, "csv");
        assert_eq!(options.statuses.len(), 2);
        assert_eq!(options.type_filter, Some("code".to_string()));
        assert_eq!(options.output_file, Some("export.csv".to_string()));
    }
}
