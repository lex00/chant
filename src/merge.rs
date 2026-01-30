//! Spec merge logic and utilities.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: guides/recovery.md
//! - ignore: false

use crate::config::Config;
use crate::git::MergeResult;
use crate::spec::{Spec, SpecStatus};
use crate::spec_group::{extract_member_number, is_member_of};
use anyhow::Result;

/// Load main_branch from config with fallback to "main"
pub fn load_main_branch(config: &Config) -> String {
    config.defaults.main_branch.clone()
}

/// Get the list of specs to merge based on arguments
/// Returns vector of (spec_id, Spec) tuples
pub fn get_specs_to_merge(
    args: &[String],
    all: bool,
    all_specs: &[Spec],
) -> Result<Vec<(String, Spec)>> {
    let mut result = Vec::new();

    if all {
        // Collect all specs with status == Completed that have branches
        for spec in all_specs {
            if spec.frontmatter.status == SpecStatus::Completed {
                result.push((spec.id.clone(), spec.clone()));
            }
        }
    } else {
        // Resolve each ID using the same matching logic as resolve_spec:
        // 1. Exact match
        // 2. Suffix match (ends_with)
        // 3. Contains match (partial_id anywhere in spec.id)
        for partial_id in args {
            // Try exact match first
            if let Some(spec) = all_specs.iter().find(|s| s.id == *partial_id) {
                result.push((spec.id.clone(), spec.clone()));
                continue;
            }

            // Try suffix match
            let suffix_matches: Vec<_> = all_specs
                .iter()
                .filter(|s| s.id.ends_with(partial_id))
                .collect();
            if suffix_matches.len() == 1 {
                result.push((suffix_matches[0].id.clone(), suffix_matches[0].clone()));
                continue;
            }

            // Try contains match
            let contains_matches: Vec<_> = all_specs
                .iter()
                .filter(|s| s.id.contains(partial_id))
                .collect();
            if contains_matches.len() == 1 {
                result.push((contains_matches[0].id.clone(), contains_matches[0].clone()));
                continue;
            }

            if contains_matches.len() > 1 {
                anyhow::bail!(
                    "Ambiguous spec ID '{}'. Matches: {}",
                    partial_id,
                    contains_matches
                        .iter()
                        .map(|s| s.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            anyhow::bail!("Spec not found: {}", partial_id);
        }
    }

    Ok(result)
}

/// Validate that a spec can be merged. Public API used in tests.
#[allow(dead_code)] // Public API method used in tests
pub fn validate_spec_can_merge(spec: &Spec, branch_exists: bool) -> Result<()> {
    // Check status is Completed
    match &spec.frontmatter.status {
        SpecStatus::Completed => {}
        other => {
            let status_str = format!("{:?}", other);
            anyhow::bail!(
                "{}",
                crate::merge_errors::spec_status_not_mergeable(&spec.id, &status_str)
            );
        }
    }

    // Check branch exists
    if !branch_exists {
        anyhow::bail!("{}", crate::merge_errors::no_branch_for_spec(&spec.id));
    }

    Ok(())
}

/// Check if a spec is a driver spec (has member specs)
pub fn is_driver_spec(spec: &Spec, all_specs: &[Spec]) -> bool {
    let members = collect_member_specs(spec, all_specs);
    !members.is_empty()
}

/// Collect member specs of a driver spec in order (by sequence number)
fn collect_member_specs(driver_spec: &Spec, all_specs: &[Spec]) -> Vec<Spec> {
    let driver_id = &driver_spec.id;
    let mut members: Vec<(u32, Spec)> = Vec::new();

    for spec in all_specs {
        if is_member_of(&spec.id, driver_id) {
            // Extract sequence number from member ID
            if let Some(seq_num) = extract_member_number(&spec.id) {
                members.push((seq_num, spec.clone()));
            }
        }
    }

    // Sort by sequence number
    members.sort_by_key(|m| m.0);

    // Return just the specs
    members.into_iter().map(|(_, spec)| spec).collect()
}

/// Merge a driver spec and all its members in order.
///
/// This function:
/// 1. Collects all member specs in order
/// 2. Validates all members are completed and branches exist
/// 3. Merges each member spec in sequence
/// 4. If any member merge fails, stops and reports which member failed
/// 5. After all members succeed, merges the driver spec itself
/// 6. Returns a list of all merge results (members + driver)
///
/// If any validation fails, returns an error with a clear listing of incomplete members.
pub fn merge_driver_spec(
    driver_spec: &Spec,
    all_specs: &[Spec],
    branch_prefix: &str,
    main_branch: &str,
    should_delete_branch: bool,
    dry_run: bool,
) -> Result<Vec<MergeResult>> {
    use crate::git;

    // Collect member specs in order
    let members = collect_member_specs(driver_spec, all_specs);

    // Check preconditions for all members
    let mut incomplete_members = Vec::new();
    for member in &members {
        // Check status is Completed
        if member.frontmatter.status != SpecStatus::Completed {
            incomplete_members.push(format!(
                "{} (status: {:?})",
                member.id, member.frontmatter.status
            ));
        }
    }

    // Check all member branches exist (unless dry_run)
    if !dry_run {
        for member in &members {
            let branch_name = format!("{}{}", branch_prefix, member.id);
            match git::branch_exists(&branch_name) {
                Ok(exists) => {
                    if !exists {
                        incomplete_members.push(format!("{} (branch not found)", member.id));
                    }
                }
                Err(e) => {
                    anyhow::bail!("Failed to check branch for {}: {}", member.id, e);
                }
            }
        }
    }

    // If any preconditions failed, report them all
    if !incomplete_members.is_empty() {
        anyhow::bail!(
            "{}",
            crate::merge_errors::driver_members_incomplete(&driver_spec.id, &incomplete_members)
        );
    }

    // Merge each member spec in order
    let mut all_results = Vec::new();
    for member in &members {
        let branch_name = format!("{}{}", branch_prefix, member.id);
        match git::merge_single_spec(
            &member.id,
            &branch_name,
            main_branch,
            should_delete_branch,
            dry_run,
        ) {
            Ok(result) => {
                if !result.success {
                    anyhow::bail!(
                        "{}",
                        crate::merge_errors::member_merge_failed(
                            &driver_spec.id,
                            &member.id,
                            &format!("Merge returned unsuccessful for {}", result.spec_id)
                        )
                    );
                }
                all_results.push(result);
            }
            Err(e) => {
                anyhow::bail!(
                    "{}",
                    crate::merge_errors::member_merge_failed(
                        &driver_spec.id,
                        &member.id,
                        &e.to_string()
                    )
                );
            }
        }
    }

    // After all members succeed, merge the driver spec itself
    let driver_branch = format!("{}{}", branch_prefix, driver_spec.id);
    match git::merge_single_spec(
        &driver_spec.id,
        &driver_branch,
        main_branch,
        should_delete_branch,
        dry_run,
    ) {
        Ok(result) => {
            all_results.push(result);
            Ok(all_results)
        }
        Err(e) => {
            anyhow::bail!(
                "{}",
                crate::merge_errors::member_merge_failed(
                    &driver_spec.id,
                    &driver_spec.id,
                    &e.to_string()
                )
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DefaultsConfig;

    fn make_config(defaults: DefaultsConfig) -> Config {
        Config {
            project: crate::config::ProjectConfig {
                name: "test".to_string(),
                prefix: None,
            },
            defaults,
            providers: crate::provider::ProviderConfig::default(),
            parallel: crate::config::ParallelConfig::default(),
            repos: vec![],
            enterprise: crate::config::EnterpriseConfig::default(),
            approval: crate::config::ApprovalConfig::default(),
            validation: crate::config::OutputValidationConfig::default(),
            site: crate::config::SiteConfig::default(),
        }
    }

    fn make_spec(id: &str, status: SpecStatus) -> Spec {
        Spec {
            id: id.to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status,
                ..Default::default()
            },
            title: Some(format!("Spec {}", id)),
            body: format!("Body {}", id),
        }
    }

    #[test]
    fn test_load_main_branch_default() {
        let config = make_config(DefaultsConfig::default());
        let branch = load_main_branch(&config);
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_load_main_branch_custom() {
        let config = make_config(DefaultsConfig {
            main_branch: "master".to_string(),
            ..Default::default()
        });
        let branch = load_main_branch(&config);
        assert_eq!(branch, "master");
    }

    #[test]
    fn test_get_specs_to_merge_all() {
        let specs = vec![
            make_spec("spec1", SpecStatus::Completed),
            make_spec("spec2", SpecStatus::Pending),
            make_spec("spec3", SpecStatus::Completed),
        ];

        let result = get_specs_to_merge(&[], true, &specs).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "spec1");
        assert_eq!(result[1].0, "spec3");
    }

    #[test]
    fn test_get_specs_to_merge_specific() {
        let specs = vec![
            make_spec("spec1", SpecStatus::Completed),
            make_spec("spec2", SpecStatus::Completed),
        ];

        let result = get_specs_to_merge(&["spec1".to_string()], false, &specs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "spec1");
    }

    #[test]
    fn test_get_specs_to_merge_not_found() {
        let specs = vec![make_spec("spec1", SpecStatus::Pending)];

        let result = get_specs_to_merge(&["nonexistent".to_string()], false, &specs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Spec not found"));
    }

    #[test]
    fn test_validate_spec_can_merge_completed() {
        let spec = make_spec("spec1", SpecStatus::Completed);
        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_spec_can_merge_pending_fails() {
        let spec = make_spec("spec1", SpecStatus::Pending);
        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot merge spec spec1"));
        assert!(err_msg.contains("Pending"));
        assert!(err_msg.contains("Next Steps"));
    }

    #[test]
    fn test_validate_spec_can_merge_in_progress_fails() {
        let spec = make_spec("spec1", SpecStatus::InProgress);
        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot merge spec spec1"));
        assert!(err_msg.contains("InProgress"));
        assert!(err_msg.contains("Next Steps"));
    }

    #[test]
    fn test_validate_spec_can_merge_failed_fails() {
        let spec = make_spec("spec1", SpecStatus::Failed);
        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot merge spec spec1"));
        assert!(err_msg.contains("Failed"));
        assert!(err_msg.contains("Next Steps"));
    }

    #[test]
    fn test_validate_spec_can_merge_no_branch() {
        let spec = make_spec("spec1", SpecStatus::Completed);
        let result = validate_spec_can_merge(&spec, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No branch found"));
    }

    #[test]
    fn test_collect_member_specs() {
        let driver = Spec {
            id: "driver".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let member1 = Spec {
            id: "driver.1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Member 1".to_string()),
            body: "Member 1".to_string(),
        };

        let member2 = Spec {
            id: "driver.2".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Member 2".to_string()),
            body: "Member 2".to_string(),
        };

        let member3 = Spec {
            id: "driver.3".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Member 3".to_string()),
            body: "Member 3".to_string(),
        };

        let all_specs = vec![
            driver.clone(),
            member3.clone(),
            member1.clone(),
            member2.clone(),
        ];

        let result = collect_member_specs(&driver, &all_specs);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, "driver.1");
        assert_eq!(result[1].id, "driver.2");
        assert_eq!(result[2].id, "driver.3");
    }

    #[test]
    fn test_collect_member_specs_with_nested() {
        let driver = Spec {
            id: "driver".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let member1 = Spec {
            id: "driver.1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Member 1".to_string()),
            body: "Member 1".to_string(),
        };

        let member2 = Spec {
            id: "driver.2".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Member 2".to_string()),
            body: "Member 2".to_string(),
        };

        let all_specs = vec![driver.clone(), member1.clone(), member2.clone()];

        let result = collect_member_specs(&driver, &all_specs);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "driver.1");
        assert_eq!(result[1].id, "driver.2");
    }

    #[test]
    fn test_collect_member_specs_empty() {
        let driver = make_spec("driver", SpecStatus::Pending);
        let other = make_spec("other", SpecStatus::Pending);
        let all_specs = vec![driver.clone(), other];
        let result = collect_member_specs(&driver, &all_specs);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_is_driver_spec_with_members() {
        let driver = make_spec("driver", SpecStatus::Pending);
        let member1 = make_spec("driver.1", SpecStatus::Pending);
        let all_specs = vec![driver.clone(), member1];
        let result = is_driver_spec(&driver, &all_specs);
        assert!(result);
    }

    #[test]
    fn test_is_driver_spec_without_members() {
        let driver = make_spec("driver", SpecStatus::Pending);
        let other = make_spec("other", SpecStatus::Pending);
        let all_specs = vec![driver.clone(), other];
        let result = is_driver_spec(&driver, &all_specs);
        assert!(!result);
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_driver_spec_all_members_completed() {
        let driver = Spec {
            id: "driver".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let member1 = Spec {
            id: "driver.1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 1".to_string()),
            body: "Member 1".to_string(),
        };

        let member2 = Spec {
            id: "driver.2".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 2".to_string()),
            body: "Member 2".to_string(),
        };

        let member3 = Spec {
            id: "driver.3".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 3".to_string()),
            body: "Member 3".to_string(),
        };

        let all_specs = vec![driver.clone(), member1, member2, member3];

        // In dry-run mode, this should succeed (no branch validation)
        let result = merge_driver_spec(&driver, &all_specs, "spec-", "main", false, true);
        assert!(result.is_ok());
        let results = result.unwrap();
        // Should have 4 results: 3 members + 1 driver
        assert_eq!(results.len(), 4);
        // All should be in dry-run mode
        assert!(results.iter().all(|r| r.dry_run));
    }

    #[test]
    fn test_merge_driver_spec_member_pending() {
        let driver = Spec {
            id: "driver".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let member1 = Spec {
            id: "driver.1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 1".to_string()),
            body: "Member 1".to_string(),
        };

        let member2 = Spec {
            id: "driver.2".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Member 2".to_string()),
            body: "Member 2".to_string(),
        };

        let all_specs = vec![driver.clone(), member1, member2];

        let result = merge_driver_spec(&driver, &all_specs, "spec-", "main", false, true);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("Cannot merge driver spec"));
        assert!(error.contains("driver.2"));
        assert!(error.contains("incomplete"));
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_driver_spec_multiple_members_in_order() {
        let driver = Spec {
            id: "2026-01-24-01y-73b".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let member1 = Spec {
            id: "2026-01-24-01y-73b.1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 1".to_string()),
            body: "Member 1".to_string(),
        };

        let member2 = Spec {
            id: "2026-01-24-01y-73b.2".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 2".to_string()),
            body: "Member 2".to_string(),
        };

        let member3 = Spec {
            id: "2026-01-24-01y-73b.3".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 3".to_string()),
            body: "Member 3".to_string(),
        };

        let all_specs = vec![driver.clone(), member3, member1, member2];

        let result = merge_driver_spec(&driver, &all_specs, "spec-", "main", false, true);
        assert!(result.is_ok());
        let results = result.unwrap();
        // Should have 4 results in correct order: .1, .2, .3, driver
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].spec_id, "2026-01-24-01y-73b.1");
        assert_eq!(results[1].spec_id, "2026-01-24-01y-73b.2");
        assert_eq!(results[2].spec_id, "2026-01-24-01y-73b.3");
        assert_eq!(results[3].spec_id, "2026-01-24-01y-73b");
    }

    #[test]
    #[serial_test::serial]
    fn test_merge_driver_spec_dry_run_shows_all_merges() {
        let driver = Spec {
            id: "driver".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let member1 = Spec {
            id: "driver.1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 1".to_string()),
            body: "Member 1".to_string(),
        };

        let member2 = Spec {
            id: "driver.2".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Member 2".to_string()),
            body: "Member 2".to_string(),
        };

        let all_specs = vec![driver.clone(), member1, member2];

        let result = merge_driver_spec(&driver, &all_specs, "spec-", "main", false, true);
        assert!(result.is_ok());
        let results = result.unwrap();
        // 3 merges: member1, member2, driver
        assert_eq!(results.len(), 3);
        // All should be in dry-run mode
        assert!(results.iter().all(|r| r.dry_run));
        // All should be marked as success in dry-run
        assert!(results.iter().all(|r| r.success));
    }
}
