//! Spec group/driver orchestration logic.
//!
//! This module manages spec membership and group completion tracking for driver specs.
//! Driver specs can have member specs identified by numeric suffixes (e.g., `.1`, `.2`).
//! This module handles relationships between drivers and their members.

use crate::spec::{Spec, SpecStatus};
use anyhow::Result;
use std::path::Path;

/// Check if `member_id` is a group member of `driver_id`.
///
/// Member IDs have format: `DRIVER_ID.N` or `DRIVER_ID.N.M` where N and M are numbers.
/// For example: `2026-01-25-00y-abc.1` is a member of `2026-01-25-00y-abc`.
///
/// # Examples
///
/// ```ignore
/// assert!(is_member_of("2026-01-25-00y-abc.1", "2026-01-25-00y-abc"));
/// assert!(is_member_of("2026-01-25-00y-abc.2.1", "2026-01-25-00y-abc"));
/// assert!(!is_member_of("2026-01-25-00y-abc", "2026-01-25-00y-abc")); // Not a member
/// assert!(!is_member_of("2026-01-25-00x-xyz", "2026-01-25-00y-abc")); // Different driver
/// ```
pub fn is_member_of(member_id: &str, driver_id: &str) -> bool {
    // Member IDs have format: DRIVER_ID.N or DRIVER_ID.N.M
    if !member_id.starts_with(driver_id) {
        return false;
    }

    let suffix = &member_id[driver_id.len()..];
    suffix.starts_with('.') && suffix.len() > 1
}

/// Get all member specs of a driver spec.
///
/// Returns a vector of references to all spec members of the given driver.
/// If the driver has no members, returns an empty vector.
///
/// # Arguments
///
/// * `driver_id` - The ID of the driver spec
/// * `specs` - All available specs to search
///
/// # Examples
///
/// ```ignore
/// let members = get_members("2026-01-25-00y-abc", &specs);
/// ```
pub fn get_members<'a>(driver_id: &str, specs: &'a [Spec]) -> Vec<&'a Spec> {
    specs
        .iter()
        .filter(|s| is_member_of(&s.id, driver_id))
        .collect()
}

/// Check if all members of a driver spec are completed.
///
/// Returns true if:
/// - The driver has no members, or
/// - All members have status `Completed` or `Cancelled`
///
/// # Arguments
///
/// * `driver_id` - The ID of the driver spec
/// * `specs` - All available specs
///
/// # Examples
///
/// ```ignore
/// if all_members_completed("2026-01-25-00y-abc", &specs) {
///     println!("All members are done!");
/// }
/// ```
pub fn all_members_completed(driver_id: &str, specs: &[Spec]) -> bool {
    let members = get_members(driver_id, specs);
    if members.is_empty() {
        return true; // No members, so all are "completed"
    }
    members.iter().all(|m| {
        m.frontmatter.status == SpecStatus::Completed
            || m.frontmatter.status == SpecStatus::Cancelled
    })
}

/// Get list of incomplete member spec IDs for a driver spec.
///
/// Returns a vector of IDs for all members that are not in `Completed` status.
/// Returns an empty vector if the spec is not a driver or has no incomplete members.
///
/// # Arguments
///
/// * `driver_id` - The ID of the driver spec
/// * `all_specs` - All available specs
///
/// # Examples
///
/// ```ignore
/// let incomplete = get_incomplete_members("2026-01-25-00y-abc", &specs);
/// for member_id in incomplete {
///     println!("Incomplete member: {}", member_id);
/// }
/// ```
pub fn get_incomplete_members(driver_id: &str, all_specs: &[Spec]) -> Vec<String> {
    get_members(driver_id, all_specs)
        .into_iter()
        .filter(|m| m.frontmatter.status != SpecStatus::Completed)
        .map(|m| m.id.clone())
        .collect()
}

/// Extract the driver ID from a member ID.
///
/// For member specs with numeric suffixes, returns the base driver ID.
/// For non-member specs, returns None.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(extract_driver_id("2026-01-25-00y-abc.1"), Some("2026-01-25-00y-abc".to_string()));
/// assert_eq!(extract_driver_id("2026-01-25-00y-abc.3.2"), Some("2026-01-25-00y-abc".to_string()));
/// assert_eq!(extract_driver_id("2026-01-25-00y-abc"), None);
/// assert_eq!(extract_driver_id("2026-01-25-00y-abc.abc"), None);
/// ```
pub fn extract_driver_id(member_id: &str) -> Option<String> {
    // Member IDs have format: DRIVER_ID.N or DRIVER_ID.N.M
    if let Some(pos) = member_id.find('.') {
        let (prefix, suffix) = member_id.split_at(pos);
        // Check that what follows the dot is numeric (at least up to the first non-digit)
        if suffix.len() > 1
            && suffix[1..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        {
            return Some(prefix.to_string());
        }
    }
    None
}

/// Extract the member number from a member ID.
///
/// For member specs with format `DRIVER_ID.N` or `DRIVER_ID.N.M`, extracts `N`.
/// For non-member specs, returns None.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(extract_member_number("2026-01-25-00y-abc.1"), Some(1));
/// assert_eq!(extract_member_number("2026-01-25-00y-abc.3"), Some(3));
/// assert_eq!(extract_member_number("2026-01-25-00y-abc.10"), Some(10));
/// assert_eq!(extract_member_number("2026-01-25-00y-abc.3.2"), Some(3));
/// assert_eq!(extract_member_number("2026-01-25-00y-abc"), None);
/// assert_eq!(extract_member_number("2026-01-25-00y-abc.abc"), None);
/// ```
pub fn extract_member_number(member_id: &str) -> Option<u32> {
    if let Some(pos) = member_id.find('.') {
        let suffix = &member_id[pos + 1..];
        // Extract just the first numeric part after the dot
        let num_str: String = suffix.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !num_str.is_empty() {
            return num_str.parse::<u32>().ok();
        }
    }
    None
}

/// Compare two spec IDs with numeric sorting for member specs and base36 sequences.
///
/// This function provides a natural sort order where member spec numbers and base36
/// sequence portions are compared numerically rather than lexicographically. This ensures:
/// - Specs like `2026-01-25-00y-abc.10` sort after `2026-01-25-00y-abc.2`
/// - Specs like `2026-01-25-010-xxx` sort after `2026-01-25-00z-yyy`
///
/// # Sorting behavior
///
/// - For non-member specs, parses date/sequence/suffix and compares sequence numerically
/// - For member specs (with `.N` suffix), compares the base ID using date/sequence/suffix
///   parsing first, then compares member numbers numerically
/// - Mixed member/non-member specs: non-members sort before members with the same base
///
/// # Examples
///
/// ```ignore
/// use std::cmp::Ordering;
/// assert_eq!(compare_spec_ids("2026-01-25-00y-abc.2", "2026-01-25-00y-abc.10"), Ordering::Less);
/// assert_eq!(compare_spec_ids("2026-01-25-00y-abc.10", "2026-01-25-00y-abc.2"), Ordering::Greater);
/// assert_eq!(compare_spec_ids("2026-01-25-00y-abc", "2026-01-25-00y-def"), Ordering::Less);
/// assert_eq!(compare_spec_ids("2026-01-25-00y-abc", "2026-01-25-00y-abc.1"), Ordering::Less);
/// assert_eq!(compare_spec_ids("2026-01-25-010-xxx", "2026-01-25-00z-yyy"), Ordering::Greater);
/// ```
pub fn compare_spec_ids(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Try to extract driver IDs and member numbers
    let a_driver = extract_driver_id(a);
    let b_driver = extract_driver_id(b);

    match (a_driver, b_driver) {
        (Some(a_base), Some(b_base)) => {
            // Both are member specs
            // First compare the base IDs with sequence parsing
            match compare_base_ids(&a_base, &b_base) {
                Ordering::Equal => {
                    // Same base ID, compare member numbers numerically
                    let a_num = extract_member_number(a).unwrap_or(u32::MAX);
                    let b_num = extract_member_number(b).unwrap_or(u32::MAX);
                    a_num.cmp(&b_num)
                }
                other => other,
            }
        }
        (Some(a_base), None) => {
            // a is a member, b is not
            // Compare a's base with b using sequence parsing
            match compare_base_ids(&a_base, b) {
                Ordering::Equal => {
                    // b is the driver of a, so b comes first
                    Ordering::Greater
                }
                other => other,
            }
        }
        (None, Some(b_base)) => {
            // a is not a member, b is
            // Compare a with b's base using sequence parsing
            match compare_base_ids(a, &b_base) {
                Ordering::Equal => {
                    // a is the driver of b, so a comes first
                    Ordering::Less
                }
                other => other,
            }
        }
        (None, None) => {
            // Neither are member specs, use sequence parsing
            compare_base_ids(a, b)
        }
    }
}

/// Compare two base spec IDs by parsing date, sequence, and suffix.
///
/// Spec IDs have format: YYYY-MM-DD-SSS-XXX where:
/// - YYYY-MM-DD is the date (compared lexicographically)
/// - SSS is a base36 sequence (compared numerically)
/// - XXX is a random base36 suffix (compared lexicographically as tiebreaker)
fn compare_base_ids(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Parse both IDs into (date, sequence, suffix)
    let a_parts = parse_spec_id_parts(a);
    let b_parts = parse_spec_id_parts(b);

    match (a_parts, b_parts) {
        (Some((a_date, a_seq, a_suffix)), Some((b_date, b_seq, b_suffix))) => {
            // Compare date lexicographically
            match a_date.cmp(b_date) {
                Ordering::Equal => {
                    // Same date, compare sequence numerically
                    match a_seq.cmp(&b_seq) {
                        Ordering::Equal => {
                            // Same sequence, compare suffix lexicographically
                            a_suffix.cmp(b_suffix)
                        }
                        other => other,
                    }
                }
                other => other,
            }
        }
        // If parsing fails, fall back to lexicographic comparison
        _ => a.cmp(b),
    }
}

/// Parse a spec ID into (date, sequence_number, suffix).
/// Returns None if the ID doesn't match the expected format.
fn parse_spec_id_parts(id: &str) -> Option<(&str, u32, &str)> {
    let parts: Vec<&str> = id.split('-').collect();

    // Expected format: YYYY-MM-DD-SSS-XXX (5 parts minimum)
    if parts.len() < 5 {
        return None;
    }

    // Date is parts[0..3] joined: YYYY-MM-DD
    let date = &id[..10]; // "YYYY-MM-DD" is always 10 chars

    // Sequence is parts[3], parse from base36
    let seq = crate::id::parse_base36(parts[3])?;

    // Suffix is parts[4]
    let suffix = parts[4];

    Some((date, seq, suffix))
}

/// Check if all prior siblings of a member spec are completed.
///
/// For a member spec like `DRIVER_ID.3`, checks that `DRIVER_ID.1` and `DRIVER_ID.2`
/// are both in `Completed` status. For `DRIVER_ID.1`, returns true (no prior siblings).
/// For non-member specs, returns true (sibling check doesn't apply).
///
/// # Arguments
///
/// * `member_id` - The ID of the member spec to check
/// * `all_specs` - All available specs
///
/// # Examples
///
/// ```ignore
/// // For a spec DRIVER_ID.3, checks that DRIVER_ID.1 and DRIVER_ID.2 are completed
/// assert!(all_prior_siblings_completed("2026-01-25-00y-abc.3", &specs));
/// ```
pub fn all_prior_siblings_completed(member_id: &str, all_specs: &[Spec]) -> bool {
    // Find the current member spec
    if let Some(member_spec) = all_specs.iter().find(|s| s.id == member_id) {
        // If member has explicit depends_on, skip sequential check (use DAG dependencies instead)
        if member_spec.frontmatter.depends_on.is_some() {
            return true;
        }
    }

    // Fall back to sequential ordering if no explicit dependencies
    if let Some(driver_id) = extract_driver_id(member_id) {
        if let Some(member_num) = extract_member_number(member_id) {
            // Check all specs with numbers less than member_num
            for i in 1..member_num {
                let sibling_id = format!("{}.{}", driver_id, i);
                let sibling = all_specs.iter().find(|s| s.id == sibling_id);
                if let Some(s) = sibling {
                    if s.frontmatter.status != SpecStatus::Completed
                        && s.frontmatter.status != SpecStatus::Cancelled
                    {
                        return false;
                    }
                }
                // Missing siblings are skipped — don't block on deleted/nonexistent specs
            }
            return true;
        }
    }
    // Not a member spec, so this check doesn't apply
    true
}

/// Mark the driver spec as in_progress if the current spec is a member.
///
/// When a member spec begins execution, its driver spec should transition from
/// `Pending` to `InProgress` (if not already). This function handles that transition.
///
/// # Arguments
///
/// * `specs_dir` - Path to the specs directory
/// * `member_id` - The ID of the member spec that is starting
///
/// # Returns
///
/// Returns `Ok(())` if successful or the driver doesn't exist.
/// Returns `Err` if file I/O fails.
///
/// # Examples
///
/// ```ignore
/// mark_driver_in_progress(&specs_dir, "2026-01-25-00y-abc.1")?;
/// ```
/// Mark a driver spec as in_progress when one of its members starts work.
///
/// This creates a "phantom" in_progress status for the driver that serves as a placeholder
/// until all members complete. The driver will be auto-completed when the last member finishes.
///
/// Note: This means that during member execution, both the driver AND the active member
/// will show as in_progress. This is by design but can be confusing in status displays.
///
/// Set `skip` to true to avoid marking the driver (useful in chain mode where we want
/// only one spec to show as in_progress at a time).
pub fn mark_driver_in_progress_conditional(
    specs_dir: &Path,
    member_id: &str,
    skip: bool,
) -> Result<()> {
    use crate::spec::TransitionBuilder;

    if skip {
        return Ok(());
    }

    if let Some(driver_id) = extract_driver_id(member_id) {
        // Try to load the driver spec
        let driver_path = specs_dir.join(format!("{}.md", driver_id));
        if driver_path.exists() {
            let mut driver = Spec::load(&driver_path)?;
            if driver.frontmatter.status == SpecStatus::Pending {
                TransitionBuilder::new(&mut driver).to(SpecStatus::InProgress)?;
                driver.save(&driver_path)?;
            }
        }
    }
    Ok(())
}

/// Mark a driver spec as in_progress when one of its members starts work.
///
/// Convenience wrapper that always marks the driver. For conditional marking,
/// use `mark_driver_in_progress_conditional`.
pub fn mark_driver_in_progress(specs_dir: &Path, member_id: &str) -> Result<()> {
    mark_driver_in_progress_conditional(specs_dir, member_id, false)
}

/// Auto-complete a driver spec if all its members are now completed.
///
/// When a member spec completes, check if all other members are also completed.
/// If so, and the driver is in `InProgress` status, automatically mark the driver
/// as `Completed` with completion timestamp.
///
/// # Arguments
///
/// * `member_id` - The ID of the member spec that just completed
/// * `all_specs` - All available specs
/// * `specs_dir` - Path to the specs directory
///
/// # Returns
///
/// Returns `Ok(true)` if the driver was auto-completed.
/// Returns `Ok(false)` if the driver was not ready for completion.
/// Returns `Err` if file I/O fails.
///
/// # Examples
///
/// ```ignore
/// if auto_complete_driver_if_ready("2026-01-25-00y-abc.2", &specs, &specs_dir)? {
///     println!("Driver was auto-completed!");
/// }
/// ```
pub fn auto_complete_driver_if_ready(
    member_id: &str,
    all_specs: &[Spec],
    specs_dir: &Path,
) -> Result<bool> {
    // Only member specs can trigger driver auto-completion
    let Some(driver_id) = extract_driver_id(member_id) else {
        return Ok(false);
    };

    // Find the driver spec
    let Some(driver_spec) = all_specs.iter().find(|s| s.id == driver_id) else {
        return Ok(false);
    };

    // Only auto-complete if driver is in_progress or pending
    // (Pending is allowed for chain mode where drivers aren't marked InProgress)
    if driver_spec.frontmatter.status != SpecStatus::InProgress
        && driver_spec.frontmatter.status != SpecStatus::Pending
    {
        return Ok(false);
    }

    // Check if all members are completed
    if !all_members_completed(&driver_id, all_specs) {
        return Ok(false);
    }

    // All members are completed, so auto-complete the driver
    let driver_path = specs_dir.join(format!("{}.md", driver_id));
    let mut driver = Spec::load(&driver_path)?;

    use crate::spec::TransitionBuilder;
    // Use force() to allow Pending→Completed transition in chain mode
    TransitionBuilder::new(&mut driver)
        .force()
        .to(SpecStatus::Completed)?;
    driver.frontmatter.completed_at = Some(crate::utc_now_iso());
    driver.frontmatter.model = Some("auto-completed".to_string());

    driver.save(&driver_path)?;

    Ok(true)
}

/// Mark a driver spec as failed when one of its members fails.
///
/// When a member spec fails during chain execution, the driver spec should be marked
/// as `Failed` to indicate partial group failure and prevent it from appearing ready.
///
/// # Arguments
///
/// * `member_id` - The ID of the member spec that failed
/// * `specs_dir` - Path to the specs directory
///
/// # Returns
///
/// Returns `Ok(true)` if the driver was marked as failed.
/// Returns `Ok(false)` if there is no driver or driver doesn't need updating.
/// Returns `Err` if file I/O fails.
pub fn mark_driver_failed_on_member_failure(member_id: &str, specs_dir: &Path) -> Result<bool> {
    // Only member specs can trigger driver failure
    let Some(driver_id) = extract_driver_id(member_id) else {
        return Ok(false);
    };

    // Try to load the driver spec
    let driver_path = specs_dir.join(format!("{}.md", driver_id));
    if !driver_path.exists() {
        return Ok(false);
    }

    let mut driver = Spec::load(&driver_path)?;

    // Only mark as failed if driver is InProgress or Pending
    // (already failed drivers should stay failed)
    if driver.frontmatter.status != SpecStatus::InProgress
        && driver.frontmatter.status != SpecStatus::Pending
    {
        return Ok(false);
    }

    // Mark driver as failed
    use crate::spec::TransitionBuilder;
    TransitionBuilder::new(&mut driver)
        .force()
        .to(SpecStatus::Failed)?;

    driver.save(&driver_path)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_member_of() {
        assert!(is_member_of("2026-01-22-001-x7m.1", "2026-01-22-001-x7m"));
        assert!(is_member_of("2026-01-22-001-x7m.2.1", "2026-01-22-001-x7m"));
        assert!(!is_member_of("2026-01-22-001-x7m", "2026-01-22-001-x7m"));
        assert!(!is_member_of("2026-01-22-002-y8n", "2026-01-22-001-x7m"));
    }

    #[test]
    fn test_extract_driver_id() {
        assert_eq!(
            extract_driver_id("2026-01-22-001-x7m.1"),
            Some("2026-01-22-001-x7m".to_string())
        );
        assert_eq!(
            extract_driver_id("2026-01-22-001-x7m.2.1"),
            Some("2026-01-22-001-x7m".to_string())
        );
        assert_eq!(extract_driver_id("2026-01-22-001-x7m"), None);
        assert_eq!(extract_driver_id("2026-01-22-001-x7m.abc"), None);
    }

    #[test]
    fn test_extract_member_number() {
        assert_eq!(extract_member_number("2026-01-24-001-abc.1"), Some(1));
        assert_eq!(extract_member_number("2026-01-24-001-abc.3"), Some(3));
        assert_eq!(extract_member_number("2026-01-24-001-abc.10"), Some(10));
        assert_eq!(extract_member_number("2026-01-24-001-abc.3.2"), Some(3));
        assert_eq!(extract_member_number("2026-01-24-001-abc"), None);
        assert_eq!(extract_member_number("2026-01-24-001-abc.abc"), None);
    }

    #[test]
    fn test_all_prior_siblings_completed() {
        // Test spec for member .1 with no prior siblings
        let spec1 = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        // Should be ready since it has no prior siblings
        assert!(all_prior_siblings_completed(&spec1.id, &[]));

        // Test spec for member .3 with completed prior siblings
        let spec_prior_1 = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: completed
---
# Test
"#,
        )
        .unwrap();

        let spec_prior_2 = Spec::parse(
            "2026-01-24-001-abc.2",
            r#"---
status: completed
---
# Test
"#,
        )
        .unwrap();

        let spec3 = Spec::parse(
            "2026-01-24-001-abc.3",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let all_specs = vec![spec_prior_1, spec_prior_2, spec3.clone()];
        assert!(all_prior_siblings_completed(&spec3.id, &all_specs));
    }

    #[test]
    fn test_all_prior_siblings_completed_missing_skipped() {
        // Missing siblings should not block later members
        let spec_prior_1 = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: completed
---
# Test
"#,
        )
        .unwrap();

        let spec3 = Spec::parse(
            "2026-01-24-001-abc.3",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        // Only spec .1 exists, .2 is missing — should still pass
        let all_specs = vec![spec_prior_1, spec3.clone()];
        assert!(all_prior_siblings_completed(&spec3.id, &all_specs));
    }

    #[test]
    fn test_all_prior_siblings_completed_cancelled() {
        // Cancelled siblings should not block later members
        let spec_prior_1 = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: cancelled
---
# Test
"#,
        )
        .unwrap();

        let spec2 = Spec::parse(
            "2026-01-24-001-abc.2",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let all_specs = vec![spec_prior_1, spec2.clone()];
        assert!(all_prior_siblings_completed(&spec2.id, &all_specs));
    }

    #[test]
    fn test_all_prior_siblings_completed_not_completed() {
        // Test spec for member .2 with incomplete prior sibling
        let spec_prior_1 = Spec::parse(
            "2026-01-24-001-abc.1",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let spec2 = Spec::parse(
            "2026-01-24-001-abc.2",
            r#"---
status: pending
---
# Test
"#,
        )
        .unwrap();

        let all_specs = vec![spec_prior_1, spec2.clone()];
        assert!(!all_prior_siblings_completed(&spec2.id, &all_specs));
    }

    #[test]
    fn test_mark_driver_in_progress_when_member_starts() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is pending
        let driver_spec = Spec {
            id: "2026-01-24-001-abc".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
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
            frontmatter: crate::spec::SpecFrontmatter {
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
    fn test_mark_driver_in_progress_nonexistent_driver() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Try to mark driver as in_progress when driver doesn't exist
        // Should not error, just skip
        mark_driver_in_progress(specs_dir, "2026-01-24-003-ghi.1").unwrap();
    }

    #[test]
    fn test_get_incomplete_members() {
        // Driver with multiple incomplete members
        let driver = Spec::parse(
            "2026-01-24-005-mno",
            r#"---
status: in_progress
---
# Driver
"#,
        )
        .unwrap();

        let member1 = Spec::parse(
            "2026-01-24-005-mno.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let member2 = Spec::parse(
            "2026-01-24-005-mno.2",
            r#"---
status: pending
---
# Member 2
"#,
        )
        .unwrap();

        let member3 = Spec::parse(
            "2026-01-24-005-mno.3",
            r#"---
status: in_progress
---
# Member 3
"#,
        )
        .unwrap();

        let all_specs = vec![driver.clone(), member1, member2, member3];
        let incomplete = get_incomplete_members(&driver.id, &all_specs);
        assert_eq!(incomplete.len(), 2);
        assert!(incomplete.contains(&"2026-01-24-005-mno.2".to_string()));
        assert!(incomplete.contains(&"2026-01-24-005-mno.3".to_string()));
    }

    #[test]
    fn test_auto_complete_driver_not_member_spec() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // A non-member spec should not trigger auto-completion
        let driver_spec = Spec::parse(
            "2026-01-24-006-pqr",
            r#"---
status: in_progress
---
# Driver spec
"#,
        )
        .unwrap();

        let result =
            auto_complete_driver_if_ready("2026-01-24-006-pqr", &[driver_spec], specs_dir).unwrap();
        assert!(
            !result,
            "Non-member spec should not trigger auto-completion"
        );
    }

    #[test]
    fn test_auto_complete_driver_when_already_completed() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is already completed
        let driver_spec = Spec::parse(
            "2026-01-24-007-stu",
            r#"---
status: completed
---
# Driver spec
"#,
        )
        .unwrap();

        let member_spec = Spec::parse(
            "2026-01-24-007-stu.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let all_specs = vec![driver_spec, member_spec];
        let result =
            auto_complete_driver_if_ready("2026-01-24-007-stu.1", &all_specs, specs_dir).unwrap();
        assert!(
            !result,
            "Driver already completed should not be re-completed"
        );
    }

    #[test]
    fn test_auto_complete_driver_from_pending() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create driver spec file (pending status)
        let driver_content = r#"---
status: pending
---
# Driver spec
"#;
        fs::write(specs_dir.join("2026-01-24-009-xyz.md"), driver_content).unwrap();

        // Parse specs for all_specs
        let driver_spec = Spec::parse("2026-01-24-009-xyz", driver_content).unwrap();

        let member_spec = Spec::parse(
            "2026-01-24-009-xyz.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let all_specs = vec![driver_spec, member_spec];

        // Auto-complete should succeed for pending driver (chain mode scenario)
        let result =
            auto_complete_driver_if_ready("2026-01-24-009-xyz.1", &all_specs, specs_dir).unwrap();
        assert!(
            result,
            "Pending driver should be auto-completed when all members are done (chain mode)"
        );

        // Verify driver is now completed
        let updated_driver = Spec::load(&specs_dir.join("2026-01-24-009-xyz.md")).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::Completed);
    }

    #[test]
    fn test_auto_complete_driver_incomplete_members() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is in_progress
        let driver_spec = Spec {
            id: "2026-01-24-008-vwx".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "# Driver\n\nBody.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-008-vwx.md");
        driver_spec.save(&driver_path).unwrap();

        // Create member specs where not all are completed
        let member1 = Spec::parse(
            "2026-01-24-008-vwx.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let member2 = Spec::parse(
            "2026-01-24-008-vwx.2",
            r#"---
status: in_progress
---
# Member 2
"#,
        )
        .unwrap();

        let all_specs = vec![driver_spec, member1, member2];
        let result =
            auto_complete_driver_if_ready("2026-01-24-008-vwx.1", &all_specs, specs_dir).unwrap();
        assert!(
            !result,
            "Driver should not complete when members are incomplete"
        );
    }

    #[test]
    fn test_auto_complete_driver_success() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that is in_progress
        let driver_spec = Spec {
            id: "2026-01-24-009-yz0".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "# Driver\n\nBody.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-009-yz0.md");
        driver_spec.save(&driver_path).unwrap();

        // Create member specs where all are completed
        let member1 = Spec::parse(
            "2026-01-24-009-yz0.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let member2 = Spec::parse(
            "2026-01-24-009-yz0.2",
            r#"---
status: completed
---
# Member 2
"#,
        )
        .unwrap();

        let all_specs = vec![driver_spec, member1, member2];

        // Auto-complete should succeed
        let result =
            auto_complete_driver_if_ready("2026-01-24-009-yz0.2", &all_specs, specs_dir).unwrap();
        assert!(
            result,
            "Driver should be auto-completed when all members are completed"
        );

        // Verify driver was updated
        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::Completed);
        assert_eq!(
            updated_driver.frontmatter.model,
            Some("auto-completed".to_string())
        );
        assert!(updated_driver.frontmatter.completed_at.is_some());
    }

    #[test]
    fn test_auto_complete_driver_nonexistent_driver() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Try to auto-complete when driver doesn't exist
        let all_specs = vec![];
        let result =
            auto_complete_driver_if_ready("2026-01-24-010-abc.1", &all_specs, specs_dir).unwrap();
        assert!(
            !result,
            "Should return false when driver spec doesn't exist"
        );
    }

    #[test]
    fn test_auto_complete_driver_single_member() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Driver with single member
        let driver_spec = Spec {
            id: "2026-01-24-011-def".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::InProgress,
                ..Default::default()
            },
            title: Some("Driver".to_string()),
            body: "# Driver\n\nBody.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-011-def.md");
        driver_spec.save(&driver_path).unwrap();

        // Single member
        let member = Spec::parse(
            "2026-01-24-011-def.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let all_specs = vec![driver_spec, member];

        // Auto-complete should succeed
        let result =
            auto_complete_driver_if_ready("2026-01-24-011-def.1", &all_specs, specs_dir).unwrap();
        assert!(
            result,
            "Driver should be auto-completed when single member completes"
        );

        // Verify driver was updated
        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(updated_driver.frontmatter.status, SpecStatus::Completed);
        assert_eq!(
            updated_driver.frontmatter.model,
            Some("auto-completed".to_string())
        );
    }

    #[test]
    fn test_compare_spec_ids_member_numeric_sort() {
        use std::cmp::Ordering;

        // Test numeric sorting for member specs
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc.2", "2026-01-25-00y-abc.10"),
            Ordering::Less
        );
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc.10", "2026-01-25-00y-abc.2"),
            Ordering::Greater
        );
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc.1", "2026-01-25-00y-abc.1"),
            Ordering::Equal
        );

        // Test with larger numbers
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc.99", "2026-01-25-00y-abc.100"),
            Ordering::Less
        );
    }

    #[test]
    fn test_compare_spec_ids_different_drivers() {
        use std::cmp::Ordering;

        // Different driver IDs should use lexicographic comparison
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc.1", "2026-01-25-00y-def.1"),
            Ordering::Less
        );
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-def.1", "2026-01-25-00y-abc.1"),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_spec_ids_non_member_specs() {
        use std::cmp::Ordering;

        // Non-member specs should use lexicographic comparison
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc", "2026-01-25-00y-def"),
            Ordering::Less
        );
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-def", "2026-01-25-00y-abc"),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_spec_ids_driver_vs_member() {
        use std::cmp::Ordering;

        // Driver should come before its members
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc", "2026-01-25-00y-abc.1"),
            Ordering::Less
        );
        assert_eq!(
            compare_spec_ids("2026-01-25-00y-abc.1", "2026-01-25-00y-abc"),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_spec_ids_sorting_list() {
        // Test sorting a list of specs with mixed member numbers
        let mut ids = vec![
            "2026-01-25-00y-abc.10",
            "2026-01-25-00y-abc.2",
            "2026-01-25-00y-abc.1",
            "2026-01-25-00y-abc",
            "2026-01-25-00y-abc.3",
        ];

        ids.sort_by(|a, b| compare_spec_ids(a, b));

        assert_eq!(
            ids,
            vec![
                "2026-01-25-00y-abc",
                "2026-01-25-00y-abc.1",
                "2026-01-25-00y-abc.2",
                "2026-01-25-00y-abc.3",
                "2026-01-25-00y-abc.10",
            ]
        );
    }

    #[test]
    fn test_compare_spec_ids_base36_sequence_rollover() {
        use std::cmp::Ordering;

        // Test that base36 sequence 010 (decimal 36) sorts after 00z (decimal 35)
        assert_eq!(
            compare_spec_ids("2026-01-25-010-xxx", "2026-01-25-00z-yyy"),
            Ordering::Greater
        );
        assert_eq!(
            compare_spec_ids("2026-01-25-00z-yyy", "2026-01-25-010-xxx"),
            Ordering::Less
        );

        // Test sorting a list with base36 rollover
        let mut ids = vec![
            "2026-01-25-010-aaa",
            "2026-01-25-00a-bbb",
            "2026-01-25-00z-ccc",
            "2026-01-25-001-ddd",
            "2026-01-25-011-eee",
        ];

        ids.sort_by(|a, b| compare_spec_ids(a, b));

        assert_eq!(
            ids,
            vec![
                "2026-01-25-001-ddd", // 1
                "2026-01-25-00a-bbb", // 10
                "2026-01-25-00z-ccc", // 35
                "2026-01-25-010-aaa", // 36
                "2026-01-25-011-eee", // 37
            ]
        );
    }

    #[test]
    fn test_driver_auto_completion_with_two_members() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path();

        // Create a driver spec that starts as pending
        let driver_spec = Spec {
            id: "2026-01-24-012-ghi".to_string(),
            frontmatter: crate::spec::SpecFrontmatter {
                status: SpecStatus::Pending,
                ..Default::default()
            },
            title: Some("Driver spec with 2 members".to_string()),
            body: "# Driver\n\nBody.".to_string(),
        };

        let driver_path = specs_dir.join("2026-01-24-012-ghi.md");
        driver_spec.save(&driver_path).unwrap();

        // Create first member (initially pending)
        let _member1 = Spec::parse(
            "2026-01-24-012-ghi.1",
            r#"---
status: pending
---
# Member 1
"#,
        )
        .unwrap();

        // Create second member (initially pending)
        let member2 = Spec::parse(
            "2026-01-24-012-ghi.2",
            r#"---
status: pending
---
# Member 2
"#,
        )
        .unwrap();

        // Step 1: First member starts - should mark driver as in_progress
        mark_driver_in_progress(specs_dir, "2026-01-24-012-ghi.1").unwrap();

        let updated_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(
            updated_driver.frontmatter.status,
            SpecStatus::InProgress,
            "Driver should be in_progress after first member starts"
        );

        // Step 2: First member completes - driver should NOT complete yet
        let member1_completed = Spec::parse(
            "2026-01-24-012-ghi.1",
            r#"---
status: completed
---
# Member 1
"#,
        )
        .unwrap();

        let all_specs = vec![
            updated_driver.clone(),
            member1_completed.clone(),
            member2.clone(),
        ];
        let result =
            auto_complete_driver_if_ready("2026-01-24-012-ghi.1", &all_specs, specs_dir).unwrap();
        assert!(
            !result,
            "Driver should NOT auto-complete when first member is done but second is pending"
        );

        let still_in_progress = Spec::load(&driver_path).unwrap();
        assert_eq!(
            still_in_progress.frontmatter.status,
            SpecStatus::InProgress,
            "Driver should still be in_progress"
        );

        // Step 3: Second member completes - driver SHOULD auto-complete
        let member2_completed = Spec::parse(
            "2026-01-24-012-ghi.2",
            r#"---
status: completed
---
# Member 2
"#,
        )
        .unwrap();

        let all_specs = vec![
            still_in_progress.clone(),
            member1_completed.clone(),
            member2_completed.clone(),
        ];
        let result =
            auto_complete_driver_if_ready("2026-01-24-012-ghi.2", &all_specs, specs_dir).unwrap();
        assert!(
            result,
            "Driver should auto-complete when all members are completed"
        );

        let final_driver = Spec::load(&driver_path).unwrap();
        assert_eq!(
            final_driver.frontmatter.status,
            SpecStatus::Completed,
            "Driver should be completed after all members complete"
        );
        assert_eq!(
            final_driver.frontmatter.model,
            Some("auto-completed".to_string()),
            "Driver should have auto-completed model"
        );
        assert!(
            final_driver.frontmatter.completed_at.is_some(),
            "Driver should have completed_at timestamp"
        );
    }
}
