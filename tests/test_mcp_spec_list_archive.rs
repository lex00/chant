use chant::spec::load_all_specs;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_all_specs_excludes_archive() {
    // Create temp directory structure
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    let specs_dir = base_path.join(".chant/specs");
    let archive_dir = base_path.join(".chant/archive");

    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&archive_dir).unwrap();

    // Create a spec in specs directory
    let spec_in_specs = r#"---
id: 2026-02-01-001-abc
status: pending
---

# Active Spec

This is an active spec.
"#;
    fs::write(specs_dir.join("2026-02-01-001-abc.md"), spec_in_specs).unwrap();

    // Create a spec in archive directory
    let spec_in_archive = r#"---
id: 2026-01-01-001-xyz
status: completed
---

# Archived Spec

This is an archived spec.
"#;
    fs::write(archive_dir.join("2026-01-01-001-xyz.md"), spec_in_archive).unwrap();

    // Load specs from specs_dir only
    let specs = load_all_specs(&specs_dir).unwrap();

    // Verify only the active spec is loaded
    assert_eq!(specs.len(), 1, "Expected 1 spec, but got {}", specs.len());
    assert_eq!(specs[0].id, "2026-02-01-001-abc");

    // Verify the archived spec is NOT included
    let archived_spec_found = specs.iter().any(|s| s.id == "2026-01-01-001-xyz");
    assert!(
        !archived_spec_found,
        "Archived spec should not be included in spec list"
    );
}
