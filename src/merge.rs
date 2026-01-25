use crate::spec::{Spec, SpecStatus};
use crate::config::Config;
use anyhow::Result;

/// Load main_branch from config with fallback to "main"
#[allow(dead_code)]
pub fn load_main_branch(config: &Config) -> String {
    config.defaults.main_branch.clone()
}

/// Get the list of specs to merge based on arguments
/// Returns vector of (spec_id, Spec) tuples
#[allow(dead_code)]
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
        // Resolve each ID and collect matching specs
        for partial_id in args {
            // Find spec matching this partial ID
            let mut found = false;
            for spec in all_specs {
                if &spec.id == partial_id || spec.id.ends_with(partial_id) {
                    result.push((spec.id.clone(), spec.clone()));
                    found = true;
                    break;
                }
            }

            if !found {
                anyhow::bail!("Spec not found: {}", partial_id);
            }
        }
    }

    Ok(result)
}

/// Validate that a spec can be merged
#[allow(dead_code)]
pub fn validate_spec_can_merge(spec: &Spec, branch_exists: bool) -> Result<()> {
    // Check status is Completed
    if spec.frontmatter.status != SpecStatus::Completed {
        match spec.frontmatter.status {
            SpecStatus::Pending => {
                anyhow::bail!("Spec must be completed before merging");
            }
            SpecStatus::InProgress => {
                anyhow::bail!("Spec must be completed before merging");
            }
            SpecStatus::Failed => {
                anyhow::bail!("Cannot merge failed spec");
            }
            SpecStatus::Completed => {
                // This shouldn't be reached, but included for completeness
                anyhow::bail!("Spec must be completed before merging");
            }
        }
    }

    // Check branch exists
    if !branch_exists {
        anyhow::bail!("No branch found for spec: {}", spec.id);
    }

    Ok(())
}

/// Collect member specs of a driver spec in order (by sequence number)
#[allow(dead_code)]
pub fn collect_member_specs(driver_spec: &Spec, all_specs: &[Spec]) -> Vec<Spec> {
    let driver_id = &driver_spec.id;
    let mut members: Vec<(u32, Spec)> = Vec::new();

    for spec in all_specs {
        if is_member_of(&spec.id, driver_id) {
            // Extract sequence number from member ID
            if let Some(seq_num) = extract_sequence_number(&spec.id, driver_id) {
                members.push((seq_num, spec.clone()));
            }
        }
    }

    // Sort by sequence number
    members.sort_by_key(|m| m.0);

    // Return just the specs
    members.into_iter().map(|(_, spec)| spec).collect()
}

/// Check if `member_id` is a group member of `driver_id`.
#[allow(dead_code)]
fn is_member_of(member_id: &str, driver_id: &str) -> bool {
    if !member_id.starts_with(driver_id) {
        return false;
    }

    let suffix = &member_id[driver_id.len()..];
    suffix.starts_with('.') && suffix.len() > 1
}

/// Extract the first sequence number from a member ID
/// For "driver.1" returns Some(1)
/// For "driver.1.2" returns Some(1)
#[allow(dead_code)]
fn extract_sequence_number(member_id: &str, driver_id: &str) -> Option<u32> {
    let suffix = &member_id[driver_id.len()..];
    if !suffix.starts_with('.') {
        return None;
    }

    let parts: Vec<&str> = suffix[1..].split('.').collect();
    if parts.is_empty() {
        return None;
    }

    parts[0].parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DefaultsConfig;

    #[test]
    fn test_load_main_branch_default() {
        let config = Config {
            project: crate::config::ProjectConfig {
                name: "test".to_string(),
                prefix: None,
            },
            defaults: DefaultsConfig::default(),
            git: crate::config::GitConfig::default(),
        };

        let branch = load_main_branch(&config);
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_load_main_branch_custom() {
        let config = Config {
            project: crate::config::ProjectConfig {
                name: "test".to_string(),
                prefix: None,
            },
            defaults: DefaultsConfig {
                main_branch: "master".to_string(),
                ..Default::default()
            },
            git: crate::config::GitConfig::default(),
        };

        let branch = load_main_branch(&config);
        assert_eq!(branch, "master");
    }

    #[test]
    fn test_get_specs_to_merge_all() {
        let specs = vec![
            Spec {
                id: "spec1".to_string(),
                frontmatter: crate::spec::SpecFrontmatter {
                    status: SpecStatus::Completed,
                    ..Default::default()
                },
                title: Some("Spec 1".to_string()),
                body: "Body 1".to_string(),
            },
            Spec {
                id: "spec2".to_string(),
                frontmatter: crate::spec::SpecFrontmatter {
                    status: SpecStatus::Pending,
                    ..Default::default()
                },
                title: Some("Spec 2".to_string()),
                body: "Body 2".to_string(),
            },
            Spec {
                id: "spec3".to_string(),
                frontmatter: crate::spec::SpecFrontmatter {
                    status: SpecStatus::Completed,
                    ..Default::default()
                },
                title: Some("Spec 3".to_string()),
                body: "Body 3".to_string(),
            },
        ];

        let result = get_specs_to_merge(&[], true, &specs).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "spec1");
        assert_eq!(result[1].0, "spec3");
    }

    #[test]
    fn test_get_specs_to_merge_specific() {
        let specs = vec![
            Spec {
                id: "spec1".to_string(),
                frontmatter: crate::spec::SpecFrontmatter {
                    status: SpecStatus::Completed,
                    ..Default::default()
                },
                title: Some("Spec 1".to_string()),
                body: "Body 1".to_string(),
            },
            Spec {
                id: "spec2".to_string(),
                frontmatter: crate::spec::SpecFrontmatter {
                    status: SpecStatus::Completed,
                    ..Default::default()
                },
                title: Some("Spec 2".to_string()),
                body: "Body 2".to_string(),
            },
        ];

        let result =
            get_specs_to_merge(&["spec1".to_string()], false, &specs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "spec1");
    }

    #[test]
    fn test_get_specs_to_merge_not_found() {
        let specs = vec![Spec {
            id: "spec1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Spec 1".to_string()),
            body: "Body 1".to_string(),
        }];

        let result = get_specs_to_merge(&["nonexistent".to_string()], false, &specs);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Spec not found"));
    }

    #[test]
    fn test_validate_spec_can_merge_completed() {
        let spec = Spec {
            id: "spec1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Spec 1".to_string()),
            body: "Body 1".to_string(),
        };

        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_spec_can_merge_pending_fails() {
        let spec = Spec {
            id: "spec1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Spec 1".to_string()),
            body: "Body 1".to_string(),
        };

        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Spec must be completed before merging"));
    }

    #[test]
    fn test_validate_spec_can_merge_in_progress_fails() {
        let spec = Spec {
            id: "spec1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Spec 1".to_string()),
            body: "Body 1".to_string(),
        };

        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Spec must be completed before merging"));
    }

    #[test]
    fn test_validate_spec_can_merge_failed_fails() {
        let spec = Spec {
            id: "spec1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Failed,
                ..Default::default()
            },
            title: Some("Spec 1".to_string()),
            body: "Body 1".to_string(),
        };

        let result = validate_spec_can_merge(&spec, true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot merge failed spec"));
    }

    #[test]
    fn test_validate_spec_can_merge_no_branch() {
        let spec = Spec {
            id: "spec1".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Completed,
                ..Default::default()
            },
            title: Some("Spec 1".to_string()),
            body: "Body 1".to_string(),
        };

        let result = validate_spec_can_merge(&spec, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No branch found"));
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
        let driver = Spec {
            id: "driver".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Driver".to_string()),
            body: "Driver".to_string(),
        };

        let other = Spec {
            id: "other".to_string(),
            frontmatter: crate::spec::SpecFrontmatter::default(),
            title: Some("Other".to_string()),
            body: "Other".to_string(),
        };

        let all_specs = vec![driver.clone(), other];

        let result = collect_member_specs(&driver, &all_specs);
        assert_eq!(result.len(), 0);
    }
}
