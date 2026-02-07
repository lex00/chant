//! Test full spec lifecycle: create → pending → in_progress → completed → archived

use chant::spec::{Spec, SpecStatus};
use std::fs;
use std::path::Path;
use std::process::Command;

mod support;
use support::harness::TestHarness;

#[test]
fn test_full_spec_lifecycle() {
    let harness = TestHarness::new();
    let repo_dir = harness.path();

    let original_dir = std::env::current_dir().expect("Failed to get cwd");

    // Initialize chant
    let init_output = harness
        .run(&["init", "--minimal"])
        .expect("Failed to run chant init");
    if !init_output.status.success() {
        let _ = std::env::set_current_dir(&original_dir);
        panic!(
            "Chant init failed: {}",
            String::from_utf8_lossy(&init_output.stderr)
        );
    }

    // Step 1: Create a spec using `chant add`
    let add_output = harness
        .run(&["add", "Test lifecycle spec"])
        .expect("Failed to run chant add");

    assert!(
        add_output.status.success(),
        "chant add should succeed. stderr: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Find the created spec
    let specs_dir = repo_dir.join(".chant/specs");
    let spec_files: Vec<_> = fs::read_dir(&specs_dir)
        .expect("Failed to read specs dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    assert!(!spec_files.is_empty(), "No spec file was created");
    let spec_path = spec_files[0].path();
    let spec_id = spec_path.file_stem().unwrap().to_str().unwrap().to_string();

    // Step 2: Verify spec is in pending status
    let spec = Spec::load(&spec_path).expect("Failed to load spec");
    assert_eq!(
        spec.frontmatter.status,
        SpecStatus::Pending,
        "New spec should be pending"
    );

    // Step 3: Transition to in_progress
    // Create a spec update that marks it in_progress
    let mut spec_content = fs::read_to_string(&spec_path).expect("Failed to read spec");
    spec_content = spec_content.replace("status: pending", "status: in_progress");
    fs::write(&spec_path, spec_content).expect("Failed to write spec");

    // Commit the status change
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add");
    Command::new("git")
        .args(["commit", "-m", "Start work on spec"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit");

    // Verify in_progress status
    let spec = Spec::load(&spec_path).expect("Failed to reload spec");
    assert_eq!(
        spec.frontmatter.status,
        SpecStatus::InProgress,
        "Spec should be in_progress"
    );

    // Step 4: Simulate work completion and transition to completed
    // Create a branch with work
    let branch = format!("chant/{}", spec_id);
    Command::new("git")
        .args(["checkout", "-b", &branch])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to create branch");

    // Make changes representing work done
    fs::write(repo_dir.join("feature_impl.txt"), "Implementation complete")
        .expect("Failed to write feature file");
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to add feature");
    Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("chant({}): Implement feature", spec_id),
        ])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to commit feature");

    // Go back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(&repo_dir)
        .output()
        .expect("Failed to checkout main");

    // Merge the branch to bring commits to main
    let merge_output = harness
        .run(&["merge", &spec_id, "--delete-branch", "--finalize"])
        .expect("Failed to run merge");

    assert!(
        merge_output.status.success(),
        "Merge should succeed. stderr: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );

    // Verify completed status (merge with --finalize should mark it completed)
    let spec = Spec::load(&spec_path).expect("Failed to reload after merge");
    assert_eq!(
        spec.frontmatter.status,
        SpecStatus::Completed,
        "Spec should be completed after merge --finalize"
    );
    assert!(
        spec.frontmatter.completed_at.is_some(),
        "Spec should have completed_at timestamp"
    );

    // Step 5: Archive the completed spec
    let archive_output = harness
        .run(&["archive", &spec_id])
        .expect("Failed to run archive");

    assert!(
        archive_output.status.success(),
        "Archive should succeed. stderr: {}",
        String::from_utf8_lossy(&archive_output.stderr)
    );

    // Verify spec was moved to archive
    assert!(
        !spec_path.exists(),
        "Spec should be removed from specs directory"
    );

    // Archive directory structure: .chant/archive/YYYY-MM-DD/spec-id.md
    // Extract date prefix from spec_id (format: YYYY-MM-DD-...)
    let date_prefix = spec_id.split('-').take(3).collect::<Vec<_>>().join("-");
    let archive_path = repo_dir
        .join(".chant/archive")
        .join(date_prefix)
        .join(format!("{}.md", spec_id));

    assert!(
        archive_path.exists(),
        "Spec should exist in archive at {:?}",
        archive_path
    );

    // Verify archived spec still has completed status
    let archived_spec = Spec::load(&archive_path).expect("Failed to load archived spec");
    assert_eq!(
        archived_spec.frontmatter.status,
        SpecStatus::Completed,
        "Archived spec should still be completed"
    );

    // Cleanup
    let _ = std::env::set_current_dir(&original_dir);
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(&repo_dir)
        .output();
    // TempDir auto-cleans
}
