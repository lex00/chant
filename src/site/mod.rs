//! Static site generation for chant specs.
//!
//! This module provides functionality to generate a static HTML documentation
//! site from chant specs, including:
//! - Individual spec pages
//! - Index pages (by status, by label)
//! - Timeline visualization
//! - Dependency graph visualization
//! - Changelog

pub mod graph;
pub mod theme;
pub mod timeline;

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tera::Tera;

use crate::config::SiteConfig;
use crate::spec::{Spec, SpecStatus};

/// Embedded default theme templates
pub mod embedded {
    pub const BASE_HTML: &str = include_str!("../../templates/site/base.html");
    pub const INDEX_HTML: &str = include_str!("../../templates/site/index.html");
    pub const SPEC_HTML: &str = include_str!("../../templates/site/spec.html");
    pub const STATUS_INDEX_HTML: &str = include_str!("../../templates/site/status-index.html");
    pub const LABEL_INDEX_HTML: &str = include_str!("../../templates/site/label-index.html");
    pub const TIMELINE_HTML: &str = include_str!("../../templates/site/timeline.html");
    pub const GRAPH_HTML: &str = include_str!("../../templates/site/graph.html");
    pub const CHANGELOG_HTML: &str = include_str!("../../templates/site/changelog.html");
    pub const STYLES_CSS: &str = include_str!("../../templates/site/styles.css");
}

/// Site statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct SiteStats {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub pending: usize,
    pub failed: usize,
    pub other: usize,
}

impl SiteStats {
    pub fn from_specs(specs: &[&Spec]) -> Self {
        let mut stats = Self {
            total: specs.len(),
            completed: 0,
            in_progress: 0,
            pending: 0,
            failed: 0,
            other: 0,
        };

        for spec in specs {
            match spec.frontmatter.status {
                SpecStatus::Completed => stats.completed += 1,
                SpecStatus::InProgress => stats.in_progress += 1,
                SpecStatus::Pending => stats.pending += 1,
                SpecStatus::Failed => stats.failed += 1,
                _ => stats.other += 1,
            }
        }

        stats
    }
}

/// Spec data for templates
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpecTemplateData {
    pub id: String,
    pub short_id: String,
    pub title: Option<String>,
    pub status: String,
    pub r#type: String,
    pub labels: Vec<String>,
    pub depends_on: Vec<String>,
    pub target_files: Vec<String>,
    pub completed_at: Option<String>,
    pub model: Option<String>,
    pub body_html: String,
}

impl SpecTemplateData {
    pub fn from_spec(spec: &Spec, redacted_fields: &[String]) -> Self {
        // Extract short ID (last part after date)
        let short_id = spec
            .id
            .split('-')
            .next_back()
            .map(|s| s.to_string())
            .unwrap_or_else(|| spec.id.clone());

        // Convert body to HTML
        let body_html = markdown_to_html(&spec.body);

        // Get status as string
        let status = format!("{:?}", spec.frontmatter.status).to_lowercase();

        Self {
            id: spec.id.clone(),
            short_id,
            title: spec.title.clone(),
            status,
            r#type: spec.frontmatter.r#type.clone(),
            labels: spec.frontmatter.labels.clone().unwrap_or_default(),
            depends_on: if redacted_fields.contains(&"depends_on".to_string()) {
                vec![]
            } else {
                spec.frontmatter.depends_on.clone().unwrap_or_default()
            },
            target_files: if redacted_fields.contains(&"target_files".to_string()) {
                vec![]
            } else {
                spec.frontmatter.target_files.clone().unwrap_or_default()
            },
            completed_at: if redacted_fields.contains(&"completed_at".to_string()) {
                None
            } else {
                spec.frontmatter.completed_at.clone()
            },
            model: if redacted_fields.contains(&"model".to_string()) {
                None
            } else {
                spec.frontmatter.model.clone()
            },
            body_html,
        }
    }
}

/// Convert markdown to HTML using pulldown-cmark
fn markdown_to_html(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let options = Options::all();
    let parser = Parser::new_ext(markdown, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

/// Site generator
pub struct SiteGenerator {
    config: SiteConfig,
    tera: Tera,
    specs: Vec<Spec>,
}

impl SiteGenerator {
    /// Create a new site generator with the given configuration
    pub fn new(config: SiteConfig, specs: Vec<Spec>, theme_dir: Option<&Path>) -> Result<Self> {
        let tera = if let Some(dir) = theme_dir {
            if dir.exists() {
                // Load templates from theme directory
                let pattern = format!("{}/**/*.html", dir.display());
                Tera::new(&pattern)
                    .with_context(|| format!("Failed to load templates from {}", dir.display()))?
            } else {
                Self::create_embedded_tera()?
            }
        } else {
            Self::create_embedded_tera()?
        };

        Ok(Self {
            config,
            tera,
            specs,
        })
    }

    /// Create a Tera instance with embedded templates
    fn create_embedded_tera() -> Result<Tera> {
        let mut tera = Tera::default();

        tera.add_raw_template("base.html", embedded::BASE_HTML)?;
        tera.add_raw_template("index.html", embedded::INDEX_HTML)?;
        tera.add_raw_template("spec.html", embedded::SPEC_HTML)?;
        tera.add_raw_template("status-index.html", embedded::STATUS_INDEX_HTML)?;
        tera.add_raw_template("label-index.html", embedded::LABEL_INDEX_HTML)?;
        tera.add_raw_template("timeline.html", embedded::TIMELINE_HTML)?;
        tera.add_raw_template("graph.html", embedded::GRAPH_HTML)?;
        tera.add_raw_template("changelog.html", embedded::CHANGELOG_HTML)?;

        // Register custom filter for slugify
        tera.register_filter("slugify", slugify_filter);

        Ok(tera)
    }

    /// Filter specs based on configuration
    fn filter_specs(&self) -> Vec<&Spec> {
        self.specs
            .iter()
            .filter(|spec| {
                // Check public flag
                if let Some(false) = spec.frontmatter.public {
                    return false;
                }

                // Check status filter
                let status_str = format!("{:?}", spec.frontmatter.status).to_lowercase();
                if !self.config.include.statuses.is_empty()
                    && !self.config.include.statuses.contains(&status_str)
                {
                    return false;
                }

                // Check include labels (empty = include all)
                if !self.config.include.labels.is_empty() {
                    let spec_labels = spec.frontmatter.labels.as_ref();
                    if let Some(labels) = spec_labels {
                        if !labels
                            .iter()
                            .any(|l| self.config.include.labels.contains(l))
                        {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Check exclude labels
                if let Some(labels) = &spec.frontmatter.labels {
                    if labels
                        .iter()
                        .any(|l| self.config.exclude.labels.contains(l))
                    {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Collect all unique labels from filtered specs
    fn collect_labels(&self, specs: &[&Spec]) -> Vec<String> {
        let mut labels: HashSet<String> = HashSet::new();

        for spec in specs {
            if let Some(spec_labels) = &spec.frontmatter.labels {
                for label in spec_labels {
                    labels.insert(label.clone());
                }
            }
        }

        let mut labels: Vec<_> = labels.into_iter().collect();
        labels.sort();
        labels
    }

    /// Build the static site
    pub fn build(&self, output_dir: &Path) -> Result<BuildResult> {
        let mut result = BuildResult::default();

        // Create output directory structure
        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(output_dir.join("specs"))?;
        fs::create_dir_all(output_dir.join("status"))?;
        fs::create_dir_all(output_dir.join("labels"))?;

        // Filter specs
        let filtered_specs = self.filter_specs();
        let labels = self.collect_labels(&filtered_specs);
        let stats = SiteStats::from_specs(&filtered_specs);

        // Prepare common context
        let mut base_context = tera::Context::new();
        base_context.insert("site_title", &self.config.title);
        base_context.insert("base_url", &self.config.base_url);
        base_context.insert("features", &self.config.features);
        base_context.insert("labels", &labels);

        // Write CSS
        let css_content = if let Some(theme_path) = self.find_theme_file("styles.css") {
            fs::read_to_string(&theme_path)?
        } else {
            embedded::STYLES_CSS.to_string()
        };
        fs::write(output_dir.join("styles.css"), css_content)?;
        result.files_written += 1;

        // Generate individual spec pages
        let spec_data: Vec<SpecTemplateData> = filtered_specs
            .iter()
            .map(|s| SpecTemplateData::from_spec(s, &self.config.exclude.fields))
            .collect();

        for (i, spec) in spec_data.iter().enumerate() {
            let mut context = base_context.clone();
            context.insert("spec", spec);

            // Add prev/next navigation
            if i > 0 {
                context.insert("prev_spec", &spec_data[i - 1]);
            }
            if i < spec_data.len() - 1 {
                context.insert("next_spec", &spec_data[i + 1]);
            }

            let html = self.tera.render("spec.html", &context)?;
            let spec_path = output_dir.join("specs").join(format!("{}.html", spec.id));
            fs::write(&spec_path, html)?;
            result.files_written += 1;
        }

        // Generate index page
        {
            let mut context = base_context.clone();
            context.insert("specs", &spec_data);
            context.insert("stats", &stats);

            let html = self.tera.render("index.html", &context)?;
            fs::write(output_dir.join("index.html"), html)?;
            result.files_written += 1;
        }

        // Generate status index pages
        if self.config.features.status_indexes {
            for (status, display) in [
                ("completed", "Completed"),
                ("in_progress", "In Progress"),
                ("pending", "Pending"),
            ] {
                let status_specs: Vec<_> = spec_data
                    .iter()
                    .filter(|s| s.status == status)
                    .cloned()
                    .collect();

                let mut context = base_context.clone();
                context.insert("status", status);
                context.insert("status_display", display);
                context.insert("specs", &status_specs);

                let html = self.tera.render("status-index.html", &context)?;
                let filename = format!("{}.html", status.replace('_', "-"));
                fs::write(output_dir.join("status").join(&filename), html)?;
                result.files_written += 1;
            }
        }

        // Generate label index pages
        if self.config.features.label_indexes {
            for label in &labels {
                let label_specs: Vec<_> = spec_data
                    .iter()
                    .filter(|s| s.labels.contains(label))
                    .cloned()
                    .collect();

                let mut context = base_context.clone();
                context.insert("label", label);
                context.insert("specs", &label_specs);

                let html = self.tera.render("label-index.html", &context)?;
                let filename = format!("{}.html", slugify(label));
                fs::write(output_dir.join("labels").join(&filename), html)?;
                result.files_written += 1;
            }
        }

        // Generate timeline page
        if self.config.features.timeline {
            let timeline_groups = timeline::build_timeline_groups(
                &filtered_specs,
                self.config.timeline.group_by,
                self.config.timeline.include_pending,
            );

            let mut context = base_context.clone();
            context.insert("timeline_groups", &timeline_groups);

            let html = self.tera.render("timeline.html", &context)?;
            fs::write(output_dir.join("timeline.html"), html)?;
            result.files_written += 1;
        }

        // Generate dependency graph page
        if self.config.features.dependency_graph {
            let (ascii_graph, roots, leaves) =
                graph::build_dependency_graph(&filtered_specs, self.config.graph.detail);

            let roots_data: Vec<_> = roots
                .iter()
                .map(|id| {
                    spec_data
                        .iter()
                        .find(|s| &s.id == id)
                        .cloned()
                        .unwrap_or_else(|| SpecTemplateData {
                            id: id.clone(),
                            short_id: id.clone(),
                            title: None,
                            status: "unknown".to_string(),
                            r#type: "unknown".to_string(),
                            labels: vec![],
                            depends_on: vec![],
                            target_files: vec![],
                            completed_at: None,
                            model: None,
                            body_html: String::new(),
                        })
                })
                .collect();

            let leaves_data: Vec<_> = leaves
                .iter()
                .map(|id| {
                    spec_data
                        .iter()
                        .find(|s| &s.id == id)
                        .cloned()
                        .unwrap_or_else(|| SpecTemplateData {
                            id: id.clone(),
                            short_id: id.clone(),
                            title: None,
                            status: "unknown".to_string(),
                            r#type: "unknown".to_string(),
                            labels: vec![],
                            depends_on: vec![],
                            target_files: vec![],
                            completed_at: None,
                            model: None,
                            body_html: String::new(),
                        })
                })
                .collect();

            let mut context = base_context.clone();
            context.insert("ascii_graph", &ascii_graph);
            context.insert("roots", &roots_data);
            context.insert("leaves", &leaves_data);

            let html = self.tera.render("graph.html", &context)?;
            fs::write(output_dir.join("graph.html"), html)?;
            result.files_written += 1;
        }

        // Generate changelog page
        if self.config.features.changelog {
            let changelog_groups = build_changelog_groups(&filtered_specs);

            let changelog_data: Vec<HashMap<String, serde_json::Value>> = changelog_groups
                .into_iter()
                .map(|(date, specs)| {
                    let mut map = HashMap::new();
                    map.insert("date".to_string(), serde_json::json!(date));
                    let specs_data: Vec<_> = specs
                        .iter()
                        .map(|s| SpecTemplateData::from_spec(s, &self.config.exclude.fields))
                        .collect();
                    map.insert("specs".to_string(), serde_json::json!(specs_data));
                    map
                })
                .collect();

            let mut context = base_context.clone();
            context.insert("changelog_groups", &changelog_data);

            let html = self.tera.render("changelog.html", &context)?;
            fs::write(output_dir.join("changelog.html"), html)?;
            result.files_written += 1;
        }

        result.specs_included = filtered_specs.len();
        Ok(result)
    }

    /// Find a theme file in the custom theme directory
    fn find_theme_file(&self, filename: &str) -> Option<std::path::PathBuf> {
        let theme_dir = Path::new(".chant/site/theme");
        if theme_dir.exists() {
            let path = theme_dir.join(filename);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
}

/// Build changelog groups from completed specs
fn build_changelog_groups<'a>(specs: &[&'a Spec]) -> Vec<(String, Vec<&'a Spec>)> {
    let mut groups: HashMap<String, Vec<&'a Spec>> = HashMap::new();

    for spec in specs {
        if spec.frontmatter.status == SpecStatus::Completed {
            // Get date from completed_at or extract from ID
            let date = if let Some(completed_at) = &spec.frontmatter.completed_at {
                completed_at
                    .split('T')
                    .next()
                    .unwrap_or("Unknown")
                    .to_string()
            } else {
                // Extract date from ID (e.g., 2026-01-30-00a-xyz -> 2026-01-30)
                let parts: Vec<_> = spec.id.split('-').collect();
                if parts.len() >= 3 {
                    format!("{}-{}-{}", parts[0], parts[1], parts[2])
                } else {
                    "Unknown".to_string()
                }
            };

            groups.entry(date).or_default().push(*spec);
        }
    }

    // Sort by date descending
    let mut sorted: Vec<_> = groups.into_iter().collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0));
    sorted
}

/// Slugify a string for use in URLs
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Tera filter for slugify
fn slugify_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match value.as_str() {
        Some(s) => Ok(tera::Value::String(slugify(s))),
        None => Ok(value.clone()),
    }
}

/// Result of building the site
#[derive(Debug, Default)]
pub struct BuildResult {
    pub files_written: usize,
    pub specs_included: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("API_Integration"), "api-integration");
        assert_eq!(slugify("feature/auth"), "feature-auth");
        assert_eq!(slugify("multiple   spaces"), "multiple-spaces");
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "# Hello\n\nThis is **bold** text.";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }
}
