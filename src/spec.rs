//! Spec parsing, frontmatter handling, and spec lifecycle management.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: concepts/specs.md, reference/schema.md
//! - ignore: false

// Submodules
mod frontmatter;
mod lifecycle;
mod parse;
mod state_machine;

// Re-export types from submodules
pub use frontmatter::{
    Approval, ApprovalStatus, BlockingDependency, SpecFrontmatter, SpecStatus, SpecType,
    VerificationStatus,
};
pub use lifecycle::{
    apply_blocked_status_with_repos, is_completed, is_failed, load_all_specs,
    load_all_specs_with_options, resolve_spec,
};
pub use parse::{split_frontmatter, Spec};
pub use state_machine::{
    transition_to_blocked, transition_to_failed, transition_to_in_progress, transition_to_paused,
    TransitionBuilder, TransitionError,
};

// Re-export group/driver functions from spec_group for backward compatibility
pub use crate::spec_group::{
    all_members_completed, all_prior_siblings_completed, auto_complete_driver_if_ready,
    extract_driver_id, extract_member_number, get_incomplete_members, get_members, is_member_of,
    mark_driver_failed_on_member_failure, mark_driver_in_progress,
    mark_driver_in_progress_conditional,
};

/// Normalize model names from full Claude model IDs to short names.
/// Examples: "claude-sonnet-4-20250514" -> "sonnet", "claude-opus-4-5" -> "opus"
///
/// This is used to ensure consistent model names across the system, whether they come
/// from environment variables, config files, or are parsed from spec frontmatter.
pub fn normalize_model_name(model: &str) -> String {
    let lower = model.to_lowercase();
    if lower.contains("opus") {
        "opus".to_string()
    } else if lower.contains("sonnet") {
        "sonnet".to_string()
    } else if lower.contains("haiku") {
        "haiku".to_string()
    } else {
        model.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_verification_fields(
        spec: &Spec,
        last_verified: Option<&str>,
        verification_status: Option<&str>,
        verification_failures: Option<Vec<&str>>,
    ) {
        use std::str::FromStr;
        assert_eq!(
            spec.frontmatter.last_verified,
            last_verified.map(String::from)
        );
        assert_eq!(
            spec.frontmatter.verification_status,
            verification_status.and_then(|s| VerificationStatus::from_str(s).ok())
        );
        assert_eq!(
            spec.frontmatter.verification_failures,
            verification_failures.map(|v| v.iter().map(|s| s.to_string()).collect())
        );
    }

    fn assert_replay_fields(
        spec: &Spec,
        replayed_at: Option<&str>,
        replay_count: Option<u32>,
        original_completed_at: Option<&str>,
    ) {
        assert_eq!(spec.frontmatter.replayed_at, replayed_at.map(String::from));
        assert_eq!(spec.frontmatter.replay_count, replay_count);
        assert_eq!(
            spec.frontmatter.original_completed_at,
            original_completed_at.map(String::from)
        );
    }

    #[test]
    fn test_normalize_model_name_opus() {
        assert_eq!(normalize_model_name("claude-opus-4-5"), "opus");
        assert_eq!(normalize_model_name("claude-opus-4-20250514"), "opus");
        assert_eq!(normalize_model_name("CLAUDE-OPUS-4"), "opus");
        assert_eq!(normalize_model_name("opus"), "opus");
    }

    #[test]
    fn test_normalize_model_name_sonnet() {
        assert_eq!(normalize_model_name("claude-sonnet-4-20250514"), "sonnet");
        assert_eq!(normalize_model_name("claude-sonnet-4-5"), "sonnet");
        assert_eq!(normalize_model_name("CLAUDE-SONNET-3"), "sonnet");
        assert_eq!(normalize_model_name("sonnet"), "sonnet");
    }

    #[test]
    fn test_normalize_model_name_haiku() {
        assert_eq!(normalize_model_name("claude-haiku-4-5"), "haiku");
        assert_eq!(normalize_model_name("claude-haiku-3-20240307"), "haiku");
        assert_eq!(normalize_model_name("CLAUDE-HAIKU-4"), "haiku");
        assert_eq!(normalize_model_name("haiku"), "haiku");
    }

    #[test]
    fn test_normalize_model_name_passthrough() {
        assert_eq!(
            normalize_model_name("gpt-4"),
            "gpt-4",
            "Non-Claude models should pass through unchanged"
        );
        assert_eq!(
            normalize_model_name("llama-3"),
            "llama-3",
            "Non-Claude models should pass through unchanged"
        );
    }

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

        let spec2 = Spec::parse(
            "002",
            r#"---
status: in_progress
---
# Test
"#,
        )
        .unwrap();
        assert!(!spec2.is_ready(&[]));
    }

    #[test]
    fn test_spec_has_acceptance_criteria() {
        let spec_with_ac = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Acceptance Criteria

- [ ] Thing 1
- [ ] Thing 2
"#,
        )
        .unwrap();
        assert!(spec_with_ac.has_acceptance_criteria());

        let spec_without_ac = Spec::parse(
            "002",
            r#"---
status: pending
---
# Test

Description
"#,
        )
        .unwrap();
        assert!(!spec_without_ac.has_acceptance_criteria());
    }

    #[test]
    fn test_count_checkboxes() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
---
# Test

## Acceptance Criteria

- [ ] Thing 1
- [x] Thing 2
- [ ] Thing 3
"#,
        )
        .unwrap();
        assert_eq!(spec.count_unchecked_checkboxes(), 2);
        assert_eq!(spec.count_total_checkboxes(), 3);
    }

    #[test]
    fn test_split_frontmatter() {
        let content = r#"---
type: code
status: pending
---

# Title

Body"#;
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_some());
        assert!(body.contains("# Title"));
    }

    #[test]
    fn test_split_frontmatter_no_frontmatter() {
        let content = "# Title\n\nBody";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_approval_required() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
approval:
  required: true
  status: pending
---
# Test
"#,
        )
        .unwrap();
        assert!(spec.requires_approval());
        assert!(!spec.is_approved());
        assert!(!spec.is_rejected());
    }

    #[test]
    fn test_approval_granted() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
approval:
  required: true
  status: approved
  by: "user@example.com"
  at: "2026-01-25T12:00:00Z"
---
# Test
"#,
        )
        .unwrap();
        assert!(!spec.requires_approval());
        assert!(spec.is_approved());
        assert!(!spec.is_rejected());
    }

    #[test]
    fn test_approval_rejected() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
approval:
  required: true
  status: rejected
  by: "user@example.com"
  at: "2026-01-25T12:00:00Z"
---
# Test
"#,
        )
        .unwrap();
        assert!(spec.requires_approval());
        assert!(!spec.is_approved());
        assert!(spec.is_rejected());
    }

    #[test]
    fn test_verification_fields() {
        let spec = Spec::parse(
            "001",
            r#"---
status: completed
last_verified: "2026-01-25T12:00:00Z"
verification_status: "passed"
---
# Test
"#,
        )
        .unwrap();
        assert_verification_fields(&spec, Some("2026-01-25T12:00:00Z"), Some("passed"), None);
    }

    #[test]
    fn test_replay_fields() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
replayed_at: "2026-01-25T14:00:00Z"
replay_count: 2
original_completed_at: "2026-01-24T12:00:00Z"
---
# Test
"#,
        )
        .unwrap();
        assert_replay_fields(
            &spec,
            Some("2026-01-25T14:00:00Z"),
            Some(2),
            Some("2026-01-24T12:00:00Z"),
        );
    }

    #[test]
    fn test_has_frontmatter_field() {
        let spec = Spec::parse(
            "001",
            r#"---
status: pending
model: "sonnet"
labels: ["bug", "urgent"]
---
# Test
"#,
        )
        .unwrap();
        assert!(spec.has_frontmatter_field("status"));
        assert!(spec.has_frontmatter_field("model"));
        assert!(spec.has_frontmatter_field("labels"));
        assert!(!spec.has_frontmatter_field("context"));
    }
}
