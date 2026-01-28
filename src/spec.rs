//! Spec parsing, frontmatter handling, and spec lifecycle management.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: concepts/specs.md, reference/schema.md
//! - ignore: false

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// Re-export group/driver functions from spec_group for backward compatibility
pub use crate::spec_group::{
    all_members_completed, all_prior_siblings_completed, auto_complete_driver_if_ready,
    extract_driver_id, extract_member_number, get_incomplete_members, get_members, is_member_of,
    mark_driver_in_progress,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Failed,
    NeedsAttention,
    Ready,
    Blocked,
    Cancelled,
}

/// Approval status for a spec
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
}

/// Approval information for a spec
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Approval {
    /// Whether approval is required for this spec
    #[serde(default)]
    pub required: bool,
    /// Current approval status
    #[serde(default)]
    pub status: ApprovalStatus,
    /// Name of the person who approved/rejected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Timestamp of approval/rejection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// Represents a dependency that is blocking a spec from being ready.
#[derive(Debug, Clone)]
pub struct BlockingDependency {
    /// The spec ID of the blocking dependency.
    pub spec_id: String,
    /// The title of the blocking dependency, if available.
    pub title: Option<String>,
    /// The current status of the blocking dependency.
    pub status: SpecStatus,
    /// When the dependency was completed, if applicable.
    pub completed_at: Option<String>,
    /// Whether this is a sibling dependency (from group ordering).
    pub is_sibling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecFrontmatter {
    #[serde(default = "default_type")]
    pub r#type: String,
    #[serde(default)]
    pub status: SpecStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    // Documentation-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<String>>,
    // Research-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub informed_by: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    // Conflict-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicting_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_specs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_spec: Option<String>,
    // Verification-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_failures: Option<Vec<String>>,
    // Replay tracking fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replayed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_completed_at: Option<String>,
    // Derivation tracking - which fields were automatically derived
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_fields: Option<Vec<String>>,
    // Approval workflow fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
}

fn default_type() -> String {
    "code".to_string()
}

impl Default for SpecFrontmatter {
    fn default() -> Self {
        Self {
            r#type: default_type(),
            status: SpecStatus::Pending,
            depends_on: None,
            labels: None,
            target_files: None,
            context: None,
            prompt: None,
            branch: None,
            commits: None,
            pr: None,
            completed_at: None,
            model: None,
            tracks: None,
            informed_by: None,
            origin: None,
            schedule: None,
            source_branch: None,
            target_branch: None,
            conflicting_files: None,
            blocked_specs: None,
            original_spec: None,
            last_verified: None,
            verification_status: None,
            verification_failures: None,
            replayed_at: None,
            replay_count: None,
            original_completed_at: None,
            derived_fields: None,
            approval: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub frontmatter: SpecFrontmatter,
    pub title: Option<String>,
    pub body: String,
}

impl Spec {
    /// Count unchecked checkboxes (`- [ ]`) in the Acceptance Criteria section only.
    /// Returns the count of unchecked items in that section, skipping code fences.
    /// Uses the LAST `## Acceptance Criteria` heading outside code fences.
    pub fn count_unchecked_checkboxes(&self) -> usize {
        let acceptance_criteria_marker = "## Acceptance Criteria";

        // First pass: find the line number of the LAST AC heading outside code fences
        let mut in_code_fence = false;
        let mut last_ac_line: Option<usize> = None;

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                last_ac_line = Some(line_num);
            }
        }

        let Some(ac_start) = last_ac_line else {
            return 0;
        };

        // Second pass: count checkboxes from the AC section until next ## heading
        let mut in_code_fence = false;
        let mut in_ac_section = false;
        let mut count = 0;

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if in_code_fence {
                continue;
            }

            // Start counting at the last AC heading we found
            if line_num == ac_start {
                in_ac_section = true;
                continue;
            }

            // Stop at the next ## heading after our AC section
            if in_ac_section && trimmed.starts_with("## ") {
                break;
            }

            if in_ac_section && line.contains("- [ ]") {
                count += line.matches("- [ ]").count();
            }
        }

        count
    }

    /// Count total checkboxes (both checked and unchecked) in the Acceptance Criteria section.
    /// Used to assess spec complexity.
    pub fn count_total_checkboxes(&self) -> usize {
        let acceptance_criteria_marker = "## Acceptance Criteria";

        // First pass: find the line number of the LAST AC heading outside code fences
        let mut in_code_fence = false;
        let mut last_ac_line: Option<usize> = None;

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                last_ac_line = Some(line_num);
            }
        }

        let Some(ac_start) = last_ac_line else {
            return 0;
        };

        // Second pass: count all checkboxes from the AC section until next ## heading
        let mut in_code_fence = false;
        let mut in_ac_section = false;
        let mut count = 0;

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
                continue;
            }

            if in_code_fence {
                continue;
            }

            if line_num == ac_start {
                in_ac_section = true;
                continue;
            }

            if in_ac_section && trimmed.starts_with("## ") {
                break;
            }

            // Count both unchecked and checked checkboxes
            if in_ac_section {
                count += line.matches("- [ ]").count();
                count += line.matches("- [x]").count();
                count += line.matches("- [X]").count();
            }
        }

        count
    }

    /// Parse a spec from file content.
    pub fn parse(id: &str, content: &str) -> Result<Self> {
        let (frontmatter_str, body) = split_frontmatter(content);

        let frontmatter: SpecFrontmatter = if let Some(fm) = frontmatter_str {
            serde_yaml::from_str(&fm).context("Failed to parse spec frontmatter")?
        } else {
            SpecFrontmatter::default()
        };

        // Extract title from first heading
        let title = extract_title(body);

        Ok(Self {
            id: id.to_string(),
            frontmatter,
            title,
            body: body.to_string(),
        })
    }

    /// Load a spec from a file path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read spec from {}", path.display()))?;

        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid spec filename"))?;

        Self::parse(id, &content)
    }

    /// Save the spec to a file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let frontmatter = serde_yaml::to_string(&self.frontmatter)?;
        let content = format!("---\n{}---\n{}", frontmatter, self.body);
        fs::write(path, content)?;
        Ok(())
    }

    /// Add derived fields to the spec's frontmatter.
    /// Updates the frontmatter with the provided derived fields.
    pub fn add_derived_fields(&mut self, fields: std::collections::HashMap<String, String>) {
        let mut derived_field_names = Vec::new();

        for (key, value) in fields {
            // Track which fields were derived
            derived_field_names.push(key.clone());

            // Map frontmatter field names - we need to be careful with reserved names
            // For now, we'll set generic custom fields
            // In practice, derived fields might map to existing frontmatter fields or custom ones
            // This implementation preserves the spec's type system

            // Handle specific known derived fields that map to frontmatter
            match key.as_str() {
                "labels" => {
                    // labels is already a Vec, so split on comma if multiple
                    let label_vec = value.split(',').map(|s| s.trim().to_string()).collect();
                    self.frontmatter.labels = Some(label_vec);
                }
                "context" => {
                    // context is a Vec of strings
                    let context_vec = value.split(',').map(|s| s.trim().to_string()).collect();
                    self.frontmatter.context = Some(context_vec);
                }
                // For other fields, we could extend the frontmatter struct
                // For now, we'll append them to the body as a note
                _ => {
                    // Store other derived fields in a way they can be captured
                    // One approach is to add them to the body as metadata
                    // Another is to extend SpecFrontmatter to include a custom map
                    // For this implementation, we'll store in context if empty
                    if self.frontmatter.context.is_none() {
                        self.frontmatter.context = Some(vec![]);
                    }
                    if let Some(ref mut ctx) = self.frontmatter.context {
                        ctx.push(format!("derived_{}={}", key, value));
                    }
                }
            }
        }

        // Update the derived_fields tracking
        if !derived_field_names.is_empty() {
            self.frontmatter.derived_fields = Some(derived_field_names);
        }
    }

    /// Check if this spec has unmet dependencies that would block it.
    /// Returns true if the spec has dependencies pointing to incomplete specs.
    /// Note: This only checks local dependencies. For cross-repo dependencies,
    /// use the is_blocked method from the deps module.
    pub fn is_blocked(&self, all_specs: &[Spec]) -> bool {
        // Check dependencies are all completed
        if let Some(deps) = &self.frontmatter.depends_on {
            for dep_id in deps {
                let dep = all_specs.iter().find(|s| s.id == *dep_id);
                match dep {
                    Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                    _ => return true, // Found an unmet dependency
                }
            }
        }

        false
    }

    /// Check if this spec is ready to execute.
    pub fn is_ready(&self, all_specs: &[Spec]) -> bool {
        // Must be pending and not blocked
        if self.frontmatter.status != SpecStatus::Pending {
            return false;
        }

        // Check dependencies are completed
        if self.is_blocked(all_specs) {
            return false;
        }

        // Check that all prior siblings are completed (if this is a member spec)
        if !all_prior_siblings_completed(&self.id, all_specs) {
            return false;
        }

        // Check group members are completed (if this is a driver)
        let members: Vec<_> = all_specs
            .iter()
            .filter(|s| is_member_of(&s.id, &self.id))
            .collect();

        if !members.is_empty() {
            for member in members {
                if member.frontmatter.status != SpecStatus::Completed {
                    return false;
                }
            }
        }

        true
    }

    /// Get the list of dependencies that are blocking this spec.
    ///
    /// This method reloads each dependency spec from disk to get the most current status,
    /// falling back to the in-memory spec data if the file cannot be read.
    ///
    /// Returns a list of blocking dependencies with their current status information.
    pub fn get_blocking_dependencies(
        &self,
        all_specs: &[Spec],
        specs_dir: &Path,
    ) -> Vec<BlockingDependency> {
        let mut blockers = Vec::new();

        // Check explicit depends_on dependencies
        if let Some(deps) = &self.frontmatter.depends_on {
            for dep_id in deps {
                // Try to reload from disk for fresh status
                let spec_path = specs_dir.join(format!("{}.md", dep_id));
                let dep_spec = if spec_path.exists() {
                    Spec::load(&spec_path).ok()
                } else {
                    None
                };

                // Fall back to in-memory spec if disk load fails
                let dep_spec =
                    dep_spec.or_else(|| all_specs.iter().find(|s| s.id == *dep_id).cloned());

                if let Some(spec) = dep_spec {
                    // Only add if not completed
                    if spec.frontmatter.status != SpecStatus::Completed {
                        blockers.push(BlockingDependency {
                            spec_id: spec.id.clone(),
                            title: spec.title.clone(),
                            status: spec.frontmatter.status.clone(),
                            completed_at: spec.frontmatter.completed_at.clone(),
                            is_sibling: false,
                        });
                    } else if spec.frontmatter.status == SpecStatus::Completed {
                        // Dependency is complete but spec still shows as blocked - flag this
                        blockers.push(BlockingDependency {
                            spec_id: spec.id.clone(),
                            title: spec.title.clone(),
                            status: spec.frontmatter.status.clone(),
                            completed_at: spec.frontmatter.completed_at.clone(),
                            is_sibling: false,
                        });
                    }
                } else {
                    // Dependency not found
                    blockers.push(BlockingDependency {
                        spec_id: dep_id.clone(),
                        title: None,
                        status: SpecStatus::Pending, // Unknown status, use Pending
                        completed_at: None,
                        is_sibling: false,
                    });
                }
            }
        }

        // Check prior siblings for member specs
        if let Some(driver_id) = extract_driver_id(&self.id) {
            if let Some(member_num) = extract_member_number(&self.id) {
                for i in 1..member_num {
                    let sibling_id = format!("{}.{}", driver_id, i);

                    // Try to reload from disk for fresh status
                    let spec_path = specs_dir.join(format!("{}.md", sibling_id));
                    let sibling_spec = if spec_path.exists() {
                        Spec::load(&spec_path).ok()
                    } else {
                        None
                    };

                    // Fall back to in-memory spec
                    let sibling_spec = sibling_spec
                        .or_else(|| all_specs.iter().find(|s| s.id == sibling_id).cloned());

                    if let Some(spec) = sibling_spec {
                        if spec.frontmatter.status != SpecStatus::Completed {
                            blockers.push(BlockingDependency {
                                spec_id: spec.id.clone(),
                                title: spec.title.clone(),
                                status: spec.frontmatter.status.clone(),
                                completed_at: spec.frontmatter.completed_at.clone(),
                                is_sibling: true,
                            });
                        }
                    } else {
                        // Sibling not found
                        blockers.push(BlockingDependency {
                            spec_id: sibling_id,
                            title: None,
                            status: SpecStatus::Pending,
                            completed_at: None,
                            is_sibling: true,
                        });
                    }
                }
            }
        }

        blockers
    }

    /// Check if the spec's frontmatter contains a specific field.
    /// Returns true if the field exists and has a non-None value.
    pub fn has_frontmatter_field(&self, field: &str) -> bool {
        match field {
            "type" => true,   // type always has a value (defaults to "code")
            "status" => true, // status always has a value (defaults to Pending)
            "depends_on" => self.frontmatter.depends_on.is_some(),
            "labels" => self.frontmatter.labels.is_some(),
            "target_files" => self.frontmatter.target_files.is_some(),
            "context" => self.frontmatter.context.is_some(),
            "prompt" => self.frontmatter.prompt.is_some(),
            "branch" => self.frontmatter.branch.is_some(),
            "commits" => self.frontmatter.commits.is_some(),
            "pr" => self.frontmatter.pr.is_some(),
            "completed_at" => self.frontmatter.completed_at.is_some(),
            "model" => self.frontmatter.model.is_some(),
            "tracks" => self.frontmatter.tracks.is_some(),
            "informed_by" => self.frontmatter.informed_by.is_some(),
            "origin" => self.frontmatter.origin.is_some(),
            "schedule" => self.frontmatter.schedule.is_some(),
            "source_branch" => self.frontmatter.source_branch.is_some(),
            "target_branch" => self.frontmatter.target_branch.is_some(),
            "conflicting_files" => self.frontmatter.conflicting_files.is_some(),
            "blocked_specs" => self.frontmatter.blocked_specs.is_some(),
            "original_spec" => self.frontmatter.original_spec.is_some(),
            "last_verified" => self.frontmatter.last_verified.is_some(),
            "verification_status" => self.frontmatter.verification_status.is_some(),
            "verification_failures" => self.frontmatter.verification_failures.is_some(),
            "replayed_at" => self.frontmatter.replayed_at.is_some(),
            "replay_count" => self.frontmatter.replay_count.is_some(),
            "original_completed_at" => self.frontmatter.original_completed_at.is_some(),
            "approval" => self.frontmatter.approval.is_some(),
            _ => false, // Unknown field
        }
    }

    /// Check if this spec requires approval before work can begin.
    /// Returns true if approval is required AND the spec is not yet approved.
    pub fn requires_approval(&self) -> bool {
        if let Some(ref approval) = self.frontmatter.approval {
            approval.required && approval.status != ApprovalStatus::Approved
        } else {
            false
        }
    }

    /// Check if this spec has been approved.
    pub fn is_approved(&self) -> bool {
        if let Some(ref approval) = self.frontmatter.approval {
            approval.status == ApprovalStatus::Approved
        } else {
            // No approval section means no approval required (implicitly approved)
            true
        }
    }

    /// Check if this spec has been rejected.
    pub fn is_rejected(&self) -> bool {
        if let Some(ref approval) = self.frontmatter.approval {
            approval.status == ApprovalStatus::Rejected
        } else {
            false
        }
    }
}

/// Split content into frontmatter and body.
///
/// If the content starts with `---`, extracts the YAML frontmatter between
/// the first and second `---` delimiters, and returns the body after.
/// Otherwise returns None for frontmatter and the entire content as body.
pub fn split_frontmatter(content: &str) -> (Option<String>, &str) {
    let content = content.trim();

    if !content.starts_with("---") {
        return (None, content);
    }

    let rest = &content[3..];
    if let Some(end) = rest.find("---") {
        let frontmatter = rest[..end].to_string();
        let body = rest[end + 3..].trim_start();
        (Some(frontmatter), body)
    } else {
        (None, content)
    }
}

fn extract_title(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.to_string());
        }
    }
    None
}

/// Apply blocked status to specs with unmet dependencies.
/// For pending specs that have incomplete dependencies, updates their status to blocked.
/// This is a local-only version that only checks dependencies within the current repo.
fn apply_blocked_status(specs: &mut [Spec]) {
    apply_blocked_status_with_repos(specs, std::path::Path::new(".chant/specs"), &[]);
}

/// Apply blocked status considering both local and cross-repo dependencies.
/// This version supports cross-repo dependency checking when repos config is available.
#[allow(dead_code)]
pub fn apply_blocked_status_with_repos(
    specs: &mut [Spec],
    specs_dir: &std::path::Path,
    repos: &[crate::config::RepoConfig],
) {
    // Build a reference list of specs for dependency checking
    let specs_snapshot = specs.to_vec();

    for spec in specs.iter_mut() {
        // Handle both Pending and Blocked specs
        if spec.frontmatter.status != SpecStatus::Pending
            && spec.frontmatter.status != SpecStatus::Blocked
        {
            continue;
        }

        // Check if this spec has unmet dependencies (local only)
        let is_blocked_locally = spec.is_blocked(&specs_snapshot);

        // Check cross-repo dependencies if repos config is available
        let is_blocked_cross_repo = !repos.is_empty()
            && crate::deps::is_blocked_by_dependencies(spec, &specs_snapshot, specs_dir, repos);

        if is_blocked_locally || is_blocked_cross_repo {
            // Has unmet dependencies - mark as blocked
            spec.frontmatter.status = SpecStatus::Blocked;
        } else if spec.frontmatter.status == SpecStatus::Blocked {
            // No unmet dependencies and was previously blocked - revert to pending
            spec.frontmatter.status = SpecStatus::Pending;
        }
    }
}

pub fn load_all_specs(specs_dir: &Path) -> Result<Vec<Spec>> {
    let mut specs = Vec::new();

    if !specs_dir.exists() {
        return Ok(specs);
    }

    load_specs_recursive(specs_dir, &mut specs)?;

    // Apply blocked status to specs with unmet dependencies
    apply_blocked_status(&mut specs);

    Ok(specs)
}

/// Recursively load specs from a directory and its subdirectories.
fn load_specs_recursive(dir: &Path, specs: &mut Vec<Spec>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            // Recursively load from subdirectories
            load_specs_recursive(&path, specs)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            match Spec::load(&path) {
                Ok(spec) => specs.push(spec),
                Err(e) => {
                    eprintln!("Warning: Failed to load spec {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(())
}

/// Resolve a partial spec ID to a full spec.
/// Searches both active specs and archived specs.
pub fn resolve_spec(specs_dir: &Path, partial_id: &str) -> Result<Spec> {
    let mut specs = load_all_specs(specs_dir)?;

    // Also load archived specs
    let archive_dir = specs_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine archive directory"))?
        .join("archive");
    if archive_dir.exists() {
        let archived_specs = load_all_specs(&archive_dir)?;
        specs.extend(archived_specs);
    }

    // Exact match
    if let Some(spec) = specs.iter().find(|s| s.id == partial_id) {
        return Ok(spec.clone());
    }

    // Suffix match (random suffix)
    let suffix_matches: Vec<_> = specs
        .iter()
        .filter(|s| s.id.ends_with(partial_id))
        .collect();
    if suffix_matches.len() == 1 {
        return Ok(suffix_matches[0].clone());
    }

    // Sequence match for today (e.g., "001")
    if partial_id.len() == 3 {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_pattern = format!("{}-{}-", today, partial_id);
        let today_matches: Vec<_> = specs
            .iter()
            .filter(|s| s.id.starts_with(&today_pattern))
            .collect();
        if today_matches.len() == 1 {
            return Ok(today_matches[0].clone());
        }
    }

    // Partial date match (e.g., "22-001" or "01-22-001")
    let partial_matches: Vec<_> = specs.iter().filter(|s| s.id.contains(partial_id)).collect();
    if partial_matches.len() == 1 {
        return Ok(partial_matches[0].clone());
    }

    if partial_matches.len() > 1 {
        anyhow::bail!(
            "Ambiguous spec ID '{}'. Matches: {}",
            partial_id,
            partial_matches
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    anyhow::bail!("Spec not found: {}", partial_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spec() {
        let content = r#"---
type: code
status: pending
---

# Fix the bug

Description here.
"#;
        let spec = Spec::parse("2026-01-22-001-x7m", content).unwrap();
        assert_eq!(spec.id, "2026-01-22-001-x7m");
        assert_eq!(spec.frontmatter.status, SpecStatus::Pending);
        assert_eq!(spec.title, Some("Fix the bug".to_string()));
    }

    #[test]
    fn test_spec_is_ready() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        assert!(spec.is_ready(&[]));
    }

    #[test]
    fn test_spec_not_ready_if_in_progress() {
        let spec = Spec::parse(
            "001",
            r#"---
status: in_progress
---
# Test
"#,
        )
        .unwrap();

        assert!(!spec.is_ready(&[]));
    }

    #[test]
    fn test_count_unchecked_checkboxes() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Acceptance Criteria

- [ ] First unchecked item
- [x] Checked item
- [ ] Second unchecked item
- [X] Another checked item
"#,
        )
        .unwrap();

        assert_eq!(spec.count_unchecked_checkboxes(), 2);
    }

    #[test]
    fn test_count_unchecked_checkboxes_none() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Acceptance Criteria

- [x] All checked
- [X] Also checked
"#,
        )
        .unwrap();

        assert_eq!(spec.count_unchecked_checkboxes(), 0);
    }

    #[test]
    fn test_count_unchecked_checkboxes_skip_code_blocks() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Expected Format

```markdown
## Acceptance Criteria

- [ ] Example checkbox in code block
```

## Acceptance Criteria

- [x] Real checkbox
- [ ] Another real unchecked
"#,
        )
        .unwrap();

        // Should only count the 1 unchecked in the real Acceptance Criteria section
        // The one in the code block should be ignored
        assert_eq!(spec.count_unchecked_checkboxes(), 1);
    }

    #[test]
    fn test_count_unchecked_checkboxes_nested_code_blocks() {
        // This tests the pattern from spec 01j where there are multiple code fences
        // and the first ## Acceptance Criteria appears inside a code block example
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Example

```markdown
## Expected Format

```markdown
## Acceptance Criteria

- [ ] Example checkbox in code block
```

## Acceptance Criteria

- [x] Real checkbox
```

## Acceptance Criteria

- [x] First real checked
- [ ] Second real unchecked
"#,
        )
        .unwrap();

        // The real Acceptance Criteria section at the end has 1 unchecked checkbox
        // The ones inside the code block examples should all be ignored
        assert_eq!(spec.count_unchecked_checkboxes(), 1);
    }

    #[test]
    fn test_parse_spec_with_model() {
        let content = r#"---
type: code
status: completed
commits:
  - abc1234
completed_at: 2026-01-24T15:30:00Z
model: claude-opus-4-5
---

# Test spec with model

Description here.
"#;
        let spec = Spec::parse("2026-01-24-001-abc", content).unwrap();
        assert_eq!(spec.frontmatter.model, Some("claude-opus-4-5".to_string()));
        assert_eq!(spec.frontmatter.status, SpecStatus::Completed);
        assert_eq!(spec.frontmatter.commits, Some(vec!["abc1234".to_string()]));
    }

    #[test]
    fn test_parse_spec_without_model() {
        let content = r#"---
type: code
status: pending
---

# Test spec without model
"#;
        let spec = Spec::parse("2026-01-24-002-def", content).unwrap();
        assert_eq!(spec.frontmatter.model, None);
    }

    #[test]
    fn test_spec_save_includes_model() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-spec.md");

        let spec = Spec {
            id: "2026-01-24-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                model: Some("claude-opus-4-5".to_string()),
                commits: Some(vec!["abc1234".to_string()]),
                ..Default::default()
            },
            title: Some("Test spec".to_string()),
            body: "# Test spec\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(saved_content.contains("model: claude-opus-4-5"));
        assert!(saved_content.contains("commits:"));
    }

    #[test]
    fn test_mark_driver_in_progress_when_member_starts() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is pending
        let driver_spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Driver spec".to_string()),
            body: "# Driver spec\n\nBody content.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-001-abc.md");
        driver_spec.save(&driver_path).unwrap();

        // Mark driver as in_progress when member starts
        mark_driver_in_progress(specs_dir, "2026-01-24-001-abc.1").unwrap();

        // Verify driver status was updated to in_progress
        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::InProgress);
    }

    #[test]
    fn test_mark_driver_in_progress_skips_if_already_in_progress() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is already in_progress
        let driver_spec = Spec {
            id: "2026-01-24-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Driver spec".to_string()),
            body: "# Driver spec\n\nBody content.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-002-def.md");
        driver_spec.save(&driver_path).unwrap();

        // Try to mark driver as in_progress
        mark_driver_in_progress(specs_dir, "2026-01-24-002-def.1").unwrap();

        // Verify driver status is still in_progress (not changed)
        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::InProgress);
    }

    #[test]
    fn test_is_ready_checks_prior_siblings() {
        // Create a sequence where .2 depends on .1 being completed
        let spec1 = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: completed
---
# Test
"#,
        )
        .unwrap();

        let spec2_ready = Spec::parse(
            "2026-01-24-001-abc.2",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let all_specs = vec![spec1];
        assert!(spec2_ready.is_ready(&all_specs));

        // Now make spec1 not completed
        let spec1_incomplete = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let spec2_not_ready = Spec::parse(
            "2026-01-24-001-abc.2",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let all_specs = vec![spec1_incomplete];
        assert!(!spec2_not_ready.is_ready(&all_specs));
    }

    #[test]
    fn test_parse_spec_with_labels() {
        let content = r#"---
type: code
status: pending
labels:
  - foo
  - bar
---

# Test spec with labels
"#;
        let spec = Spec::parse("2026-01-24-012-xyz", content).unwrap();
        assert_eq!(
            spec.frontmatter.labels,
            Some(vec!["foo".to_string(), "bar".to_string()])
        );
    }

    #[test]
    fn test_spec_save_persistence_with_labels() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-labels.md");

        let spec = Spec {
            id: "2026-01-24-013-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                labels: Some(vec!["test".to_string(), "validation".to_string()]),
                ..Default::default()
            },
            title: Some("Test labels".to_string()),
            body: "# Test labels\n\nBody content.".to_string(),
        };

        // Save the spec
        spec.save(&spec_path).unwrap();

        // Load it back
        let loaded_spec = Spec::load(&spec_path).unwrap();

        // Verify labels were persisted
        assert_eq!(
            loaded_spec.frontmatter.labels,
            Some(vec!["test".to_string(), "validation".to_string()])
        );
    }

    #[test]
    fn test_spec_save_without_labels() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-no-labels.md");

        let spec = Spec {
            id: "2026-01-24-014-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Test no labels".to_string()),
            body: "# Test no labels\n\nBody content.".to_string(),
        };

        // Save the spec
        spec.save(&spec_path).unwrap();

        // Load it back
        let loaded_spec = Spec::load(&spec_path).unwrap();

        // Verify labels are None
        assert_eq!(loaded_spec.frontmatter.labels, None);
    }

    #[test]
    fn test_resolve_spec_finds_archived_specs() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        let archive_dir = temp_dir.path().join("archive").join("2026-01-24");

        // Create specs directory structure
        fs::create_dir_all(&specs_dir).unwrap();
        fs::create_dir_all(&archive_dir).unwrap();

        // Create an archived spec
        let archived_spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Archived spec".to_string()),
            body: "# Archived spec\n\nArchived content.".to_string(),
        };

        let archived_path = archive_dir.join("2026-01-24-001-abc.md");
        archived_spec.save(&archived_path).unwrap();

        // Try to resolve the archived spec
        let resolved = resolve_spec(&specs_dir, "2026-01-24-001-abc").unwrap();
        assert_eq!(resolved.id, "2026-01-24-001-abc");
        assert_eq!(resolved.frontmatter.status, SpecStatus::Completed);
    }

    #[test]
    fn test_resolve_spec_finds_archived_by_partial_id() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        let archive_dir = temp_dir.path().join("archive").join("2026-01-24");

        // Create specs directory structure
        fs::create_dir_all(&specs_dir).unwrap();
        fs::create_dir_all(&archive_dir).unwrap();

        // Create an archived spec
        let archived_spec = Spec {
            id: "2026-01-24-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Another archived spec".to_string()),
            body: "# Another archived spec\n\nArchived content.".to_string(),
        };

        let archived_path = archive_dir.join("2026-01-24-002-def.md");
        archived_spec.save(&archived_path).unwrap();

        // Try to resolve by suffix
        let resolved = resolve_spec(&specs_dir, "def").unwrap();
        assert_eq!(resolved.id, "2026-01-24-002-def");
    }

    #[test]
    fn test_resolve_spec_prioritizes_active_over_archived() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        let archive_dir = temp_dir.path().join("archive").join("2026-01-24");

        // Create directory structure
        fs::create_dir_all(&specs_dir).unwrap();
        fs::create_dir_all(&archive_dir).unwrap();

        // Create an active spec
        let active_spec = Spec {
            id: "2026-01-24-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Active spec".to_string()),
            body: "# Active spec\n\nActive content.".to_string(),
        };

        let active_path = specs_dir.join("2026-01-24-003-ghi.md");
        active_spec.save(&active_path).unwrap();

        // Create an archived spec with similar ID
        let archived_spec = Spec {
            id: "2026-01-24-003-xyz".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Old spec".to_string()),
            body: "# Old spec\n\nArchived content.".to_string(),
        };

        let archived_path = archive_dir.join("2026-01-24-003-xyz.md");
        archived_spec.save(&archived_path).unwrap();

        // Resolve should find the active one
        let resolved = resolve_spec(&specs_dir, "2026-01-24-003-ghi").unwrap();
        assert_eq!(resolved.id, "2026-01-24-003-ghi");
        assert_eq!(resolved.frontmatter.status, SpecStatus::Pending);
    }

    #[test]
    fn test_parse_documentation_spec_with_tracks() {
        let content = r#"---
type: documentation
status: pending
tracks:
  - src/auth/*.rs
  - src/lib.rs
target_files:
  - docs/auth.md
---

# Document auth module

Description here.
"#;
        let spec = Spec::parse("2026-01-26-001-abc", content).unwrap();
        assert_eq!(spec.frontmatter.r#type, "documentation");
        assert_eq!(
            spec.frontmatter.tracks,
            Some(vec!["src/auth/*.rs".to_string(), "src/lib.rs".to_string()])
        );
        assert_eq!(
            spec.frontmatter.target_files,
            Some(vec!["docs/auth.md".to_string()])
        );
    }

    #[test]
    fn test_parse_research_spec_with_origin_and_informed_by() {
        let content = r#"---
type: research
status: pending
origin:
  - data/metrics.csv
informed_by:
  - docs/schema.md
  - src/**/*.rs
target_files:
  - analysis/findings.md
---

# Analyze metrics

Description here.
"#;
        let spec = Spec::parse("2026-01-26-002-def", content).unwrap();
        assert_eq!(spec.frontmatter.r#type, "research");
        assert_eq!(
            spec.frontmatter.origin,
            Some(vec!["data/metrics.csv".to_string()])
        );
        assert_eq!(
            spec.frontmatter.informed_by,
            Some(vec![
                "docs/schema.md".to_string(),
                "src/**/*.rs".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_research_spec_with_schedule() {
        let content = r#"---
type: research
status: pending
informed_by:
  - docs/*.md
schedule: weekly
target_files:
  - reports/weekly.md
---

# Weekly analysis

Description here.
"#;
        let spec = Spec::parse("2026-01-26-003-ghi", content).unwrap();
        assert_eq!(spec.frontmatter.schedule, Some("weekly".to_string()));
    }

    #[test]
    fn test_spec_save_includes_new_fields() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-new-fields.md");

        let spec = Spec {
            id: "2026-01-26-004-jkl".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "research".to_string(),
                status: SpecStatus::Pending,
                origin: Some(vec!["data/input.csv".to_string()]),
                informed_by: Some(vec!["docs/reference.md".to_string()]),
                schedule: Some("daily".to_string()),
                target_files: Some(vec!["output/report.md".to_string()]),
                ..Default::default()
            },
            title: Some("Research spec".to_string()),
            body: "# Research spec\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(saved_content.contains("origin:"));
        assert!(saved_content.contains("informed_by:"));
        assert!(saved_content.contains("schedule: daily"));
    }

    #[test]
    fn test_documentation_spec_tracks_field_roundtrip() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-tracks.md");

        let spec = Spec {
            id: "2026-01-26-005-mno".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "documentation".to_string(),
                status: SpecStatus::Pending,
                tracks: Some(vec!["src/**/*.rs".to_string()]),
                target_files: Some(vec!["docs/api.md".to_string()]),
                ..Default::default()
            },
            title: Some("Doc spec".to_string()),
            body: "# Doc spec\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();
        let loaded_spec = Spec::load(&spec_path).unwrap();

        assert_eq!(loaded_spec.frontmatter.r#type, "documentation");
        assert_eq!(
            loaded_spec.frontmatter.tracks,
            Some(vec!["src/**/*.rs".to_string()])
        );
    }

    #[test]
    fn test_is_blocked_with_unmet_dependencies() {
        // Create a pending spec with unmet dependencies
        let spec_with_unmet = Spec::parse(
            "2026-01-26-001-abc",
            r#"---
status: pending
depends_on:
  - 2026-01-26-002-def
---
# Test
"#,
        )
        .unwrap();

        // Create the dependency as pending (not completed)
        let dependency_pending = Spec::parse(
            "2026-01-26-002-def",
            r#"---
status: pending
---
# Dependency
"#,
        )
        .unwrap();

        let all_specs = vec![dependency_pending];

        // The spec should be blocked because dependency is not completed
        assert!(spec_with_unmet.is_blocked(&all_specs));
    }

    #[test]
    fn test_is_not_blocked_with_met_dependencies() {
        // Create a pending spec with met dependencies
        let spec_with_met = Spec::parse(
            "2026-01-26-001-abc",
            r#"---
status: pending
depends_on:
  - 2026-01-26-002-def
---
# Test
"#,
        )
        .unwrap();

        // Create the dependency as completed
        let dependency_completed = Spec::parse(
            "2026-01-26-002-def",
            r#"---
status: completed
---
# Dependency
"#,
        )
        .unwrap();

        let all_specs = vec![dependency_completed];

        // The spec should not be blocked because dependency is completed
        assert!(!spec_with_met.is_blocked(&all_specs));
    }

    #[test]
    fn test_apply_blocked_status_to_specs_with_unmet_deps() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a completed spec
        let completed_spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Completed".to_string()),
            body: "# Completed\n\nBody.".to_string(),
        };

        // Create a pending spec with unmet dependency
        let pending_with_unmet = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-26-003-ghi".to_string()]),
                ..Default::default()
            },
            title: Some("Pending with unmet".to_string()),
            body: "# Pending with unmet\n\nBody.".to_string(),
        };

        // Create an incomplete spec that blocks the above
        let incomplete_dep = Spec {
            id: "2026-01-26-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Incomplete dependency".to_string()),
            body: "# Incomplete dependency\n\nBody.".to_string(),
        };

        // Save all specs
        completed_spec
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        pending_with_unmet
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();
        incomplete_dep
            .save(&specs_dir.join("2026-01-26-003-ghi.md"))
            .unwrap();

        // Load all specs (this applies blocked status)
        let specs = load_all_specs(specs_dir).unwrap();

        // Find the spec with unmet dependency
        let spec_with_unmet = specs.iter().find(|s| s.id == "2026-01-26-002-def").unwrap();

        // It should now have status Blocked
        assert_eq!(spec_with_unmet.frontmatter.status, SpecStatus::Blocked);

        // The completed and pending specs should keep their original status
        let completed = specs.iter().find(|s| s.id == "2026-01-26-001-abc").unwrap();
        assert_eq!(completed.frontmatter.status, SpecStatus::Completed);

        let incomplete = specs.iter().find(|s| s.id == "2026-01-26-003-ghi").unwrap();
        assert_eq!(incomplete.frontmatter.status, SpecStatus::Pending);
    }

    #[test]
    fn test_spec_not_ready_if_blocked() {
        // Create a blocked spec
        let blocked_spec = Spec::parse(
            "2026-01-26-001-abc",
            r#"---
status: blocked
---
# Blocked spec
"#,
        )
        .unwrap();

        // A blocked spec should not be ready
        assert!(!blocked_spec.is_ready(&[]));
    }

    #[test]
    fn test_apply_blocked_status_only_to_pending_with_unmet_deps() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a completed spec with unmet dependency (should not change to blocked)
        let completed_with_unmet = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                depends_on: Some(vec!["2026-01-26-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("Completed with unmet".to_string()),
            body: "# Completed\n\nBody.".to_string(),
        };

        // Create an incomplete dependency
        let incomplete_dep = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Incomplete dependency".to_string()),
            body: "# Incomplete dependency\n\nBody.".to_string(),
        };

        // Save specs
        completed_with_unmet
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        incomplete_dep
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();

        // Load all specs
        let specs = load_all_specs(specs_dir).unwrap();

        // The completed spec should remain completed, not change to blocked
        let completed = specs.iter().find(|s| s.id == "2026-01-26-001-abc").unwrap();
        assert_eq!(completed.frontmatter.status, SpecStatus::Completed);
    }

    #[test]
    fn test_parse_spec_with_all_verification_fields() {
        let content = r#"---
type: code
status: pending
last_verified: 2026-01-26T10:30:00Z
verification_status: passed
verification_failures:
  - test_case_1
  - test_case_2
---

# Spec with verification fields

Description here.
"#;
        let spec = Spec::parse("2026-01-26-001-abc", content).unwrap();
        assert_eq!(
            spec.frontmatter.last_verified,
            Some("2026-01-26T10:30:00Z".to_string())
        );
        assert_eq!(
            spec.frontmatter.verification_status,
            Some("passed".to_string())
        );
        assert_eq!(
            spec.frontmatter.verification_failures,
            Some(vec!["test_case_1".to_string(), "test_case_2".to_string()])
        );
    }

    #[test]
    fn test_parse_spec_without_verification_fields() {
        let content = r#"---
type: code
status: pending
---

# Legacy spec without verification fields

Description here.
"#;
        let spec = Spec::parse("2026-01-26-002-def", content).unwrap();
        assert_eq!(spec.frontmatter.last_verified, None);
        assert_eq!(spec.frontmatter.verification_status, None);
        assert_eq!(spec.frontmatter.verification_failures, None);
    }

    #[test]
    fn test_parse_spec_with_only_last_verified() {
        let content = r#"---
type: code
status: pending
last_verified: 2026-01-25T15:00:00Z
---

# Spec with only last_verified

Description here.
"#;
        let spec = Spec::parse("2026-01-26-003-ghi", content).unwrap();
        assert_eq!(
            spec.frontmatter.last_verified,
            Some("2026-01-25T15:00:00Z".to_string())
        );
        assert_eq!(spec.frontmatter.verification_status, None);
        assert_eq!(spec.frontmatter.verification_failures, None);
    }

    #[test]
    fn test_parse_spec_with_empty_verification_failures() {
        let content = r#"---
type: code
status: pending
verification_failures: []
---

# Spec with empty verification_failures

Description here.
"#;
        let spec = Spec::parse("2026-01-26-004-jkl", content).unwrap();
        assert_eq!(spec.frontmatter.verification_failures, Some(vec![]));
    }

    #[test]
    fn test_spec_roundtrip_with_verification_fields() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-verification.md");

        let spec = Spec {
            id: "2026-01-26-005-mno".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                last_verified: Some("2026-01-26T12:00:00Z".to_string()),
                verification_status: Some("passed".to_string()),
                verification_failures: Some(vec!["failure_1".to_string()]),
                ..Default::default()
            },
            title: Some("Verification test".to_string()),
            body: "# Verification test\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let loaded_spec = Spec::load(&spec_path).unwrap();

        assert_eq!(
            loaded_spec.frontmatter.last_verified,
            Some("2026-01-26T12:00:00Z".to_string())
        );
        assert_eq!(
            loaded_spec.frontmatter.verification_status,
            Some("passed".to_string())
        );
        assert_eq!(
            loaded_spec.frontmatter.verification_failures,
            Some(vec!["failure_1".to_string()])
        );
    }

    #[test]
    fn test_spec_save_without_verification_fields() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-no-verification.md");

        let spec = Spec {
            id: "2026-01-26-006-pqr".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("No verification".to_string()),
            body: "# No verification\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(!saved_content.contains("last_verified:"));
        assert!(!saved_content.contains("verification_status:"));
        assert!(!saved_content.contains("verification_failures:"));
    }

    #[test]
    fn test_parse_spec_with_partial_verification_fields() {
        let content = r#"---
type: code
status: pending
verification_status: in_progress
verification_failures:
  - test_a
---

# Partial verification fields

Description here.
"#;
        let spec = Spec::parse("2026-01-26-007-stu", content).unwrap();
        assert_eq!(spec.frontmatter.last_verified, None);
        assert_eq!(
            spec.frontmatter.verification_status,
            Some("in_progress".to_string())
        );
        assert_eq!(
            spec.frontmatter.verification_failures,
            Some(vec!["test_a".to_string()])
        );
    }

    #[test]
    fn test_spec_includes_verification_fields_when_set() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-verify-output.md");

        let spec = Spec {
            id: "2026-01-26-008-vwx".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                last_verified: Some("2026-01-26T14:30:00Z".to_string()),
                verification_status: Some("passed".to_string()),
                verification_failures: Some(vec![]),
                ..Default::default()
            },
            title: Some("Verification output test".to_string()),
            body: "# Verification output test\n\nBody.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(saved_content.contains("last_verified: 2026-01-26T14:30:00Z"));
        assert!(saved_content.contains("verification_status: passed"));
        assert!(saved_content.contains("verification_failures:"));
    }

    #[test]
    fn test_parse_spec_with_replay_fields() {
        let content = r#"---
type: code
status: completed
completed_at: 2026-01-20T10:00:00Z
original_completed_at: 2026-01-20T10:00:00Z
replayed_at: 2026-01-26T14:00:00Z
replay_count: 2
---

# Spec with replay tracking

Description here.
"#;
        let spec = Spec::parse("2026-01-26-001-abc", content).unwrap();
        assert_eq!(
            spec.frontmatter.replayed_at,
            Some("2026-01-26T14:00:00Z".to_string())
        );
        assert_eq!(spec.frontmatter.replay_count, Some(2));
        assert_eq!(
            spec.frontmatter.original_completed_at,
            Some("2026-01-20T10:00:00Z".to_string())
        );
    }

    #[test]
    fn test_parse_spec_without_replay_fields() {
        let content = r#"---
type: code
status: completed
completed_at: 2026-01-20T10:00:00Z
---

# Spec without replay tracking

Description here.
"#;
        let spec = Spec::parse("2026-01-26-002-def", content).unwrap();
        assert_eq!(spec.frontmatter.replayed_at, None);
        assert_eq!(spec.frontmatter.replay_count, None);
        assert_eq!(spec.frontmatter.original_completed_at, None);
    }

    #[test]
    fn test_spec_roundtrip_with_replay_fields() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-replay.md");

        let spec = Spec {
            id: "2026-01-26-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-20T10:00:00Z".to_string()),
                original_completed_at: Some("2026-01-20T10:00:00Z".to_string()),
                replayed_at: Some("2026-01-26T14:00:00Z".to_string()),
                replay_count: Some(1),
                ..Default::default()
            },
            title: Some("Replay test".to_string()),
            body: "# Replay test\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();
        let loaded_spec = Spec::load(&spec_path).unwrap();

        assert_eq!(
            loaded_spec.frontmatter.replayed_at,
            Some("2026-01-26T14:00:00Z".to_string())
        );
        assert_eq!(loaded_spec.frontmatter.replay_count, Some(1));
        assert_eq!(
            loaded_spec.frontmatter.original_completed_at,
            Some("2026-01-20T10:00:00Z".to_string())
        );
    }

    #[test]
    fn test_spec_save_without_replay_fields() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-no-replay.md");

        let spec = Spec {
            id: "2026-01-26-004-jkl".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-20T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("No replay".to_string()),
            body: "# No replay\n\nBody content.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(!saved_content.contains("replayed_at:"));
        assert!(!saved_content.contains("replay_count:"));
        assert!(!saved_content.contains("original_completed_at:"));
    }

    #[test]
    fn test_spec_includes_replay_fields_when_set() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("test-replay-output.md");

        let spec = Spec {
            id: "2026-01-26-005-mno".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-26T14:00:00Z".to_string()),
                original_completed_at: Some("2026-01-20T10:00:00Z".to_string()),
                replayed_at: Some("2026-01-26T14:00:00Z".to_string()),
                replay_count: Some(1),
                ..Default::default()
            },
            title: Some("Replay output test".to_string()),
            body: "# Replay output test\n\nBody.".to_string(),
        };

        spec.save(&spec_path).unwrap();

        let saved_content = std::fs::read_to_string(&spec_path).unwrap();
        assert!(saved_content.contains("replayed_at: 2026-01-26T14:00:00Z"));
        assert!(saved_content.contains("replay_count: 1"));
        assert!(saved_content.contains("original_completed_at: 2026-01-20T10:00:00Z"));
    }

    #[test]
    fn test_parse_spec_with_only_replay_count() {
        let content = r#"---
type: code
status: completed
replay_count: 3
---

# Spec with only replay_count

Description here.
"#;
        let spec = Spec::parse("2026-01-26-006-pqr", content).unwrap();
        assert_eq!(spec.frontmatter.replay_count, Some(3));
        assert_eq!(spec.frontmatter.replayed_at, None);
        assert_eq!(spec.frontmatter.original_completed_at, None);
    }

    #[test]
    fn test_parse_spec_with_empty_replay_count() {
        let content = r#"---
type: code
status: completed
replay_count: 0
---

# Spec with zero replay count

Description here.
"#;
        let spec = Spec::parse("2026-01-26-007-stu", content).unwrap();
        assert_eq!(spec.frontmatter.replay_count, Some(0));
    }

    #[test]
    fn test_has_frontmatter_field_required_fields_present() {
        let content = r#"---
type: code
status: pending
team: backend
component: auth
jira_key: AUTH-123
---

# Test spec with required fields
"#;
        let spec = Spec::parse("2026-01-27-001-abc", content).unwrap();

        // Fields that always exist
        assert!(spec.has_frontmatter_field("type"));
        assert!(spec.has_frontmatter_field("status"));
    }

    #[test]
    fn test_has_frontmatter_field_optional_fields_present() {
        let content = r#"---
type: code
status: pending
labels:
  - important
  - bugfix
target_files:
  - src/main.rs
---

# Test spec with optional fields
"#;
        let spec = Spec::parse("2026-01-27-002-def", content).unwrap();

        // Optional fields that are present
        assert!(spec.has_frontmatter_field("labels"));
        assert!(spec.has_frontmatter_field("target_files"));

        // Optional fields that are not present
        assert!(!spec.has_frontmatter_field("model"));
        assert!(!spec.has_frontmatter_field("pr"));
        assert!(!spec.has_frontmatter_field("commits"));
    }

    #[test]
    fn test_has_frontmatter_field_unknown_field() {
        let content = r#"---
type: code
status: pending
---

# Test spec
"#;
        let spec = Spec::parse("2026-01-27-003-ghi", content).unwrap();

        // Unknown fields should return false
        assert!(!spec.has_frontmatter_field("nonexistent_field"));
        assert!(!spec.has_frontmatter_field("custom_field"));
    }

    #[test]
    fn test_has_frontmatter_field_verification_fields() {
        let content = r#"---
type: code
status: pending
last_verified: 2026-01-27T10:00:00Z
verification_status: passed
---

# Test spec with verification fields
"#;
        let spec = Spec::parse("2026-01-27-004-jkl", content).unwrap();

        assert!(spec.has_frontmatter_field("last_verified"));
        assert!(spec.has_frontmatter_field("verification_status"));
        assert!(!spec.has_frontmatter_field("verification_failures"));
    }

    #[test]
    fn test_has_frontmatter_field_depends_on() {
        let content = r#"---
type: code
status: pending
depends_on:
  - 2026-01-27-001-abc
---

# Test spec with dependency
"#;
        let spec = Spec::parse("2026-01-27-005-mno", content).unwrap();

        assert!(spec.has_frontmatter_field("depends_on"));
    }

    #[test]
    fn test_has_frontmatter_field_all_optional_present() {
        let content = r#"---
type: documentation
status: completed
depends_on:
  - 2026-01-27-001-abc
labels:
  - doc
target_files:
  - docs/api.md
context:
  - src/api.rs
prompt: standard
branch: chant/feature
commits:
  - abc123
pr: 42
completed_at: 2026-01-27T15:00:00Z
model: claude-opus-4-5
tracks:
  - src/**/*.rs
informed_by:
  - docs/reference.md
origin:
  - data/metrics.csv
schedule: weekly
source_branch: main
target_branch: feature
conflicting_files:
  - src/config.rs
blocked_specs:
  - 2026-01-27-002-def
original_spec: 2026-01-27-000-xyz
last_verified: 2026-01-27T10:00:00Z
verification_status: passed
verification_failures:
  - test_1
replayed_at: 2026-01-27T14:00:00Z
replay_count: 2
original_completed_at: 2026-01-27T12:00:00Z
---

# Test spec with all fields present
"#;
        let spec = Spec::parse("2026-01-27-006-pqr", content).unwrap();

        // Verify all optional fields are present
        assert!(spec.has_frontmatter_field("depends_on"));
        assert!(spec.has_frontmatter_field("labels"));
        assert!(spec.has_frontmatter_field("target_files"));
        assert!(spec.has_frontmatter_field("context"));
        assert!(spec.has_frontmatter_field("prompt"));
        assert!(spec.has_frontmatter_field("branch"));
        assert!(spec.has_frontmatter_field("commits"));
        assert!(spec.has_frontmatter_field("pr"));
        assert!(spec.has_frontmatter_field("completed_at"));
        assert!(spec.has_frontmatter_field("model"));
        assert!(spec.has_frontmatter_field("tracks"));
        assert!(spec.has_frontmatter_field("informed_by"));
        assert!(spec.has_frontmatter_field("origin"));
        assert!(spec.has_frontmatter_field("schedule"));
        assert!(spec.has_frontmatter_field("source_branch"));
        assert!(spec.has_frontmatter_field("target_branch"));
        assert!(spec.has_frontmatter_field("conflicting_files"));
        assert!(spec.has_frontmatter_field("blocked_specs"));
        assert!(spec.has_frontmatter_field("original_spec"));
        assert!(spec.has_frontmatter_field("last_verified"));
        assert!(spec.has_frontmatter_field("verification_status"));
        assert!(spec.has_frontmatter_field("verification_failures"));
        assert!(spec.has_frontmatter_field("replayed_at"));
        assert!(spec.has_frontmatter_field("replay_count"));
        assert!(spec.has_frontmatter_field("original_completed_at"));
    }

    #[test]
    fn test_spec_ready_after_dependency_completed() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a pending spec that will be the dependency
        let dep_spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Dependency".to_string()),
            body: "# Dependency\n\nBody.".to_string(),
        };

        // Create a pending spec that depends on the above
        let dependent_spec = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-26-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent".to_string()),
            body: "# Dependent\n\nBody.".to_string(),
        };

        // Save both specs
        dep_spec
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        dependent_spec
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();

        // Load specs - dependent should be Blocked because dep is not Completed
        let specs = load_all_specs(specs_dir).unwrap();
        let dependent = specs.iter().find(|s| s.id == "2026-01-26-002-def").unwrap();
        assert_eq!(dependent.frontmatter.status, SpecStatus::Blocked);

        // Now mark the dependency as completed and save it
        let mut completed_dep = dep_spec.clone();
        completed_dep.frontmatter.status = SpecStatus::Completed;
        completed_dep.frontmatter.completed_at = Some("2026-01-26T10:00:00Z".to_string());
        completed_dep
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();

        // Load specs again - dependent should now be Ready (or at least Pending, not Blocked)
        let specs = load_all_specs(specs_dir).unwrap();
        let dependent = specs.iter().find(|s| s.id == "2026-01-26-002-def").unwrap();

        // BUG: This will fail because dependent is still marked as Blocked
        // even though its dependency is now completed
        assert_eq!(
            dependent.frontmatter.status,
            SpecStatus::Pending,
            "Spec should no longer be blocked after its dependency is completed"
        );
    }

    #[test]
    fn test_blocked_spec_unblocked_when_dependency_completed() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a completed spec that will be the dependency
        let dep_spec = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-26T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Dependency".to_string()),
            body: "# Dependency\n\nBody.".to_string(),
        };

        // Create a BLOCKED spec that depends on the above
        // This represents a spec that was previously marked as blocked
        let dependent_spec = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Blocked, // Already marked as blocked in file
                depends_on: Some(vec!["2026-01-26-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent".to_string()),
            body: "# Dependent\n\nBody.".to_string(),
        };

        // Save both specs
        dep_spec
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        dependent_spec
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();

        // Load specs - the dependent should be unblocked because its dependency is completed
        let specs = load_all_specs(specs_dir).unwrap();
        let dependent = specs.iter().find(|s| s.id == "2026-01-26-002-def").unwrap();

        // BUG: This will fail because apply_blocked_status only changes Pending specs,
        // so Blocked specs stay Blocked even if their dependencies are now met
        assert_eq!(
            dependent.frontmatter.status,
            SpecStatus::Pending,
            "Spec should no longer be blocked after its dependency is completed"
        );
    }

    #[test]
    fn test_cascade_dependency_unblocking() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a chain: A depends on B, B depends on C
        let spec_c = Spec {
            id: "2026-01-26-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-26T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("C".to_string()),
            body: "# C\n\nBody.".to_string(),
        };

        let spec_b = Spec {
            id: "2026-01-26-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Blocked,
                depends_on: Some(vec!["2026-01-26-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("B".to_string()),
            body: "# B\n\nBody.".to_string(),
        };

        let spec_a = Spec {
            id: "2026-01-26-003-ghi".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Blocked,
                depends_on: Some(vec!["2026-01-26-002-def".to_string()]),
                ..Default::default()
            },
            title: Some("A".to_string()),
            body: "# A\n\nBody.".to_string(),
        };

        // Save all specs
        spec_c
            .save(&specs_dir.join("2026-01-26-001-abc.md"))
            .unwrap();
        spec_b
            .save(&specs_dir.join("2026-01-26-002-def.md"))
            .unwrap();
        spec_a
            .save(&specs_dir.join("2026-01-26-003-ghi.md"))
            .unwrap();

        // Load specs - B should be unblocked (C is completed), but A should still be blocked
        let specs = load_all_specs(specs_dir).unwrap();
        let b = specs.iter().find(|s| s.id == "2026-01-26-002-def").unwrap();
        let a = specs.iter().find(|s| s.id == "2026-01-26-003-ghi").unwrap();

        assert_eq!(
            b.frontmatter.status,
            SpecStatus::Pending,
            "B should be unblocked since C is completed"
        );
        assert_eq!(
            a.frontmatter.status,
            SpecStatus::Blocked,
            "A should still be blocked since B is only pending"
        );
    }

    #[test]
    fn test_get_blocking_dependencies_with_pending_dependency() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a pending dependency
        let dep_spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Pending dependency".to_string()),
            body: "# Pending dependency\n\nBody.".to_string(),
        };

        // Create a spec that depends on the above
        let dependent_spec = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent spec".to_string()),
            body: "# Dependent spec\n\nBody.".to_string(),
        };

        // Save specs to disk
        dep_spec
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();
        dependent_spec
            .save(&specs_dir.join("2026-01-27-002-def.md"))
            .unwrap();

        let all_specs = vec![dep_spec.clone()];
        let blockers = dependent_spec.get_blocking_dependencies(&all_specs, specs_dir);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].spec_id, "2026-01-27-001-abc");
        assert_eq!(blockers[0].title, Some("Pending dependency".to_string()));
        assert_eq!(blockers[0].status, SpecStatus::Pending);
        assert!(!blockers[0].is_sibling);
    }

    #[test]
    fn test_get_blocking_dependencies_with_completed_dependency() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a completed dependency
        let dep_spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T10:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("Completed dependency".to_string()),
            body: "# Completed dependency\n\nBody.".to_string(),
        };

        // Create a spec that depends on the above (but is still marked as blocked)
        let dependent_spec = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Blocked,
                depends_on: Some(vec!["2026-01-27-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent spec".to_string()),
            body: "# Dependent spec\n\nBody.".to_string(),
        };

        // Save specs to disk
        dep_spec
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();

        let all_specs = vec![dep_spec.clone()];
        let blockers = dependent_spec.get_blocking_dependencies(&all_specs, specs_dir);

        // Should still return the completed dependency to flag potential bug
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].spec_id, "2026-01-27-001-abc");
        assert_eq!(blockers[0].status, SpecStatus::Completed);
        assert_eq!(
            blockers[0].completed_at,
            Some("2026-01-27T10:00:00Z".to_string())
        );
    }

    #[test]
    fn test_get_blocking_dependencies_with_missing_dependency() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a spec that depends on a non-existent spec
        let dependent_spec = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-001-missing".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent spec".to_string()),
            body: "# Dependent spec\n\nBody.".to_string(),
        };

        let all_specs: Vec<Spec> = vec![];
        let blockers = dependent_spec.get_blocking_dependencies(&all_specs, specs_dir);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].spec_id, "2026-01-27-001-missing");
        assert_eq!(blockers[0].title, None);
        assert_eq!(blockers[0].status, SpecStatus::Pending); // Default for unknown
    }

    #[test]
    fn test_get_blocking_dependencies_with_sibling() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a pending sibling .1
        let sibling_spec = Spec {
            id: "2026-01-27-001-abc.1".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("First sibling".to_string()),
            body: "# First sibling\n\nBody.".to_string(),
        };

        // Create sibling .2 that depends on .1
        let member_spec = Spec {
            id: "2026-01-27-001-abc.2".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Second sibling".to_string()),
            body: "# Second sibling\n\nBody.".to_string(),
        };

        // Save sibling spec to disk
        sibling_spec
            .save(&specs_dir.join("2026-01-27-001-abc.1.md"))
            .unwrap();

        let all_specs = vec![sibling_spec.clone()];
        let blockers = member_spec.get_blocking_dependencies(&all_specs, specs_dir);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].spec_id, "2026-01-27-001-abc.1");
        assert_eq!(blockers[0].title, Some("First sibling".to_string()));
        assert_eq!(blockers[0].status, SpecStatus::InProgress);
        assert!(blockers[0].is_sibling);
    }

    #[test]
    fn test_get_blocking_dependencies_reloads_from_disk() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a pending dependency in memory
        let dep_spec_in_memory = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("In-memory status".to_string()),
            body: "# Dependency\n\nBody.".to_string(),
        };

        // But save it to disk as completed with a different title in the body
        let dep_spec_on_disk = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Completed,
                completed_at: Some("2026-01-27T12:00:00Z".to_string()),
                ..Default::default()
            },
            title: Some("On-disk status".to_string()),
            body: "# On-disk status\n\nBody.".to_string(),
        };

        dep_spec_on_disk
            .save(&specs_dir.join("2026-01-27-001-abc.md"))
            .unwrap();

        // Create a spec that depends on the above
        let dependent_spec = Spec {
            id: "2026-01-27-002-def".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                depends_on: Some(vec!["2026-01-27-001-abc".to_string()]),
                ..Default::default()
            },
            title: Some("Dependent spec".to_string()),
            body: "# Dependent spec\n\nBody.".to_string(),
        };

        // Pass the in-memory version (which has Pending status)
        let all_specs = vec![dep_spec_in_memory];
        let blockers = dependent_spec.get_blocking_dependencies(&all_specs, specs_dir);

        // Should prefer the disk version which is Completed
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].status, SpecStatus::Completed);
        assert_eq!(blockers[0].title, Some("On-disk status".to_string()));
    }

    #[test]
    fn test_get_blocking_dependencies_no_blockers() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a spec with no dependencies
        let spec = Spec {
            id: "2026-01-27-001-abc".to_string(),
            frontmatter: SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("No dependencies".to_string()),
            body: "# No dependencies\n\nBody.".to_string(),
        };

        let all_specs: Vec<Spec> = vec![];
        let blockers = spec.get_blocking_dependencies(&all_specs, specs_dir);

        assert!(blockers.is_empty());
    }
}
