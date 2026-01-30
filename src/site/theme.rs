//! Theme management for site generation.
//!
//! This module handles copying the default theme to the user's project
//! for customization.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::embedded;

/// Theme file information
pub struct ThemeFile {
    pub name: &'static str,
    pub content: &'static str,
    pub description: &'static str,
}

/// Get all embedded theme files
pub fn get_theme_files() -> Vec<ThemeFile> {
    vec![
        ThemeFile {
            name: "base.html",
            content: embedded::BASE_HTML,
            description: "Page skeleton, head, nav",
        },
        ThemeFile {
            name: "spec.html",
            content: embedded::SPEC_HTML,
            description: "Individual spec page",
        },
        ThemeFile {
            name: "index.html",
            content: embedded::INDEX_HTML,
            description: "Main index page",
        },
        ThemeFile {
            name: "status-index.html",
            content: embedded::STATUS_INDEX_HTML,
            description: "By-status listing",
        },
        ThemeFile {
            name: "label-index.html",
            content: embedded::LABEL_INDEX_HTML,
            description: "By-label listing",
        },
        ThemeFile {
            name: "timeline.html",
            content: embedded::TIMELINE_HTML,
            description: "Timeline view",
        },
        ThemeFile {
            name: "graph.html",
            content: embedded::GRAPH_HTML,
            description: "Dependency graph view",
        },
        ThemeFile {
            name: "changelog.html",
            content: embedded::CHANGELOG_HTML,
            description: "Changelog view",
        },
        ThemeFile {
            name: "styles.css",
            content: embedded::STYLES_CSS,
            description: "All styling",
        },
    ]
}

/// Initialize the theme directory with default templates
pub fn init_theme(theme_dir: &Path, force: bool) -> Result<InitResult> {
    let mut result = InitResult::default();

    // Create theme directory
    fs::create_dir_all(theme_dir).with_context(|| {
        format!(
            "Failed to create theme directory at {}",
            theme_dir.display()
        )
    })?;

    // Copy each template file
    for file in get_theme_files() {
        let target_path = theme_dir.join(file.name);

        if target_path.exists() && !force {
            result.skipped.push(file.name.to_string());
            continue;
        }

        fs::write(&target_path, file.content)
            .with_context(|| format!("Failed to write {}", target_path.display()))?;

        result.created.push(file.name.to_string());
    }

    Ok(result)
}

/// Check if a custom theme directory exists
pub fn theme_exists(theme_dir: &Path) -> bool {
    theme_dir.exists() && theme_dir.is_dir()
}

/// List files in a custom theme directory
pub fn list_theme_files(theme_dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();

    if !theme_dir.exists() {
        return Ok(files);
    }

    for entry in fs::read_dir(theme_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                files.push(name.to_string());
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Result of theme initialization
#[derive(Debug, Default)]
pub struct InitResult {
    /// Files that were created
    pub created: Vec<String>,
    /// Files that were skipped (already exist)
    pub skipped: Vec<String>,
}

impl InitResult {
    /// Check if any files were created
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty()
    }
}

/// Template variable documentation
pub fn get_template_variables_doc() -> &'static str {
    r#"# Template Variables Reference

## Global Variables (available in all templates)

- `site_title` - The site title from config
- `base_url` - The base URL for all links
- `features` - Object with feature toggles:
  - `features.changelog`
  - `features.dependency_graph`
  - `features.timeline`
  - `features.status_indexes`
  - `features.label_indexes`
- `labels` - List of all labels used across specs

## Index Page (`index.html`)

- `specs` - List of all spec objects
- `stats` - Site statistics object:
  - `stats.total`
  - `stats.completed`
  - `stats.in_progress`
  - `stats.pending`
  - `stats.failed`
  - `stats.other`

## Spec Page (`spec.html`)

- `spec` - The current spec object:
  - `spec.id` - Full spec ID
  - `spec.short_id` - Short ID (last segment)
  - `spec.title` - Spec title (may be null)
  - `spec.status` - Status string (lowercase)
  - `spec.type` - Spec type
  - `spec.labels` - List of label strings
  - `spec.depends_on` - List of dependency IDs
  - `spec.target_files` - List of target file paths
  - `spec.completed_at` - Completion timestamp (may be null)
  - `spec.model` - Model used (may be null)
  - `spec.body_html` - Rendered markdown body as HTML
- `prev_spec` - Previous spec (may be null)
- `next_spec` - Next spec (may be null)

## Status Index Page (`status-index.html`)

- `status` - Status key (e.g., "completed")
- `status_display` - Display name (e.g., "Completed")
- `specs` - List of specs with this status

## Label Index Page (`label-index.html`)

- `label` - The label name
- `specs` - List of specs with this label

## Timeline Page (`timeline.html`)

- `timeline_groups` - List of timeline groups:
  - `group.date` - Date/period label
  - `group.ascii_tree` - ASCII tree visualization

## Graph Page (`graph.html`)

- `ascii_graph` - ASCII dependency graph
- `roots` - List of root specs (no dependencies)
- `leaves` - List of leaf specs (no dependents)

## Changelog Page (`changelog.html`)

- `changelog_groups` - List of changelog entries:
  - `group.date` - Completion date
  - `group.specs` - List of specs completed on this date

## Filters

- `slugify` - Convert string to URL-safe slug
  - Example: `{{ label | slugify }}`

## Example Template Snippet

```html
{% for spec in specs %}
<div class="spec-card">
  <h2>{{ spec.title | default(value="Untitled") }}</h2>
  <span class="status-{{ spec.status | slugify }}">{{ spec.status }}</span>
  {% for label in spec.labels %}
    <span class="label">{{ label }}</span>
  {% endfor %}
</div>
{% endfor %}
```
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_theme_files() {
        let files = get_theme_files();
        assert!(!files.is_empty());

        // Check required files exist
        let names: Vec<_> = files.iter().map(|f| f.name).collect();
        assert!(names.contains(&"base.html"));
        assert!(names.contains(&"index.html"));
        assert!(names.contains(&"spec.html"));
        assert!(names.contains(&"styles.css"));
    }

    #[test]
    fn test_init_theme() {
        let tmp = TempDir::new().unwrap();
        let theme_dir = tmp.path().join("theme");

        let result = init_theme(&theme_dir, false).unwrap();

        assert!(result.has_changes());
        assert!(result.skipped.is_empty());
        assert!(theme_dir.join("base.html").exists());
        assert!(theme_dir.join("styles.css").exists());
    }

    #[test]
    fn test_init_theme_skip_existing() {
        let tmp = TempDir::new().unwrap();
        let theme_dir = tmp.path().join("theme");

        // First init
        init_theme(&theme_dir, false).unwrap();

        // Second init should skip
        let result = init_theme(&theme_dir, false).unwrap();
        assert!(!result.has_changes());
        assert!(!result.skipped.is_empty());
    }

    #[test]
    fn test_init_theme_force() {
        let tmp = TempDir::new().unwrap();
        let theme_dir = tmp.path().join("theme");

        // First init
        init_theme(&theme_dir, false).unwrap();

        // Force should overwrite
        let result = init_theme(&theme_dir, true).unwrap();
        assert!(result.has_changes());
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn test_list_theme_files() {
        let tmp = TempDir::new().unwrap();
        let theme_dir = tmp.path().join("theme");

        init_theme(&theme_dir, false).unwrap();

        let files = list_theme_files(&theme_dir).unwrap();
        assert!(files.contains(&"base.html".to_string()));
        assert!(files.contains(&"styles.css".to_string()));
    }
}
