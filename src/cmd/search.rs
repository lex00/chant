//! Search command handler for searching specs by title and body content
//!
//! Supports case-insensitive and case-sensitive search, filtering by date,
//! status, type, labels, and archive scope. Includes interactive wizard mode
//! when no query is provided.

use anyhow::Result;
use atty;
use chrono::{Duration, Local, NaiveDate};
use colored::Colorize;

use chant::paths::ARCHIVE_DIR;
use chant::spec::{self, Spec};

use crate::render;

/// Print usage hint for search command in non-TTY contexts
fn print_search_usage_hint() {
    println!("Usage: chant search <QUERY>\n");
    println!("Examples:");
    println!("  chant search \"authentication\"");
    println!("  chant search --status pending \"api\"");
    println!("  chant search --label bugfix\n");
    println!("Run 'chant search --help' for all options.");
}

/// Search options for filtering and display
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub title_only: bool,
    pub body_only: bool,
    pub case_sensitive: bool,
    pub status_filter: Option<String>,
    pub type_filter: Option<String>,
    pub label_filters: Vec<String>,
    pub since: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
    pub active_only: bool,
    pub archived_only: bool,
    pub global: bool,
    pub repo: Option<String>,
}

/// Parse a date specification string
fn parse_date_spec(spec: &str) -> Result<NaiveDate> {
    let today = Local::now().naive_local().date();

    // Check if it's a relative spec like "7d", "2w", "1m"
    if let Some(days_str) = spec.strip_suffix('d') {
        if let Ok(days) = days_str.parse::<i64>() {
            return Ok(today - Duration::days(days));
        }
    }

    if let Some(weeks_str) = spec.strip_suffix('w') {
        if let Ok(weeks) = weeks_str.parse::<i64>() {
            return Ok(today - Duration::weeks(weeks));
        }
    }

    if let Some(months_str) = spec.strip_suffix('m') {
        if let Ok(months) = months_str.parse::<i64>() {
            return Ok(today - Duration::days(months * 30));
        }
    }

    // Try parsing as YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(spec, "%Y-%m-%d") {
        return Ok(date);
    }

    anyhow::bail!(
        "Invalid date format: {}. Use Nd, Nw, Nm, or YYYY-MM-DD",
        spec
    )
}

/// Extract date from spec ID (YYYY-MM-DD prefix)
fn spec_date_from_id(spec_id: &str) -> Option<NaiveDate> {
    // Spec ID format: YYYY-MM-DD-XXX-abc
    let parts: Vec<&str> = spec_id.split('-').collect();
    if parts.len() >= 3 {
        let date_str = format!("{}-{}-{}", parts[0], parts[1], parts[2]);
        return NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
    }
    None
}

/// Check if a spec matches the search criteria
fn matches_search(spec: &Spec, opts: &SearchOptions) -> bool {
    // Text search
    let (search_title, search_body) = match (opts.title_only, opts.body_only) {
        (true, _) => (true, false),
        (false, true) => (false, true),
        _ => (true, true),
    };

    let mut text_match = false;

    if search_title {
        let title = spec.title.as_deref().unwrap_or("");
        if opts.case_sensitive {
            text_match = title.contains(&opts.query);
        } else {
            text_match = title.to_lowercase().contains(&opts.query.to_lowercase());
        }
    }

    if !text_match && search_body {
        if opts.case_sensitive {
            text_match = spec.body.contains(&opts.query);
        } else {
            text_match = spec
                .body
                .to_lowercase()
                .contains(&opts.query.to_lowercase());
        }
    }

    if !text_match {
        return false;
    }

    // Status filter
    if let Some(status) = &opts.status_filter {
        let status_match = match status.as_str() {
            "pending" => spec.frontmatter.status == chant::spec::SpecStatus::Pending,
            "ready" => spec.frontmatter.status == chant::spec::SpecStatus::Ready,
            "in_progress" => spec.frontmatter.status == chant::spec::SpecStatus::InProgress,
            "completed" => spec.frontmatter.status == chant::spec::SpecStatus::Completed,
            "failed" => spec.frontmatter.status == chant::spec::SpecStatus::Failed,
            _ => false,
        };
        if !status_match {
            return false;
        }
    }

    // Type filter
    if let Some(type_filter) = &opts.type_filter {
        if spec.frontmatter.r#type != *type_filter {
            return false;
        }
    }

    // Label filters (OR logic - any matching label)
    if !opts.label_filters.is_empty() {
        let has_label = if let Some(spec_labels) = &spec.frontmatter.labels {
            opts.label_filters.iter().any(|l| spec_labels.contains(l))
        } else {
            false
        };
        if !has_label {
            return false;
        }
    }

    // Date range filter
    if let Some(since) = opts.since {
        if let Some(spec_date) = spec_date_from_id(&spec.id) {
            if spec_date < since {
                return false;
            }
        }
    }

    if let Some(until) = opts.until {
        if let Some(spec_date) = spec_date_from_id(&spec.id) {
            if spec_date > until {
                return false;
            }
        }
    }

    true
}

/// Run the interactive search wizard
fn run_wizard() -> Result<()> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Confirm, Input, Select};

    let theme = ColorfulTheme::default();

    // Get search query
    let query: String = Input::with_theme(&theme)
        .with_prompt("Search query")
        .interact()?;

    if query.is_empty() {
        println!("Search query cannot be empty.");
        return Ok(());
    }

    // Select search scope
    let scope_idx = Select::with_theme(&theme)
        .with_prompt("Search in")
        .default(0)
        .items(&["Title + Body", "Title only", "Body only"])
        .interact()?;

    let (title_only, body_only) = match scope_idx {
        0 => (false, false),
        1 => (true, false),
        2 => (false, true),
        _ => (false, false),
    };

    // Select date range
    let date_options = vec![
        "Any time",
        "Last 7 days (--since 7d)",
        "Last 2 weeks (--since 2w)",
        "Last month (--since 1m)",
        "Custom...",
    ];
    let date_idx = Select::with_theme(&theme)
        .with_prompt("Date range")
        .default(0)
        .items(&date_options)
        .interact()?;

    let (since_opt, until_opt): (Option<String>, Option<String>) = match date_idx {
        0 => (None, None),
        1 => (Some("7d".to_string()), None),
        2 => (Some("2w".to_string()), None),
        3 => (Some("1m".to_string()), None),
        4 => {
            let custom: String = Input::with_theme(&theme)
                .with_prompt("Date spec (e.g., 7d, 2w, 1m, or YYYY-MM-DD)")
                .interact()?;
            (Some(custom), None)
        }
        _ => (None, None),
    };

    // Include archived?
    let include_archived = Confirm::with_theme(&theme)
        .with_prompt("Include archived specs?")
        .default(true)
        .interact()?;

    // Status filter
    let status_options = vec![
        "Any",
        "pending",
        "ready",
        "in_progress",
        "completed",
        "failed",
    ];
    let status_idx = Select::with_theme(&theme)
        .with_prompt("Filter by status")
        .default(0)
        .items(&status_options)
        .interact()?;

    let status_filter = match status_idx {
        0 => None,
        _ => Some(status_options[status_idx].to_string()),
    };

    // Type filter
    let type_options = vec!["Any", "code", "task", "documentation", "research"];
    let type_idx = Select::with_theme(&theme)
        .with_prompt("Filter by type")
        .default(0)
        .items(&type_options)
        .interact()?;

    let type_filter = match type_idx {
        0 => None,
        _ => Some(type_options[type_idx].to_string()),
    };

    let opts = SearchOptions {
        query,
        title_only,
        body_only,
        case_sensitive: false,
        status_filter,
        type_filter,
        label_filters: vec![],
        since: since_opt.as_deref().and_then(|s| parse_date_spec(s).ok()),
        until: until_opt.as_deref().and_then(|s| parse_date_spec(s).ok()),
        // include_archived=true means search both (neither flag set)
        // include_archived=false means active only
        active_only: !include_archived,
        archived_only: false,
        global: false,
        repo: None,
    };

    perform_search(&opts)
}

/// Perform the actual search and display results
fn perform_search(opts: &SearchOptions) -> Result<()> {
    let mut all_specs = Vec::new();
    let specs_dir = std::path::PathBuf::from(".");

    if opts.global || opts.repo.is_some() {
        // Load specs from multiple repos
        use chant::config::Config;
        use std::path::PathBuf;

        let config = Config::load_merged()?;

        if config.repos.is_empty() {
            anyhow::bail!(
                "No repos configured in global config. \
                 Please add repos to ~/.config/chant/config.md or use local mode without --global/--repo"
            );
        }

        // If repo filter is specified, validate it exists
        if let Some(repo_name) = &opts.repo {
            if !config.repos.iter().any(|r| &r.name == repo_name) {
                anyhow::bail!(
                    "Repository '{}' not found in global config. Available repos: {}",
                    repo_name,
                    config
                        .repos
                        .iter()
                        .map(|r| r.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        for repo_config in &config.repos {
            // Skip if filtering by repo and this isn't it
            if let Some(filter) = &opts.repo {
                if &repo_config.name != filter {
                    continue;
                }
            }

            // Expand path
            let repo_path = shellexpand::tilde(&repo_config.path).to_string();
            let repo_path = PathBuf::from(repo_path);

            let active_specs_dir = repo_path.join(".chant/specs");
            let archive_specs_dir = repo_path.join(".chant/archive");

            // Load active specs
            if !opts.archived_only && active_specs_dir.exists() {
                match spec::load_all_specs(&active_specs_dir) {
                    Ok(mut repo_specs) => {
                        for spec in &mut repo_specs {
                            spec.id = format!("{}:{}", repo_config.name, spec.id);
                        }
                        all_specs.extend(repo_specs);
                    }
                    Err(e) => {
                        eprintln!(
                            "{} Failed to load specs from repo '{}': {}",
                            "⚠".yellow(),
                            repo_config.name,
                            e
                        );
                    }
                }
            }

            // Load archived specs
            if !opts.active_only && archive_specs_dir.exists() {
                match spec::load_all_specs(&archive_specs_dir) {
                    Ok(mut repo_specs) => {
                        for spec in &mut repo_specs {
                            spec.id = format!("{}:{}", repo_config.name, spec.id);
                        }
                        all_specs.extend(repo_specs);
                    }
                    Err(e) => {
                        eprintln!(
                            "{} Failed to load archived specs from repo '{}': {}",
                            "⚠".yellow(),
                            repo_config.name,
                            e
                        );
                    }
                }
            }
        }
    } else {
        // Load local specs
        let specs_dir = crate::cmd::ensure_initialized()?;

        // Load active specs
        if !opts.archived_only {
            all_specs.extend(spec::load_all_specs(&specs_dir)?);
        }

        // Load archived specs
        if !opts.active_only {
            let archive_path = std::path::PathBuf::from(ARCHIVE_DIR);
            if archive_path.exists() {
                let mut archived = spec::load_all_specs(&archive_path)?;
                all_specs.append(&mut archived);
            }
        }
    }

    // Filter specs
    let mut results: Vec<(bool, &Spec)> = all_specs
        .iter()
        .filter(|s| matches_search(s, opts))
        .map(|s| {
            // For global search, always treat as potentially archived
            // (we can't easily check if a cross-repo spec is archived without more context)
            let is_archived = if opts.global || opts.repo.is_some() {
                s.id.contains("archive")
            } else {
                !specs_dir.join(format!("{}.md", s.id)).exists()
            };
            (is_archived, s)
        })
        .collect();

    // Sort by ID
    results.sort_by(|a, b| a.1.id.cmp(&b.1.id));

    if results.is_empty() {
        println!("No specs found matching \"{}\"", opts.query);
        return Ok(());
    }

    // Display results
    for (is_archived, spec) in &results {
        let icon = if spec.frontmatter.r#type == "conflict" {
            "⚡".yellow()
        } else {
            render::status_icon(&spec.frontmatter.status)
        };

        let title = spec.title.as_deref().unwrap_or("(no title)");
        let archive_label = if *is_archived { " [archived]" } else { "" };

        println!(
            "{} {} {}{}",
            icon,
            spec.id.cyan(),
            title,
            archive_label.dimmed()
        );
    }

    let count = results.len();
    let matches_text = if count == 1 { "spec" } else { "specs" };
    println!(
        "\nFound {} {} matching \"{}\"",
        count, matches_text, opts.query
    );

    Ok(())
}

/// Execute the search command with the given options
pub fn cmd_search(opts: Option<SearchOptions>) -> Result<()> {
    // If no options provided, check for TTY
    if opts.is_none() {
        // If not a TTY, print usage hint instead of launching wizard
        if !atty::is(atty::Stream::Stdin) {
            print_search_usage_hint();
            return Ok(());
        }
        return run_wizard();
    }

    perform_search(&opts.unwrap())
}

/// Build search options from command-line arguments
#[allow(clippy::too_many_arguments)]
pub fn build_search_options(
    query: Option<String>,
    title_only: bool,
    body_only: bool,
    case_sensitive: bool,
    status: Option<String>,
    type_: Option<String>,
    label: Vec<String>,
    since: Option<String>,
    until: Option<String>,
    active_only: bool,
    archived_only: bool,
    global: bool,
    repo: Option<&str>,
) -> Result<Option<SearchOptions>> {
    if query.is_none() {
        return Ok(None);
    }

    let query = query.unwrap();
    Ok(Some(SearchOptions {
        query,
        title_only,
        body_only,
        case_sensitive,
        status_filter: status,
        type_filter: type_,
        label_filters: label,
        since: since.as_deref().and_then(|s| parse_date_spec(s).ok()),
        until: until.as_deref().and_then(|s| parse_date_spec(s).ok()),
        active_only,
        archived_only,
        global,
        repo: repo.map(|s| s.to_string()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_spec_days() {
        let date = parse_date_spec("7d").unwrap();
        let today = Local::now().naive_local().date();
        assert_eq!(date, today - Duration::days(7));
    }

    #[test]
    fn test_parse_date_spec_weeks() {
        let date = parse_date_spec("2w").unwrap();
        let today = Local::now().naive_local().date();
        assert_eq!(date, today - Duration::weeks(2));
    }

    #[test]
    fn test_parse_date_spec_months() {
        let date = parse_date_spec("1m").unwrap();
        let today = Local::now().naive_local().date();
        assert_eq!(date, today - Duration::days(30));
    }

    #[test]
    fn test_parse_date_spec_absolute() {
        let date = parse_date_spec("2026-01-20").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 1, 20).unwrap());
    }

    #[test]
    fn test_spec_date_from_id() {
        let date = spec_date_from_id("2026-01-24-001-abc");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 1, 24));
    }

    #[test]
    fn test_spec_date_from_id_with_suffix() {
        let date = spec_date_from_id("2026-01-24-001-abc.1");
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 1, 24));
    }

    #[test]
    fn test_text_match_case_insensitive() {
        // Create a minimal spec for testing
        let spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: chant::spec::SpecFrontmatter::default(),
            title: Some("Add user Authentication".to_string()),
            body: "This spec adds user auth support.".to_string(),
        };

        let opts = SearchOptions {
            query: "auth".to_string(),
            title_only: false,
            body_only: false,
            case_sensitive: false,
            status_filter: None,
            type_filter: None,
            label_filters: vec![],
            since: None,
            until: None,
            active_only: false,
            archived_only: false,
            global: false,
            repo: None,
        };

        assert!(matches_search(&spec, &opts));

        let opts2 = SearchOptions {
            query: "AUTH".to_string(),
            ..opts.clone()
        };

        assert!(matches_search(&spec, &opts2));
    }

    #[test]
    fn test_text_match_case_sensitive() {
        let spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: chant::spec::SpecFrontmatter::default(),
            title: Some("Add user Authentication".to_string()),
            body: "This spec adds user auth support.".to_string(),
        };

        let opts = SearchOptions {
            query: "Authentication".to_string(),
            title_only: false,
            body_only: false,
            case_sensitive: true,
            status_filter: None,
            type_filter: None,
            label_filters: vec![],
            since: None,
            until: None,
            active_only: false,
            archived_only: false,
            global: false,
            repo: None,
        };

        assert!(matches_search(&spec, &opts));

        let opts2 = SearchOptions {
            query: "authentication".to_string(),
            ..opts.clone()
        };

        assert!(!matches_search(&spec, &opts2));
    }

    #[test]
    fn test_title_only_filter() {
        let spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: chant::spec::SpecFrontmatter::default(),
            title: Some("Add user".to_string()),
            body: "This is about authentication.".to_string(),
        };

        // Should match in title
        let opts = SearchOptions {
            query: "user".to_string(),
            title_only: true,
            body_only: false,
            case_sensitive: false,
            status_filter: None,
            type_filter: None,
            label_filters: vec![],
            since: None,
            until: None,
            active_only: false,
            archived_only: false,
            global: false,
            repo: None,
        };

        assert!(matches_search(&spec, &opts));

        // Should not match in body only
        let opts2 = SearchOptions {
            query: "authentication".to_string(),
            title_only: true,
            ..opts.clone()
        };

        assert!(!matches_search(&spec, &opts2));
    }

    #[test]
    fn test_body_only_filter() {
        let spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: chant::spec::SpecFrontmatter::default(),
            title: Some("Add user".to_string()),
            body: "This is about authentication.".to_string(),
        };

        // Should match in body
        let opts = SearchOptions {
            query: "authentication".to_string(),
            title_only: false,
            body_only: true,
            case_sensitive: false,
            status_filter: None,
            type_filter: None,
            label_filters: vec![],
            since: None,
            until: None,
            active_only: false,
            archived_only: false,
            global: false,
            repo: None,
        };

        assert!(matches_search(&spec, &opts));

        // Should not match in title only
        let opts2 = SearchOptions {
            query: "user".to_string(),
            ..opts.clone()
        };

        assert!(!matches_search(&spec, &opts2));
    }
}
