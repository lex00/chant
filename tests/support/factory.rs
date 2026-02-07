use chant::spec::{Spec, SpecFrontmatter, SpecStatus};

/// SpecFactory provides convenient methods to create specs with various states.
pub struct SpecFactory;

impl SpecFactory {
    /// Creates a pending spec with the given ID.
    pub fn pending(id: &str) -> Spec {
        Self::with_status(id, SpecStatus::Pending)
    }

    /// Creates an in_progress spec with the given ID.
    pub fn in_progress(id: &str) -> Spec {
        Self::with_status(id, SpecStatus::InProgress)
    }

    /// Creates a completed spec with the given ID.
    pub fn completed(id: &str) -> Spec {
        Self::with_status(id, SpecStatus::Completed)
    }

    /// Creates a failed spec with the given ID.
    pub fn failed(id: &str) -> Spec {
        Self::with_status(id, SpecStatus::Failed)
    }

    /// Creates a spec blocked by the given dependencies.
    pub fn blocked_by(id: &str, deps: &[&str]) -> Spec {
        let mut spec = Self::with_status(id, SpecStatus::Pending);
        spec.frontmatter.depends_on = Some(deps.iter().map(|s| s.to_string()).collect());
        spec
    }

    /// Creates a spec with commit references.
    pub fn with_commits(id: &str, commits: &[&str]) -> Spec {
        let mut spec = Self::with_status(id, SpecStatus::InProgress);
        spec.frontmatter.commits = Some(commits.iter().map(|s| s.to_string()).collect());
        spec
    }

    /// Creates a driver spec with the given number of member specs.
    /// Returns a vector where the first element is the driver, followed by member specs.
    pub fn driver_with_members(id: &str, n: usize) -> Vec<Spec> {
        let member_ids: Vec<String> = (0..n).map(|i| format!("{}-m{}", id, i)).collect();

        let mut driver = Self::with_status(id, SpecStatus::Pending);
        driver.frontmatter.members = Some(member_ids.clone());

        let mut result = vec![driver];

        for member_id in member_ids {
            result.push(Self::with_status(&member_id, SpecStatus::Pending));
        }

        result
    }

    /// Creates a spec with the given status.
    fn with_status(id: &str, status: SpecStatus) -> Spec {
        Spec {
            id: id.to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "code".to_string(),
                status,
                depends_on: None,
                labels: None,
                target_files: None,
                context: None,
                prompt: None,
                branch: None,
                commits: None,
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
                approval: None,
                members: None,
                output_schema: None,
                derived_fields: None,
                public: None,
                retry_state: None,
            },
            title: Some(format!("Test Spec: {}", id)),
            body: format!(
                "Test specification {}\n\n## Acceptance Criteria\n\n- [ ] Test criterion\n",
                id
            ),
        }
    }

    /// Creates a spec with full content including frontmatter as markdown.
    pub fn as_markdown(id: &str, status: &str) -> String {
        format!(
            r#"---
type: code
status: {}
---

# Test Spec: {}

Test specification for {}.

## Acceptance Criteria

- [ ] Test criterion
"#,
            status, id, id
        )
    }

    /// Creates a spec with approval metadata as markdown.
    pub fn as_markdown_with_approval(id: &str, status: &str, approval_status: &str) -> String {
        format!(
            r#"---
type: code
status: {}
approval:
  required: true
  status: {}
  by: Initial User
  at: 2026-01-01T00:00:00Z
---

# Test Spec: {}

Test specification with approval.

## Acceptance Criteria

- [ ] Test criterion
"#,
            status, approval_status, id
        )
    }

    /// Creates a spec with dependencies as markdown.
    pub fn as_markdown_with_deps(id: &str, status: &str, dependencies: &[&str]) -> String {
        let deps_yaml = if dependencies.is_empty() {
            String::new()
        } else {
            format!(
                "depends_on:\n{}",
                dependencies
                    .iter()
                    .map(|d| format!("  - {}", d))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        format!(
            r#"---
type: code
status: {}
{}---

# Test Spec: {}

Test specification for dependency testing.

## Acceptance Criteria

- [ ] Test criterion
"#,
            status,
            if deps_yaml.is_empty() {
                String::new()
            } else {
                format!("{}\n", deps_yaml)
            },
            id
        )
    }
}
