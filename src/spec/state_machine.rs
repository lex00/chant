//! State machine for spec lifecycle transitions.
//!
//! Provides centralized validation of status transitions with precondition checks.

use anyhow::Result;
use std::fmt;
use std::path::Path;
use std::process::Command;

use super::frontmatter::SpecStatus;
use super::parse::Spec;

#[derive(Debug)]
pub enum TransitionError {
    InvalidTransition { from: SpecStatus, to: SpecStatus },
    DirtyWorktree(String),
    UnmetDependencies(String),
    IncompleteCriteria,
    NoCommits,
    IncompleteMembers(String),
    ApprovalRequired,
    LintFailed,
    Other(String),
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransitionError::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {:?} to {:?}", from, to)
            }
            TransitionError::DirtyWorktree(msg) => write!(f, "Worktree is not clean: {}", msg),
            TransitionError::UnmetDependencies(msg) => write!(f, "Dependencies not met: {}", msg),
            TransitionError::IncompleteCriteria => {
                write!(f, "All acceptance criteria must be checked")
            }
            TransitionError::NoCommits => write!(f, "No commits found for spec"),
            TransitionError::IncompleteMembers(members) => {
                write!(f, "Incomplete driver members: {}", members)
            }
            TransitionError::ApprovalRequired => write!(f, "Spec requires approval"),
            TransitionError::LintFailed => write!(f, "Lint validation failed"),
            TransitionError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for TransitionError {}

/// Builder for validated state transitions.
pub struct TransitionBuilder<'a> {
    spec: &'a mut Spec,
    require_clean: bool,
    require_deps: bool,
    require_criteria: bool,
    require_commits: bool,
    require_no_incomplete_members: bool,
    check_approval: bool,
    force: bool,
    project_name: Option<String>,
    specs_dir: Option<std::path::PathBuf>,
}

impl<'a> TransitionBuilder<'a> {
    /// Create a new transition builder for a spec.
    pub fn new(spec: &'a mut Spec) -> Self {
        Self {
            spec,
            require_clean: false,
            require_deps: false,
            require_criteria: false,
            require_commits: false,
            require_no_incomplete_members: false,
            check_approval: false,
            force: false,
            project_name: None,
            specs_dir: None,
        }
    }

    /// Require worktree to be clean (no uncommitted changes).
    pub fn require_clean_tree(mut self) -> Self {
        self.require_clean = true;
        self
    }

    /// Require all dependencies to be met.
    pub fn require_dependencies_met(mut self) -> Self {
        self.require_deps = true;
        self
    }

    /// Require all acceptance criteria to be checked.
    pub fn require_all_criteria_checked(mut self) -> Self {
        self.require_criteria = true;
        self
    }

    /// Require at least one commit for the spec.
    pub fn require_commits_exist(mut self) -> Self {
        self.require_commits = true;
        self
    }

    /// Require no incomplete driver members.
    pub fn require_no_incomplete_members(mut self) -> Self {
        self.require_no_incomplete_members = true;
        self
    }

    /// Check approval status.
    pub fn check_approval(mut self) -> Self {
        self.check_approval = true;
        self
    }

    /// Force the transition, bypassing all precondition checks.
    /// Use with extreme caution - intended for exceptional cases only.
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Set the project name for worktree path resolution.
    pub fn with_project_name(mut self, project_name: Option<&str>) -> Self {
        self.project_name = project_name.map(|s| s.to_string());
        self
    }

    /// Set the specs directory path (overrides default .chant/specs).
    pub fn with_specs_dir(mut self, specs_dir: &Path) -> Self {
        self.specs_dir = Some(specs_dir.to_path_buf());
        self
    }

    /// Execute the transition to the target status.
    pub fn to(self, target: SpecStatus) -> Result<(), TransitionError> {
        let current = &self.spec.frontmatter.status;

        // Check if transition is valid
        if !self.force && !is_valid_transition(current, &target) {
            return Err(TransitionError::InvalidTransition {
                from: current.clone(),
                to: target,
            });
        }

        // Run precondition checks (unless forced)
        if !self.force {
            self.check_preconditions(&target)?;
        }

        // Apply the transition
        self.spec.frontmatter.status = target;
        Ok(())
    }

    fn check_preconditions(&self, _target: &SpecStatus) -> Result<(), TransitionError> {
        if self.check_approval && self.spec.requires_approval() {
            return Err(TransitionError::ApprovalRequired);
        }

        if self.require_deps {
            let specs_dir = self
                .specs_dir
                .as_deref()
                .unwrap_or_else(|| Path::new(".chant/specs"));
            if specs_dir.exists() {
                let all_specs = super::lifecycle::load_all_specs(specs_dir)
                    .map_err(|e| TransitionError::Other(format!("Failed to load specs: {}", e)))?;

                if self.spec.is_blocked(&all_specs) {
                    return Err(TransitionError::UnmetDependencies(
                        "Spec has unmet dependencies".to_string(),
                    ));
                }
            }
        }

        if self.require_criteria && self.spec.count_unchecked_checkboxes() > 0 {
            return Err(TransitionError::IncompleteCriteria);
        }

        if self.require_commits && !has_commits(&self.spec.id)? {
            return Err(TransitionError::NoCommits);
        }

        if self.require_no_incomplete_members {
            if let Some(members) = &self.spec.frontmatter.members {
                let specs_dir = self
                    .specs_dir
                    .as_deref()
                    .unwrap_or_else(|| Path::new(".chant/specs"));
                if specs_dir.exists() {
                    let all_specs = super::lifecycle::load_all_specs(specs_dir).map_err(|e| {
                        TransitionError::Other(format!("Failed to load specs: {}", e))
                    })?;

                    let incomplete: Vec<_> = members
                        .iter()
                        .filter_map(|m| {
                            all_specs
                                .iter()
                                .find(|s| s.id == *m)
                                .filter(|s| s.frontmatter.status != SpecStatus::Completed)
                                .map(|s| s.id.clone())
                        })
                        .collect();

                    if !incomplete.is_empty() {
                        return Err(TransitionError::IncompleteMembers(incomplete.join(", ")));
                    }
                }
            }
        }

        if self.require_clean && !is_clean(&self.spec.id, self.project_name.as_deref())? {
            return Err(TransitionError::DirtyWorktree(
                "Worktree has uncommitted changes".to_string(),
            ));
        }

        Ok(())
    }
}

/// Check if a transition from one status to another is valid.
fn is_valid_transition(from: &SpecStatus, to: &SpecStatus) -> bool {
    use SpecStatus::*;

    match (from, to) {
        // Self-transitions are always valid
        (a, b) if a == b => true,

        // From Pending
        (Pending, InProgress) => true,
        (Pending, Blocked) => true,
        (Pending, Cancelled) => true,

        // From Blocked
        (Blocked, Pending) => true,
        (Blocked, InProgress) => true,
        (Blocked, Cancelled) => true,

        // From InProgress
        (InProgress, Completed) => true,
        (InProgress, Failed) => true,
        (InProgress, NeedsAttention) => true,
        (InProgress, Paused) => true,
        (InProgress, Cancelled) => true,

        // From Failed
        (Failed, Pending) => true,
        (Failed, InProgress) => true,

        // From NeedsAttention
        (NeedsAttention, Pending) => true,
        (NeedsAttention, InProgress) => true,

        // From Paused
        (Paused, InProgress) => true,
        (Paused, Cancelled) => true,

        // From Completed - generally immutable except for special cases
        (Completed, Pending) => true, // For replay/verification scenarios

        // From Cancelled
        (Cancelled, Pending) => true,

        // Ready is a computed state, not a persistent status
        (Ready, _) | (_, Ready) => false,

        // All other transitions are invalid
        _ => false,
    }
}

// ============================================================================
// PUBLIC TRANSITION HELPERS
// ============================================================================

/// Transition a spec to InProgress with dependency validation.
pub fn transition_to_in_progress(
    spec: &mut Spec,
    specs_dir: Option<&Path>,
) -> Result<(), TransitionError> {
    TransitionBuilder::new(spec)
        .require_dependencies_met()
        .with_specs_dir(specs_dir.unwrap_or_else(|| Path::new(".chant/specs")))
        .to(SpecStatus::InProgress)
}

/// Transition a spec to Failed with cleanup (force transition).
pub fn transition_to_failed(spec: &mut Spec) -> Result<(), TransitionError> {
    TransitionBuilder::new(spec).force().to(SpecStatus::Failed)
}

/// Transition a spec to Paused (used in takeover scenarios).
pub fn transition_to_paused(spec: &mut Spec) -> Result<(), TransitionError> {
    TransitionBuilder::new(spec).to(SpecStatus::Paused)
}

/// Transition a spec to Blocked (used when creating dependency fix specs).
pub fn transition_to_blocked(spec: &mut Spec) -> Result<(), TransitionError> {
    TransitionBuilder::new(spec).to(SpecStatus::Blocked)
}

// ============================================================================
// INTERNAL HELPERS
// ============================================================================

/// Check if the spec has commits matching the chant(spec_id): pattern.
fn has_commits(spec_id: &str) -> Result<bool, TransitionError> {
    let pattern = format!("chant({}):", spec_id);
    let output = Command::new("git")
        .args(["log", "--all", "--grep", &pattern, "--format=%H"])
        .output()
        .map_err(|e| TransitionError::Other(format!("Failed to check git log: {}", e)))?;

    if !output.status.success() {
        return Ok(false);
    }

    let commits_output = String::from_utf8_lossy(&output.stdout);
    Ok(!commits_output.trim().is_empty())
}

/// Check if the worktree is clean (no uncommitted changes).
fn is_clean(spec_id: &str, project_name: Option<&str>) -> Result<bool, TransitionError> {
    use crate::worktree;

    // Check if there's an active worktree for this spec
    let check_path =
        if let Some(worktree_path) = worktree::get_active_worktree(spec_id, project_name) {
            worktree_path
        } else {
            // No worktree, check current directory
            std::path::PathBuf::from(".")
        };

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&check_path)
        .output()
        .map_err(|e| {
            TransitionError::Other(format!(
                "Failed to check git status in {:?}: {}",
                check_path, e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TransitionError::Other(format!(
            "git status failed: {}",
            stderr
        )));
    }

    let status_output = String::from_utf8_lossy(&output.stdout);
    Ok(status_output.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions_from_pending() {
        use SpecStatus::*;

        assert!(is_valid_transition(&Pending, &InProgress));
        assert!(is_valid_transition(&Pending, &Blocked));
        assert!(is_valid_transition(&Pending, &Cancelled));

        assert!(!is_valid_transition(&Pending, &Completed));
        assert!(!is_valid_transition(&Pending, &Failed));
    }

    #[test]
    fn test_valid_transitions_from_in_progress() {
        use SpecStatus::*;

        assert!(is_valid_transition(&InProgress, &Completed));
        assert!(is_valid_transition(&InProgress, &Failed));
        assert!(is_valid_transition(&InProgress, &NeedsAttention));
        assert!(is_valid_transition(&InProgress, &Paused));
        assert!(is_valid_transition(&InProgress, &Cancelled));

        assert!(!is_valid_transition(&InProgress, &Pending));
        assert!(!is_valid_transition(&InProgress, &Blocked));
    }

    #[test]
    fn test_valid_transitions_from_blocked() {
        use SpecStatus::*;

        assert!(is_valid_transition(&Blocked, &Pending));
        assert!(is_valid_transition(&Blocked, &InProgress));
        assert!(is_valid_transition(&Blocked, &Cancelled));

        assert!(!is_valid_transition(&Blocked, &Completed));
        assert!(!is_valid_transition(&Blocked, &Failed));
    }

    #[test]
    fn test_valid_transitions_from_failed() {
        use SpecStatus::*;

        assert!(is_valid_transition(&Failed, &Pending));
        assert!(is_valid_transition(&Failed, &InProgress));

        assert!(!is_valid_transition(&Failed, &Completed));
    }

    #[test]
    fn test_valid_transitions_from_paused() {
        use SpecStatus::*;

        assert!(is_valid_transition(&Paused, &InProgress));
        assert!(is_valid_transition(&Paused, &Cancelled));

        assert!(!is_valid_transition(&Paused, &Pending));
        assert!(!is_valid_transition(&Paused, &Completed));
    }

    #[test]
    fn test_valid_transitions_from_completed() {
        use SpecStatus::*;

        // Completed can transition back to Pending for replay
        assert!(is_valid_transition(&Completed, &Pending));

        // Generally, completed specs don't transition to other states
        assert!(!is_valid_transition(&Completed, &InProgress));
        assert!(!is_valid_transition(&Completed, &Failed));
    }

    #[test]
    fn test_invalid_ready_transitions() {
        use SpecStatus::*;

        // Ready is a computed state, not a persistent status
        assert!(!is_valid_transition(&Ready, &InProgress));
        assert!(!is_valid_transition(&Pending, &Ready));
    }

    #[test]
    fn test_self_transitions() {
        use SpecStatus::*;

        assert!(is_valid_transition(&Pending, &Pending));
        assert!(is_valid_transition(&InProgress, &InProgress));
        assert!(is_valid_transition(&Completed, &Completed));
    }

    #[test]
    fn test_builder_basic_transition() {
        let mut spec = Spec::parse(
            "test-001",
            r#"---
type: code
status: pending
---
# Test
"#,
        )
        .unwrap();

        let result = TransitionBuilder::new(&mut spec).to(SpecStatus::InProgress);
        assert!(result.is_ok());
        assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
    }

    #[test]
    fn test_builder_invalid_transition() {
        let mut spec = Spec::parse(
            "test-002",
            r#"---
type: code
status: pending
---
# Test
"#,
        )
        .unwrap();

        let result = TransitionBuilder::new(&mut spec).to(SpecStatus::Completed);
        assert!(result.is_err());
        match result {
            Err(TransitionError::InvalidTransition { from, to }) => {
                assert_eq!(from, SpecStatus::Pending);
                assert_eq!(to, SpecStatus::Completed);
            }
            _ => panic!("Expected InvalidTransition error"),
        }
    }

    #[test]
    fn test_builder_force_bypass() {
        let mut spec = Spec::parse(
            "test-003",
            r#"---
type: code
status: pending
---
# Test
"#,
        )
        .unwrap();

        // Force bypass allows invalid transition
        let result = TransitionBuilder::new(&mut spec)
            .force()
            .to(SpecStatus::Completed);
        assert!(result.is_ok());
        assert_eq!(spec.frontmatter.status, SpecStatus::Completed);
    }

    #[test]
    fn test_builder_criteria_check() {
        let mut spec = Spec::parse(
            "test-004",
            r#"---
type: code
status: in_progress
---
# Test

## Acceptance Criteria

- [ ] Task 1
- [ ] Task 2
"#,
        )
        .unwrap();

        let result = TransitionBuilder::new(&mut spec)
            .require_all_criteria_checked()
            .to(SpecStatus::Completed);

        assert!(result.is_err());
        match result {
            Err(TransitionError::IncompleteCriteria) => {}
            _ => panic!("Expected IncompleteCriteria error"),
        }
    }

    #[test]
    fn test_builder_criteria_check_passes() {
        let mut spec = Spec::parse(
            "test-005",
            r#"---
type: code
status: in_progress
---
# Test

## Acceptance Criteria

- [x] Task 1
- [x] Task 2
"#,
        )
        .unwrap();

        // Should fail with NoCommits instead of IncompleteCriteria
        let result = TransitionBuilder::new(&mut spec)
            .require_all_criteria_checked()
            .require_commits_exist()
            .to(SpecStatus::Completed);

        match result {
            Err(TransitionError::NoCommits) => {}
            _ => panic!("Expected NoCommits error, criteria check passed"),
        }
    }

    #[test]
    fn test_builder_approval_required() {
        let mut spec = Spec::parse(
            "test-006",
            r#"---
type: code
status: pending
approval:
  required: true
  status: pending
---
# Test
"#,
        )
        .unwrap();

        let result = TransitionBuilder::new(&mut spec)
            .check_approval()
            .to(SpecStatus::InProgress);

        assert!(result.is_err());
        match result {
            Err(TransitionError::ApprovalRequired) => {}
            _ => panic!("Expected ApprovalRequired error"),
        }
    }

    #[test]
    fn test_builder_with_project_name() {
        let mut spec = Spec::parse(
            "test-007",
            r#"---
type: code
status: pending
---
# Test
"#,
        )
        .unwrap();

        // Test that builder accepts project name
        let result = TransitionBuilder::new(&mut spec)
            .with_project_name(Some("myproject"))
            .to(SpecStatus::InProgress);

        assert!(result.is_ok());
        assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
    }

    #[test]
    fn test_builder_with_specs_dir() {
        let mut spec = Spec::parse(
            "test-008",
            r#"---
type: code
status: pending
---
# Test
"#,
        )
        .unwrap();

        // Test that builder accepts specs_dir
        let specs_dir = Path::new("/custom/specs");
        let result = TransitionBuilder::new(&mut spec)
            .with_specs_dir(specs_dir)
            .to(SpecStatus::InProgress);

        assert!(result.is_ok());
        assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
    }
}
