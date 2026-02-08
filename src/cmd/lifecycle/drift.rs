//! Drift detection - checks if documented/researched inputs have changed

use anyhow::{Context, Result};
use colored::Colorize;

use chant::spec::{self, Spec, SpecStatus};

#[derive(Debug)]
struct DriftReport {
    spec_id: String,
    spec_type: String,
    completed_at: String,
    drifted_files: Vec<DriftedFile>,
}

#[derive(Debug)]
struct DriftedFile {
    path: String,
    modified_at: String,
}

/// Check if any files matching a pattern have been modified after a certain time
fn check_files_for_changes(
    pattern: &str,
    completed_time: &chrono::DateTime<chrono::FixedOffset>,
    drift_report: &mut DriftReport,
) -> Result<()> {
    // Expand glob pattern to actual files
    let mut expanded_files = Vec::new();

    // Check if pattern is a glob
    if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
        // Use glob to expand
        use glob::glob as glob_fn;
        for entry in glob_fn(pattern)
            .context(format!("Invalid glob pattern: {}", pattern))?
            .flatten()
        {
            if entry.is_file() {
                expanded_files.push(entry);
            }
        }
    } else {
        // Literal path
        let path = std::path::PathBuf::from(pattern);
        if path.exists() && path.is_file() {
            expanded_files.push(path);
        }
    }

    // For each file, check if it was modified after completed_at
    for file_path in expanded_files {
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                let file_modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
                let completed_utc = completed_time.with_timezone(&chrono::Utc);

                if file_modified_time > completed_utc {
                    let relative_path = file_path.to_string_lossy().to_string();
                    drift_report.drifted_files.push(DriftedFile {
                        path: relative_path,
                        modified_at: file_modified_time.format("%Y-%m-%d").to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check if documentation and research specs have stale inputs
pub fn cmd_drift(id: Option<&str>) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;

    let mut specs = spec::load_all_specs(&specs_dir)?;

    // If a specific ID is provided, filter to that spec
    let specs_to_check_indices: Vec<usize> = if let Some(filter_id) = id {
        specs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id.contains(filter_id))
            .map(|(i, _)| i)
            .collect()
    } else {
        (0..specs.len()).collect()
    };

    if specs_to_check_indices.is_empty() {
        if let Some(filter_id) = id {
            anyhow::bail!("No specs found matching: {}", filter_id);
        } else {
            println!("No specs to check for drift.");
            return Ok(());
        }
    }

    let mut drifted_specs = Vec::new();
    let mut up_to_date_specs = Vec::new();

    for idx in specs_to_check_indices {
        let spec = &specs[idx];

        // Only check completed specs
        if spec.frontmatter.status != SpecStatus::Completed {
            continue;
        }

        // Get completion time
        let completed_at = match &spec.frontmatter.completed_at {
            Some(timestamp) => timestamp.clone(),
            None => {
                // If completed but no timestamp, skip
                continue;
            }
        };

        // Parse timestamp - format is ISO 8601 UTC (e.g., "2026-01-24T15:30:00Z")
        let completed_time = match chrono::DateTime::parse_from_rfc3339(&completed_at) {
            Ok(dt) => dt,
            Err(_) => {
                // If timestamp format is invalid, skip
                continue;
            }
        };

        // Check for drifts
        let mut drift_report = DriftReport {
            spec_id: spec.id.clone(),
            spec_type: spec.frontmatter.r#type.clone(),
            completed_at: completed_at.clone(),
            drifted_files: Vec::new(),
        };

        // Check tracked files (documentation specs)
        if let Some(tracked) = &spec.frontmatter.tracks {
            for file_pattern in tracked {
                check_files_for_changes(file_pattern, &completed_time, &mut drift_report)?;
            }
        }

        // Check origin files (research specs)
        if let Some(origin) = &spec.frontmatter.origin {
            for file_pattern in origin {
                check_files_for_changes(file_pattern, &completed_time, &mut drift_report)?;
            }
        }

        // Check informed_by files (research specs)
        if let Some(informed_by) = &spec.frontmatter.informed_by {
            for file_pattern in informed_by {
                check_files_for_changes(file_pattern, &completed_time, &mut drift_report)?;
            }
        }

        if drift_report.drifted_files.is_empty() {
            up_to_date_specs.push(drift_report);
        } else {
            // Update spec status to needs_attention
            let spec_mut = &mut specs[idx];
            spec::TransitionBuilder::new(spec_mut).to(SpecStatus::NeedsAttention)?;
            let spec_path = specs_dir.join(format!("{}.md", spec_mut.id));
            spec_mut.save(&spec_path)?;

            drifted_specs.push(drift_report);
        }
    }

    // Display results
    if drifted_specs.is_empty() && up_to_date_specs.is_empty() {
        println!("No completed specs with tracked/origin/informed_by fields to check.");
        return Ok(());
    }

    if !drifted_specs.is_empty() {
        println!(
            "\n{} Drifted Specs (inputs changed after completion)",
            "⚠".yellow()
        );
        println!("{}", "─".repeat(70));

        for report in &drifted_specs {
            println!(
                "\n{} Spec: {} ({})",
                "●".red(),
                report.spec_id,
                report.spec_type
            );
            println!("  Completed: {}", report.completed_at.bright_black());
            for drifted_file in &report.drifted_files {
                println!(
                    "    {} {} (modified {})",
                    "→".bright_black(),
                    drifted_file.path,
                    drifted_file.modified_at.bright_black()
                );
            }
            println!(
                "  {}",
                "Recommendation: Re-run spec to update analysis/documentation".yellow()
            );
        }
    }

    if !up_to_date_specs.is_empty() && !drifted_specs.is_empty() {
        println!();
    }

    if !up_to_date_specs.is_empty() {
        println!("\n{} Up-to-date Specs (no input changes)", "✓".green());
        println!("{}", "─".repeat(70));

        for report in &up_to_date_specs {
            println!("{} {} ({})", "●".green(), report.spec_id, report.spec_type);
        }
    }

    // Return success if checking specific spec even if it drifted
    Ok(())
}
