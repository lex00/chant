//! Spec creation functionality
//!
//! Provides the `cmd_add` command function for creating new specs.

use anyhow::Result;
use colored::Colorize;

use chant::config::Config;
use chant::score::ac_quality::calculate_ac_quality;
use chant::score::confidence::calculate_confidence;
use chant::score::splittability::calculate_splittability;
use chant::score::traffic_light::{determine_status, generate_suggestions};
use chant::scoring::{calculate_complexity, SpecScore, TrafficLight};
use chant::spec;

pub fn cmd_add(description: &str, prompt: Option<&str>, needs_approval: bool) -> Result<()> {
    let config = Config::load()?;
    let specs_dir = crate::cmd::ensure_initialized()?;

    // Use operations module for spec creation
    let options = chant::operations::create::CreateOptions {
        prompt: prompt.map(String::from),
        needs_approval,
        auto_commit: true,
    };

    let (spec, filepath) =
        chant::operations::create::create_spec(description, &specs_dir, &config, options)?;

    if !crate::cmd::ui::is_quiet() {
        println!("{} {}", "Created".green(), spec.id.cyan());
        if needs_approval {
            println!("{} Requires approval before work can begin", "ℹ".cyan());
        }
        println!("Edit: {}", filepath.display());

        // Calculate and display quality score
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
