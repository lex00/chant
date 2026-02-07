//! UI helpers for single-spec work mode

use chant::spec::{BlockingDependency, SpecStatus};
use colored::Colorize;

/// Print detailed error message for blocked spec dependencies
pub fn print_blocking_dependencies_error(spec_id: &str, blockers: &[BlockingDependency]) {
    eprintln!(
        "\n{} Spec {} is blocked by dependencies\n",
        "Error:".red().bold(),
        spec_id.cyan()
    );
    eprintln!("Blocking dependencies:");

    for blocker in blockers {
        let status_indicator = match blocker.status {
            SpecStatus::Completed => "●".green(),
            SpecStatus::InProgress => "◐".yellow(),
            SpecStatus::Failed => "✗".red(),
            SpecStatus::Blocked => "◌".magenta(),
            _ => "○".white(),
        };

        let title_display = blocker.title.as_deref().unwrap_or("");
        let sibling_marker = if blocker.is_sibling { " (sibling)" } else { "" };

        eprintln!(
            "  {} {} {}{}",
            status_indicator,
            blocker.spec_id.cyan(),
            title_display,
            sibling_marker.dimmed()
        );
        eprintln!(
            "    Status: {}",
            format!("{:?}", blocker.status).to_lowercase()
        );

        if let Some(ref completed_at) = blocker.completed_at {
            eprintln!("    Completed at: {}", completed_at);
            if blocker.status == SpecStatus::Completed {
                eprintln!(
                    "    {} This dependency is complete but spec still shows as blocked - this may be a bug",
                    "⚠️".yellow()
                );
            }
        }
    }

    eprintln!("\nNext steps:");
    eprintln!(
        "  1. Run '{}' to update dependency status",
        "chant refresh".cyan()
    );
    eprintln!(
        "  2. Use '{}' to override dependency checks",
        format!("chant work {} --skip-deps", spec_id).cyan()
    );
    eprintln!(
        "  3. Check dependency details with '{}'",
        "chant show <dep-id>".cyan()
    );

    let has_complete_blockers = blockers.iter().any(|b| b.status == SpecStatus::Completed);
    if has_complete_blockers {
        eprintln!(
            "\n{} If the dependency is truly complete, this is likely a dependency resolution bug",
            "Tip:".yellow().bold()
        );
    }
    eprintln!();
}

/// Print approval requirement error
pub fn print_approval_error(spec_id: &str) {
    eprintln!(
        "\n{} Spec {} requires approval before work can begin\n",
        "Error:".red().bold(),
        spec_id.cyan()
    );
    eprintln!("This spec has 'approval.required: true' but has not been approved yet.");
    eprintln!("\nNext steps:");
    eprintln!(
        "  1. Get approval: {}",
        format!("chant approve {} --by <name>", spec_id).cyan()
    );
    eprintln!(
        "  2. Or bypass with: {}",
        format!("chant work {} --skip-approval", spec_id).cyan()
    );
    eprintln!();
}

/// Print usage hint for work command in non-TTY contexts
pub fn print_work_usage_hint() {
    println!("Usage: chant work <SPEC_ID>\n");
    println!("Examples:");
    println!("  chant work 2026-01-27-001-abc");
    println!("  chant work 001-abc");
    println!("  chant work --parallel\n");
    println!("Run 'chant work --help' for all options.");
}

/// Format a grade enum for display with color coding
pub fn format_grade<T: std::fmt::Display>(grade: &T) -> colored::ColoredString {
    let grade_str = format!("{}", grade);
    match grade_str.as_str() {
        "A" => grade_str.green(),
        "B" => grade_str.green(),
        "C" => grade_str.yellow(),
        "D" => grade_str.red(),
        _ => grade_str.white(),
    }
}

/// Print quality score details for a spec
pub fn print_quality_assessment(quality_score: &chant::scoring::SpecScore) {
    eprintln!("Quality Assessment:");
    eprintln!(
        "  Complexity:    {}",
        format_grade(&quality_score.complexity)
    );
    eprintln!(
        "  Confidence:    {}",
        format_grade(&quality_score.confidence)
    );
    eprintln!(
        "  Splittability: {}",
        format_grade(&quality_score.splittability)
    );
    eprintln!(
        "  AC Quality:    {}",
        format_grade(&quality_score.ac_quality)
    );
    if let Some(iso) = quality_score.isolation {
        eprintln!("  Isolation:     {}", format_grade(&iso));
    }
}

/// Print quality suggestions and guidance
pub fn print_quality_suggestions_and_guidance(quality_score: &chant::scoring::SpecScore) {
    use chant::score::traffic_light;

    let suggestions = traffic_light::generate_suggestions(quality_score);
    if !suggestions.is_empty() {
        eprintln!("\nSuggestions:");
        for suggestion in &suggestions {
            eprintln!("  • {}", suggestion);
        }
    }

    let guidance = traffic_light::generate_detailed_guidance(quality_score);
    if !guidance.is_empty() {
        eprint!("{}", guidance);
    }
}

/// Prompt user to continue despite quality issues
pub fn confirm_continue_with_quality_issues() -> anyhow::Result<bool> {
    use std::io::{self, Write};
    print!("Continue anyway? [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}
