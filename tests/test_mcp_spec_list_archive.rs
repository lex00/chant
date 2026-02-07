mod support;
use support::{factory::SpecFactory, harness::TestHarness};

use chant::spec::load_all_specs;
use std::fs;

#[test]
fn test_load_all_specs_excludes_archive() {
    let harness = TestHarness::new();

    let archive_dir = harness.path().join(".chant/archive");
    fs::create_dir_all(&archive_dir).unwrap();

    // Create a spec in specs directory
    harness.create_spec(
        "2026-02-01-001-abc",
        &SpecFactory::as_markdown("2026-02-01-001-abc", "pending"),
    );

    // Create a spec in archive directory
    let spec_in_archive = SpecFactory::as_markdown("2026-01-01-001-xyz", "completed");
    fs::write(archive_dir.join("2026-01-01-001-xyz.md"), spec_in_archive).unwrap();

    // Load specs from specs_dir only
    let specs = load_all_specs(&harness.specs_dir).unwrap();

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
