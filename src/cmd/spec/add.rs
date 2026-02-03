//! Spec creation functionality
//!
//! Provides the `cmd_add` command function for creating new specs.

use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

use chant::config::Config;
use chant::derivation::{self, DerivationEngine};
use chant::id;
use chant::score::ac_quality::calculate_ac_quality;
use chant::score::confidence::calculate_confidence;
use chant::score::splittability::calculate_splittability;
use chant::score::traffic_light::{determine_status, generate_suggestions};
use chant::scoring::{calculate_complexity, SpecScore, TrafficLight};
use chant::spec;

pub fn cmd_add(description: &str, prompt: Option<&str>, needs_approval: bool) -> Result<()> {
    let config = Config::load()?;
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Generate ID
    let id = id::generate_id(&specs_dir)?;
    let filename = format!("{}.md", id);
    let filepath = specs_dir.join(&filename);

    // Create spec content
    let prompt_line = match prompt {
        Some(p) => format!("prompt: {}\n", p),
        None => String::new(),
    };

    let approval_line = if needs_approval {
        "approval:\n  required: true\n  status: pending\n"
    } else {
        ""
    };

    let content = format!(
        r#"---
type: code
status: pending
{}{}---

# {}
"#,
        prompt_line, approval_line, description
    );

    std::fs::write(&filepath, content)?;

    // Parse the spec to add derived fields if enterprise config is present
    if !config.enterprise.derived.is_empty() {
        // Load the spec we just created
        let mut spec = spec::Spec::load(&filepath)?;

        // Build derivation context
        let context = derivation::build_context(&id, &specs_dir);

        // Derive fields using the engine
        let engine = DerivationEngine::new(config.enterprise.clone());
        let derived_fields = engine.derive_fields(&context);

        // Add derived fields to spec frontmatter
        spec.add_derived_fields(derived_fields);

        // Write the spec with derived fields
        spec.save(&filepath)?;
    }

    // Auto-commit the spec file to git (skip if .chant/ is gitignored, e.g. silent mode)
    let output = Command::new("git")
        .args(["add", &filepath.to_string_lossy()])
        .output()
        .context("Failed to run git add for spec file")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If the path is ignored (silent mode), skip git commit silently
        if stderr.contains("ignored") {
            // .chant/ is gitignored (silent mode) - skip git commit
        } else {
            anyhow::bail!("Failed to stage spec file {}: {}", id, stderr);
        }
    } else {
        let commit_message = format!("chant: Add spec {}", id);
        let output = Command::new("git")
            .args(["commit", "-m", &commit_message])
            .output()
            .context("Failed to run git commit for spec file")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // It's ok if there's nothing to commit (shouldn't happen but be safe)
            if !stderr.contains("nothing to commit") && !stderr.contains("no changes added") {
                anyhow::bail!("Failed to commit spec file {}: {}", id, stderr);
            }
        }
    }

    if !chant::ui::is_quiet() {
        println!("{} {}", "Created".green(), id.cyan());
        if needs_approval {
            println!("{} Requires approval before work can begin", "ℹ".cyan());
        }
        println!("Edit: {}", filepath.display());

        // Calculate and display quality score
        let spec = spec::Spec::load(&filepath)?;
        let score = calculate_spec_score(&spec, &config);
        display_quality_feedback(&score, &spec);
    }

    Ok(())
}

/// Extract acceptance criteria text from spec body
fn extract_acceptance_criteria(spec: &spec::Spec) -> Vec<String> {
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
fn calculate_spec_score(spec: &spec::Spec, config: &Config) -> SpecScore {
    let complexity = calculate_complexity(spec);
    let confidence = calculate_confidence(spec, config);
    let splittability = calculate_splittability(spec);

    // No isolation for newly created specs (not part of a group yet)
    let isolation = None;

    // Extract acceptance criteria for AC quality scoring
    let criteria = extract_acceptance_criteria(spec);
    let ac_quality = calculate_ac_quality(&criteria);

    let mut score = SpecScore {
        complexity,
        confidence,
        splittability,
        isolation,
        ac_quality,
        traffic_light: TrafficLight::Ready, // temporary, will be overwritten
    };

    // Determine final traffic light status
    score.traffic_light = determine_status(&score);

    score
}

/// Display quality feedback with traffic light, grades, and suggestions
fn display_quality_feedback(score: &SpecScore, spec: &spec::Spec) {
    // Calculate metrics for display
    let criteria_count = spec.count_total_checkboxes();
    let file_count = spec
        .frontmatter
        .target_files
        .as_ref()
        .map(|files| files.len())
        .unwrap_or(0);
    let word_count = spec.body.split_whitespace().count();

    println!();
    println!("Quality: {}", score.traffic_light);
    println!(
        "  Complexity: {} ({} criteria, {} files, {} words)",
        score.complexity, criteria_count, file_count, word_count
    );
    println!("  Confidence: {}", score.confidence);
    println!("  AC Quality: {}", score.ac_quality);

    // Only show suggestions if status is Review or Refine
    if matches!(
        score.traffic_light,
        TrafficLight::Review | TrafficLight::Refine
    ) {
        let suggestions = generate_suggestions(score);
        if !suggestions.is_empty() {
            println!();
            println!("{}", "Suggestions:".bold());
            for suggestion in suggestions {
                println!("  • {}", suggestion);
            }
        }
    }
}
