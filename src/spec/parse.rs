//! Spec parsing functions.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use super::frontmatter::{SpecFrontmatter, SpecStatus};
use crate::spec::normalize_model_name;

#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub frontmatter: SpecFrontmatter,
    pub title: Option<String>,
    pub body: String,
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

fn branch_exists(branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", branch])
        .output()
        .context("Failed to check if branch exists")?;

    Ok(output.status.success())
}

fn read_spec_from_branch(spec_id: &str, branch: &str) -> Result<Spec> {
    let spec_path = format!(".chant/specs/{}.md", spec_id);

    // Read spec content from branch
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", branch, spec_path)])
        .output()
        .context(format!("Failed to read spec from branch {}", branch))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git show failed: {}", stderr);
    }

    let content =
        String::from_utf8(output.stdout).context("Failed to parse spec content as UTF-8")?;

    Spec::parse(spec_id, &content)
}

impl Spec {
    /// Set the spec status using validated transitions.
    /// This method validates the transition through the state machine.
    ///
    /// INTERNAL USE ONLY: Use transition helpers in state_machine.rs from cmd/ modules.
    pub(crate) fn set_status(
        &mut self,
        new_status: SpecStatus,
    ) -> Result<(), super::state_machine::TransitionError> {
        super::state_machine::TransitionBuilder::new(self).to(new_status)
    }

    /// Parse a spec from file content.
    pub fn parse(id: &str, content: &str) -> Result<Self> {
        let (frontmatter_str, body) = split_frontmatter(content);

        let mut frontmatter: SpecFrontmatter = if let Some(fm) = frontmatter_str {
            serde_yaml::from_str(&fm).context("Failed to parse spec frontmatter")?
        } else {
            SpecFrontmatter::default()
        };

        // Normalize model name if present
        if let Some(model) = &frontmatter.model {
            frontmatter.model = Some(normalize_model_name(model));
        }

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

    /// Load a spec, optionally resolving from its working branch.
    ///
    /// If the spec is in_progress and has a branch (frontmatter.branch or chant/{id}),
    /// attempt to read the spec content from that branch for live progress.
    pub fn load_with_branch_resolution(spec_path: &Path) -> Result<Self> {
        let spec = Self::load(spec_path)?;

        // Only resolve for in_progress specs
        if spec.frontmatter.status != SpecStatus::InProgress {
            return Ok(spec);
        }

        // Try to find the working branch
        let branch_name = spec
            .frontmatter
            .branch
            .clone()
            .unwrap_or_else(|| format!("chant/{}", spec.id));

        // Check if branch exists
        if !branch_exists(&branch_name)? {
            return Ok(spec);
        }

        // Read spec from branch
        match read_spec_from_branch(&spec.id, &branch_name) {
            Ok(branch_spec) => Ok(branch_spec),
            Err(_) => Ok(spec), // Fall back to main version
        }
    }

    /// Save the spec to a file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let frontmatter = serde_yaml::to_string(&self.frontmatter)?;
        let content = format!("---\n{}---\n{}", frontmatter, self.body);
        let tmp_path = path.with_extension("md.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

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

    /// Add derived fields to the spec's frontmatter.
    /// Updates the frontmatter with the provided derived fields.
    pub fn add_derived_fields(&mut self, fields: std::collections::HashMap<String, String>) {
        let mut derived_field_names = Vec::new();

        for (key, value) in fields {
            // Track which fields were derived
            derived_field_names.push(key.clone());

            // Handle specific known derived fields that map to frontmatter
            match key.as_str() {
                "labels" => {
                    let label_vec = value.split(',').map(|s| s.trim().to_string()).collect();
                    self.frontmatter.labels = Some(label_vec);
                }
                "context" => {
                    let context_vec = value.split(',').map(|s| s.trim().to_string()).collect();
                    self.frontmatter.context = Some(context_vec);
                }
                _ => {
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

    /// Auto-check all acceptance criteria checkboxes in the spec body.
    /// Replaces all `- [ ]` with `- [x]` in the Acceptance Criteria section.
    /// Returns true if any checkboxes were checked, false otherwise.
    pub fn auto_check_acceptance_criteria(&mut self) -> bool {
        let acceptance_criteria_marker = "## Acceptance Criteria";
        let mut in_code_fence = false;
        let mut last_ac_line: Option<usize> = None;

        // Find the last AC heading outside code fences
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
            return false;
        };

        // Replace unchecked boxes in the AC section
        let mut in_code_fence = false;
        let mut in_ac_section = false;
        let mut modified = false;
        let mut new_body = String::new();

        for (line_num, line) in self.body.lines().enumerate() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
            }

            if line_num == ac_start {
                in_ac_section = true;
            }

            if in_ac_section && !in_code_fence && trimmed.starts_with("## ") && line_num != ac_start
            {
                in_ac_section = false;
            }

            if in_ac_section && !in_code_fence && line.contains("- [ ]") {
                new_body.push_str(&line.replace("- [ ]", "- [x]"));
                modified = true;
            } else {
                new_body.push_str(line);
            }
            new_body.push('\n');
        }

        if modified {
            self.body = new_body.trim_end().to_string();
        }

        modified
    }

    /// Check if this spec has acceptance criteria.
    /// Returns true if the spec body contains an "## Acceptance Criteria" section
    /// with at least one checkbox item.
    pub fn has_acceptance_criteria(&self) -> bool {
        let acceptance_criteria_marker = "## Acceptance Criteria";
        let mut in_ac_section = false;
        let mut in_code_fence = false;

        for line in self.body.lines() {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") {
                in_code_fence = !in_code_fence;
            }

            if !in_code_fence && trimmed.starts_with(acceptance_criteria_marker) {
                in_ac_section = true;
                continue;
            }

            if in_ac_section && trimmed.starts_with("## ") {
                break;
            }

            if in_ac_section
                && (trimmed.starts_with("- [ ] ")
                    || trimmed.starts_with("- [x] ")
                    || trimmed.starts_with("- [X] "))
            {
                return true;
            }
        }

        false
    }

    /// Check if this spec has unmet dependencies or approval requirements that would block it.
    pub fn is_blocked(&self, all_specs: &[Spec]) -> bool {
        // Check the spec's own dependencies
        if let Some(deps) = &self.frontmatter.depends_on {
            for dep_id in deps {
                let dep = all_specs.iter().find(|s| s.id == *dep_id);
                match dep {
                    Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                    _ => return true,
                }
            }
        }

        // If this is a member spec, also check the driver's dependencies
        if let Some(driver_id) = crate::spec_group::extract_driver_id(&self.id) {
            if let Some(driver_spec) = all_specs.iter().find(|s| s.id == driver_id) {
                if let Some(driver_deps) = &driver_spec.frontmatter.depends_on {
                    for dep_id in driver_deps {
                        let dep = all_specs.iter().find(|s| s.id == *dep_id);
                        match dep {
                            Some(d) if d.frontmatter.status == SpecStatus::Completed => continue,
                            _ => return true,
                        }
                    }
                }
            }
        }

        if self.frontmatter.status == SpecStatus::Pending && self.requires_approval() {
            return true;
        }

        false
    }

    /// Check if this spec is ready to execute.
    pub fn is_ready(&self, all_specs: &[Spec]) -> bool {
        use crate::spec_group::{all_prior_siblings_completed, is_member_of};

        // Allow both Pending and Failed specs to be considered "ready"
        // Failed specs can be retried if their dependencies are met
        if self.frontmatter.status != SpecStatus::Pending
            && self.frontmatter.status != SpecStatus::Failed
        {
            return false;
        }

        if self.is_blocked(all_specs) {
            return false;
        }

        if !all_prior_siblings_completed(&self.id, all_specs) {
            return false;
        }

        let members: Vec<_> = all_specs
            .iter()
            .filter(|s| is_member_of(&s.id, &self.id))
            .collect();

        if !members.is_empty() && self.has_acceptance_criteria() {
            for member in members {
                if member.frontmatter.status != SpecStatus::Completed {
                    return false;
                }
            }
        }

        true
    }

    /// Get the list of dependencies that are blocking this spec.
    pub fn get_blocking_dependencies(
        &self,
        all_specs: &[Spec],
        specs_dir: &Path,
    ) -> Vec<super::frontmatter::BlockingDependency> {
        use super::frontmatter::BlockingDependency;
        use crate::spec_group::{extract_driver_id, extract_member_number};

        let mut blockers = Vec::new();

        if let Some(deps) = &self.frontmatter.depends_on {
            for dep_id in deps {
                let spec_path = specs_dir.join(format!("{}.md", dep_id));
                let dep_spec = if spec_path.exists() {
                    Spec::load(&spec_path).ok()
                } else {
                    None
                };

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
                    }
                } else {
                    blockers.push(BlockingDependency {
                        spec_id: dep_id.clone(),
                        title: None,
                        status: SpecStatus::Pending,
                        completed_at: None,
                        is_sibling: false,
                    });
                }
            }
        }

        if let Some(driver_id) = extract_driver_id(&self.id) {
            if let Some(member_num) = extract_member_number(&self.id) {
                for i in 1..member_num {
                    let sibling_id = format!("{}.{}", driver_id, i);
                    let spec_path = specs_dir.join(format!("{}.md", sibling_id));
                    let sibling_spec = if spec_path.exists() {
                        Spec::load(&spec_path).ok()
                    } else {
                        None
                    };

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
    pub fn has_frontmatter_field(&self, field: &str) -> bool {
        match field {
            "type" => true,
            "status" => true,
            "depends_on" => self.frontmatter.depends_on.is_some(),
            "labels" => self.frontmatter.labels.is_some(),
            "target_files" => self.frontmatter.target_files.is_some(),
            "context" => self.frontmatter.context.is_some(),
            "prompt" => self.frontmatter.prompt.is_some(),
            "branch" => self.frontmatter.branch.is_some(),
            "commits" => self.frontmatter.commits.is_some(),
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
            "members" => self.frontmatter.members.is_some(),
            "output_schema" => self.frontmatter.output_schema.is_some(),
            _ => false,
        }
    }

    /// Check if this spec requires approval before work can begin.
    pub fn requires_approval(&self) -> bool {
        use super::frontmatter::ApprovalStatus;

        if let Some(ref approval) = self.frontmatter.approval {
            approval.required && approval.status != ApprovalStatus::Approved
        } else {
            false
        }
    }

    /// Check if this spec has been approved.
    pub fn is_approved(&self) -> bool {
        use super::frontmatter::ApprovalStatus;

        if let Some(ref approval) = self.frontmatter.approval {
            approval.status == ApprovalStatus::Approved
        } else {
            true
        }
    }

    /// Check if this spec has been rejected.
    pub fn is_rejected(&self) -> bool {
        use super::frontmatter::ApprovalStatus;

        if let Some(ref approval) = self.frontmatter.approval {
            approval.status == ApprovalStatus::Rejected
        } else {
            false
        }
    }
}
