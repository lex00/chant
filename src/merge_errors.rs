//! Actionable error messages for merge operations.
//!
//! Provides structured error messages with context, diagnosis,
//! and concrete next steps to help users recover from merge failures.

/// Format a fast-forward merge failure with actionable next steps.
///
/// Used when branches have diverged and a fast-forward-only merge cannot proceed.
pub fn fast_forward_conflict(
    spec_id: &str,
    spec_branch: &str,
    main_branch: &str,
    stderr: &str,
) -> String {
    format!(
        "Error: Cannot fast-forward merge for spec {}\n\n\
         Context:\n\
         \x20 - Branch: {}\n\
         \x20 - Target: {}\n\
         \x20 - Branches have diverged from common ancestor\n\
         \x20 - Git output: {}\n\n\
         Next Steps:\n\
         \x20 1. Use no-fast-forward merge:  chant merge {} --no-ff\n\
         \x20 2. Or rebase onto {}:  chant merge {} --rebase\n\
         \x20 3. Or merge manually:  git merge --no-ff {}\n\
         \x20 4. Debug divergence:  git log {} --oneline -5\n\n\
         Tip: Use 'chant merge --help' for all available options",
        spec_id,
        spec_branch,
        main_branch,
        stderr.trim(),
        spec_id,
        main_branch,
        spec_id,
        spec_branch,
        spec_branch
    )
}

/// Format a merge conflict error with recovery steps.
///
/// Used when git detects actual content conflicts during merge.
pub fn merge_conflict(spec_id: &str, spec_branch: &str, main_branch: &str) -> String {
    format!(
        "Error: Merge conflicts detected for spec {}\n\n\
         Context:\n\
         \x20 - Branch: {}\n\
         \x20 - Target: {}\n\
         \x20 - Conflicting changes exist between branches\n\n\
         Diagnosis:\n\
         \x20 - The spec branch and {} have conflicting changes\n\
         \x20 - Merge was aborted to preserve both branches\n\n\
         Next Steps:\n\
         \x20 1. Auto-resolve conflicts:  chant merge {} --rebase --auto\n\
         \x20 2. Rebase first, then merge:  chant merge {} --rebase\n\
         \x20 3. Manual merge:  git merge --no-ff {}\n\
         \x20 4. Inspect conflicts:  git diff {} {}\n\
         \x20 5. View branch history:  git log {} --oneline -5\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id,
        spec_branch,
        main_branch,
        main_branch,
        spec_id,
        spec_id,
        spec_branch,
        main_branch,
        spec_branch,
        spec_branch
    )
}

/// Format a spec branch not found error.
pub fn branch_not_found(spec_id: &str, spec_branch: &str) -> String {
    format!(
        "Error: Spec branch '{}' not found for spec {}\n\n\
         Context:\n\
         \x20 - Expected branch: {}\n\
         \x20 - The branch may have been deleted or never created\n\n\
         Diagnosis:\n\
         \x20 - Check if the spec was worked in branch mode\n\
         \x20 - The branch may have been cleaned up after a previous merge\n\n\
         Next Steps:\n\
         \x20 1. List all chant branches:  git branch --list 'chant/*'\n\
         \x20 2. Check worktree status:  git worktree list\n\
         \x20 3. If branch existed, check reflog:  git reflog --all\n\
         \x20 4. If work was lost, re-execute:  chant work {}\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_branch, spec_id, spec_branch, spec_id
    )
}

/// Format a main branch not found error.
pub fn main_branch_not_found(main_branch: &str) -> String {
    format!(
        "Error: Main branch '{}' does not exist\n\n\
         Context:\n\
         \x20 - Expected main branch: {}\n\
         \x20 - This is typically 'main' or 'master'\n\n\
         Diagnosis:\n\
         \x20 - The repository may use a different default branch name\n\n\
         Next Steps:\n\
         \x20 1. Check available branches:  git branch -a\n\
         \x20 2. Check remote default:  git remote show origin | grep 'HEAD branch'\n\
         \x20 3. If using a different name, configure it in .chant/config.md\n\n\
         Documentation: See 'chant merge --help' for more options",
        main_branch, main_branch
    )
}

/// Format a failed spec merge error with status context.
pub fn spec_status_not_mergeable(spec_id: &str, status: &str) -> String {
    format!(
        "Error: Cannot merge spec {} (status: {})\n\n\
         Context:\n\
         \x20 - Spec: {}\n\
         \x20 - Current status: {}\n\
         \x20 - Only completed specs can be merged\n\n\
         Next Steps:\n\
         \x20 1. Check spec details:  chant show {}\n\
         \x20 2. If work is done, finalize first:  chant finalize {}\n\
         \x20 3. If needs attention, resolve issues and retry\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id, status, spec_id, status, spec_id, spec_id
    )
}

/// Format a no-branch-found error for a completed spec.
pub fn no_branch_for_spec(spec_id: &str) -> String {
    format!(
        "Error: No branch found for spec {}\n\n\
         Context:\n\
         \x20 - Spec: {}\n\
         \x20 - The spec is completed but has no associated branch\n\n\
         Diagnosis:\n\
         \x20 - The spec may have been worked in direct mode (no separate branch)\n\
         \x20 - The branch may have been deleted after a previous merge\n\n\
         Next Steps:\n\
         \x20 1. Check for existing branches:  git branch --list 'chant/*{}*'\n\
         \x20 2. Check if already merged:  git log --oneline --grep='chant({})'\n\
         \x20 3. If not merged and branch lost, re-execute:  chant work {}\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id, spec_id, spec_id, spec_id, spec_id
    )
}

/// Format a worktree conflict error when a worktree already exists.
pub fn worktree_already_exists(spec_id: &str, worktree_path: &str, branch: &str) -> String {
    format!(
        "Error: Worktree already exists for spec {}\n\n\
         Context:\n\
         \x20 - Worktree path: {}\n\
         \x20 - Branch: {}\n\
         \x20 - A worktree at this path is already in use\n\n\
         Diagnosis:\n\
         \x20 - A previous execution may not have cleaned up properly\n\
         \x20 - The worktree may still be in use by another process\n\n\
         Next Steps:\n\
         \x20 1. Clean up stale worktrees:  chant cleanup --worktrees\n\
         \x20 2. Or remove manually:  git worktree remove {} --force\n\
         \x20 3. List all worktrees:  git worktree list\n\
         \x20 4. Then retry:  chant work {}\n\n\
         Documentation: See 'chant cleanup --help' for more options",
        spec_id, worktree_path, branch, worktree_path, spec_id
    )
}

/// Format a no-commits-found error with branch diagnostic info.
pub fn no_commits_found(spec_id: &str, branch: &str) -> String {
    format!(
        "Error: No commits found matching pattern 'chant({}):'\n\n\
         Context:\n\
         \x20 - Branch: {}\n\
         \x20 - Expected pattern: 'chant({}): <description>'\n\n\
         Diagnosis:\n\
         \x20 - The agent may have forgotten to commit with the correct pattern\n\
         \x20 - Commit messages must include 'chant({}):' prefix\n\n\
         Next Steps:\n\
         \x20 1. Check commits on branch:  git log {} --oneline\n\
         \x20 2. If commits exist but wrong pattern, amend or merge manually\n\
         \x20 3. If no work was done, the branch may be empty\n\
         \x20 4. Use --allow-no-commits as fallback (special cases only)\n\n\
         Debugging: Report this if commits look correct - may be a pattern matching bug\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id, branch, spec_id, spec_id, branch
    )
}

/// Format a driver spec member incomplete error.
pub fn driver_members_incomplete(driver_id: &str, incomplete: &[String]) -> String {
    format!(
        "Error: Cannot merge driver spec {} - members are incomplete\n\n\
         Context:\n\
         \x20 - Driver spec: {}\n\
         \x20 - All member specs must be completed before merging the driver\n\n\
         Incomplete members:\n\
         \x20 - {}\n\n\
         Next Steps:\n\
         \x20 1. Check each incomplete member:  chant show <member-id>\n\
         \x20 2. Complete or cancel pending members\n\
         \x20 3. Retry driver merge:  chant merge {}\n\n\
         Documentation: See 'chant merge --help' for more options",
        driver_id,
        driver_id,
        incomplete.join("\n  - "),
        driver_id
    )
}

/// Format a member spec merge failure within a driver merge.
pub fn member_merge_failed(driver_id: &str, member_id: &str, error: &str) -> String {
    format!(
        "Error: Member spec merge failed, driver merge not attempted\n\n\
         Context:\n\
         \x20 - Driver spec: {}\n\
         \x20 - Failed member: {}\n\
         \x20 - Error: {}\n\n\
         Next Steps:\n\
         \x20 1. Resolve the member merge issue first\n\
         \x20 2. Merge the member manually:  chant merge {}\n\
         \x20 3. Then retry the driver merge:  chant merge {}\n\
         \x20 4. Or use rebase:  chant merge {} --rebase\n\n\
         Documentation: See 'chant merge --help' for more options",
        driver_id, member_id, error, member_id, driver_id, member_id
    )
}

/// Format a generic merge failure in the merge summary with next steps.
pub fn generic_merge_failed(spec_id: &str, branch: &str, main_branch: &str, error: &str) -> String {
    format!(
        "Error: Merge failed for spec {}\n\n\
         Context:\n\
         \x20 - Branch: {}\n\
         \x20 - Target: {}\n\
         \x20 - Error: {}\n\n\
         Next Steps:\n\
         \x20 1. Try with rebase:  chant merge {} --rebase\n\
         \x20 2. Or auto-resolve:  chant merge {} --rebase --auto\n\
         \x20 3. Manual merge:  git merge --no-ff {}\n\
         \x20 4. Debug:  git log {} --oneline -5\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id,
        branch,
        main_branch,
        error.trim(),
        spec_id,
        spec_id,
        branch,
        branch
    )
}

/// Format a rebase conflict error with recovery steps.
pub fn rebase_conflict(spec_id: &str, branch: &str, conflicting_files: &[String]) -> String {
    format!(
        "Error: Rebase conflict for spec {}\n\n\
         Context:\n\
         \x20 - Branch: {}\n\
         \x20 - Conflicting files:\n\
         \x20   - {}\n\n\
         Next Steps:\n\
         \x20 1. Auto-resolve:  chant merge {} --rebase --auto\n\
         \x20 2. Resolve manually, then:  git rebase --continue\n\
         \x20 3. Abort rebase:  git rebase --abort\n\
         \x20 4. Try direct merge instead:  git merge --no-ff {}\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id,
        branch,
        conflicting_files.join("\n    - "),
        spec_id,
        branch
    )
}

/// Format a merge stopped error when --continue-on-error is not set.
pub fn merge_stopped(spec_id: &str) -> String {
    format!(
        "Error: Merge stopped at spec {}\n\n\
         Context:\n\
         \x20 - Processing halted due to merge failure\n\
         \x20 - Remaining specs were not processed\n\n\
         Next Steps:\n\
         \x20 1. Resolve the issue with spec {}:  chant show {}\n\
         \x20 2. Retry with continue-on-error:  chant merge --all --continue-on-error\n\
         \x20 3. Or merge specs individually:  chant merge {}\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id, spec_id, spec_id, spec_id
    )
}

/// Format a rebase stopped error with --auto suggestion.
pub fn rebase_stopped(spec_id: &str) -> String {
    format!(
        "Error: Merge stopped at spec {} due to rebase conflict\n\n\
         Context:\n\
         \x20 - Rebase encountered conflicts\n\
         \x20 - Remaining specs were not processed\n\n\
         Next Steps:\n\
         \x20 1. Auto-resolve conflicts:  chant merge {} --rebase --auto\n\
         \x20 2. Use continue-on-error:  chant merge --all --rebase --continue-on-error\n\
         \x20 3. Resolve manually and retry\n\n\
         Documentation: See 'chant merge --help' for more options",
        spec_id, spec_id
    )
}

/// Conflict type classification for merge operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    /// Fast-forward is not possible - branches have diverged
    FastForward,
    /// Content conflicts - same lines modified in both branches
    Content,
    /// Tree conflicts - file renamed/deleted in one branch, modified in another
    Tree,
    /// Unknown conflict type
    Unknown,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::FastForward => write!(f, "fast-forward"),
            ConflictType::Content => write!(f, "content"),
            ConflictType::Tree => write!(f, "tree"),
            ConflictType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detailed merge conflict error with file list and recovery steps.
///
/// Used when git detects content or tree conflicts during merge.
pub fn merge_conflict_detailed(
    spec_id: &str,
    spec_branch: &str,
    main_branch: &str,
    conflict_type: ConflictType,
    conflicting_files: &[String],
) -> String {
    let conflict_type_str = match conflict_type {
        ConflictType::FastForward => "Cannot fast-forward",
        ConflictType::Content => "Content conflicts detected",
        ConflictType::Tree => "Tree conflicts detected",
        ConflictType::Unknown => "Merge conflicts detected",
    };

    let files_section = if conflicting_files.is_empty() {
        "  (unable to determine conflicting files)".to_string()
    } else {
        conflicting_files
            .iter()
            .map(|f| format!("  - {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let recovery_steps = match conflict_type {
        ConflictType::FastForward => format!(
            "Next steps:\n\
             \x20 1. Use no-fast-forward merge:  chant merge {} --no-ff\n\
             \x20 2. Or rebase onto {}:  chant merge {} --rebase\n\
             \x20 3. Or merge manually:  git merge --no-ff {}",
            spec_id, main_branch, spec_id, spec_branch
        ),
        ConflictType::Content | ConflictType::Tree | ConflictType::Unknown => format!(
            "Next steps:\n\
             \x20 1. Resolve conflicts manually, then:  git merge --continue\n\
             \x20 2. Or try automatic rebase:  chant merge {} --rebase --auto\n\
             \x20 3. Or abort:  git merge --abort\n\n\
             Example (resolve manually):\n\
             \x20 $ git status                    # see conflicting files\n\
             \x20 $ vim src/main.rs               # edit to resolve\n\
             \x20 $ git add src/main.rs           # stage resolved file\n\
             \x20 $ git merge --continue          # complete merge",
            spec_id
        ),
    };

    format!(
        "Error: {} for spec {}\n\n\
         Context:\n\
         \x20 - Branch: {}\n\
         \x20 - Target: {}\n\
         \x20 - Conflict type: {}\n\n\
         Files with conflicts:\n\
         {}\n\n\
         {}\n\n\
         Documentation: See 'chant merge --help' for more options",
        conflict_type_str,
        spec_id,
        spec_branch,
        main_branch,
        conflict_type,
        files_section,
        recovery_steps
    )
}

/// Classify merge conflict type from git output.
///
/// Analyzes git merge stderr and status output to determine the type of conflict.
pub fn classify_conflict_type(stderr: &str, status_output: Option<&str>) -> ConflictType {
    let stderr_lower = stderr.to_lowercase();

    // Check for fast-forward conflicts
    if stderr_lower.contains("not possible to fast-forward")
        || stderr_lower.contains("cannot fast-forward")
        || stderr_lower.contains("refusing to merge unrelated histories")
    {
        return ConflictType::FastForward;
    }

    // Check for tree conflicts (rename/delete conflicts)
    if stderr_lower.contains("conflict (rename/delete)")
        || stderr_lower.contains("conflict (modify/delete)")
        || stderr_lower.contains("deleted in")
        || stderr_lower.contains("renamed in")
        || stderr_lower.contains("conflict (add/add)")
    {
        return ConflictType::Tree;
    }

    // Check git status for conflict markers if available
    if let Some(status) = status_output {
        // Tree conflicts show as DD, AU, UD, UA, DU in status
        if status.lines().any(|line| {
            let prefix = line.get(..2).unwrap_or("");
            matches!(prefix, "DD" | "AU" | "UD" | "UA" | "DU")
        }) {
            return ConflictType::Tree;
        }

        // Content conflicts show as UU or AA in status
        if status.lines().any(|line| {
            let prefix = line.get(..2).unwrap_or("");
            matches!(prefix, "UU" | "AA")
        }) {
            return ConflictType::Content;
        }
    }

    // Check for general merge conflicts
    if stderr_lower.contains("conflict") || stderr_lower.contains("merge conflict") {
        return ConflictType::Content;
    }

    ConflictType::Unknown
}

/// Parse conflicting files from git status --porcelain output.
///
/// Returns a list of files that have conflict markers.
pub fn parse_conflicting_files(status_output: &str) -> Vec<String> {
    let mut files = Vec::new();

    for line in status_output.lines() {
        if line.len() >= 3 {
            let status = &line[0..2];
            // Conflict markers: UU, AA, DD, AU, UD, UA, DU
            if status.contains('U') || status == "AA" || status == "DD" {
                let file = line[3..].trim();
                files.push(file.to_string());
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_forward_conflict_contains_spec_id() {
        let msg = fast_forward_conflict(
            "001-abc",
            "chant/001-abc",
            "main",
            "fatal: cannot fast-forward",
        );
        assert!(msg.contains("001-abc"), "should include spec ID");
        assert!(msg.contains("chant/001-abc"), "should include branch name");
        assert!(msg.contains("main"), "should include target branch");
        assert!(msg.contains("Next Steps"), "should provide next steps");
        assert!(
            msg.contains("chant merge 001-abc --no-ff"),
            "should suggest --no-ff option"
        );
        assert!(
            msg.contains("chant merge 001-abc --rebase"),
            "should suggest --rebase option"
        );
    }

    #[test]
    fn test_merge_conflict_contains_recovery_steps() {
        let msg = merge_conflict("001-abc", "chant/001-abc", "main");
        assert!(
            msg.contains("Merge conflicts detected"),
            "should describe error type"
        );
        assert!(
            msg.contains("chant merge 001-abc --rebase --auto"),
            "should suggest auto-resolve"
        );
        assert!(
            msg.contains("git merge --no-ff chant/001-abc"),
            "should provide manual merge command"
        );
        assert!(
            msg.contains("Documentation"),
            "should reference documentation"
        );
    }

    #[test]
    fn test_branch_not_found_contains_search_steps() {
        let msg = branch_not_found("001-abc", "chant/001-abc");
        assert!(msg.contains("not found"), "should state branch is missing");
        assert!(
            msg.contains("git branch --list"),
            "should suggest listing branches"
        );
        assert!(
            msg.contains("chant work 001-abc"),
            "should suggest re-execution"
        );
    }

    #[test]
    fn test_main_branch_not_found() {
        let msg = main_branch_not_found("main");
        assert!(
            msg.contains("'main' does not exist"),
            "should state main branch is missing"
        );
        assert!(
            msg.contains("git branch -a"),
            "should suggest listing all branches"
        );
        assert!(
            msg.contains(".chant/config.md"),
            "should reference config file"
        );
    }

    #[test]
    fn test_spec_status_not_mergeable() {
        let msg = spec_status_not_mergeable("001-abc", "Failed");
        assert!(
            msg.contains("Cannot merge spec 001-abc"),
            "should state spec cannot be merged"
        );
        assert!(msg.contains("Failed"), "should include current status");
        assert!(
            msg.contains("chant show 001-abc"),
            "should suggest inspecting spec"
        );
        assert!(
            msg.contains("chant finalize 001-abc"),
            "should suggest finalizing spec"
        );
    }

    #[test]
    fn test_no_branch_for_spec() {
        let msg = no_branch_for_spec("001-abc");
        assert!(
            msg.contains("No branch found"),
            "should state no branch exists"
        );
        assert!(msg.contains("001-abc"), "should include spec ID");
        assert!(
            msg.contains("git log --oneline --grep"),
            "should suggest searching commit history"
        );
    }

    #[test]
    fn test_worktree_already_exists() {
        let msg = worktree_already_exists("001-abc", "/tmp/chant-001-abc", "chant/001-abc");
        assert!(
            msg.contains("Worktree already exists"),
            "should describe the conflict"
        );
        assert!(
            msg.contains("/tmp/chant-001-abc"),
            "should include worktree path"
        );
        assert!(
            msg.contains("git worktree remove"),
            "should suggest manual removal"
        );
        assert!(
            msg.contains("chant cleanup"),
            "should suggest cleanup command"
        );
    }

    #[test]
    fn test_no_commits_found() {
        let msg = no_commits_found("001-abc", "chant/001-abc");
        assert!(
            msg.contains("No commits found"),
            "should state no matching commits"
        );
        assert!(
            msg.contains("chant(001-abc):"),
            "should show expected pattern"
        );
        assert!(
            msg.contains("git log chant/001-abc"),
            "should suggest inspecting branch"
        );
        assert!(
            msg.contains("--allow-no-commits"),
            "should mention fallback option"
        );
    }

    #[test]
    fn test_driver_members_incomplete() {
        let incomplete = vec![
            "driver.1 (status: Pending)".to_string(),
            "driver.2 (branch not found)".to_string(),
        ];
        let msg = driver_members_incomplete("driver", &incomplete);
        assert!(
            msg.contains("Cannot merge driver spec"),
            "should state driver cannot be merged"
        );
        assert!(
            msg.contains("driver.1"),
            "should list first incomplete member"
        );
        assert!(
            msg.contains("driver.2"),
            "should list second incomplete member"
        );
        assert!(
            msg.contains("chant merge driver"),
            "should suggest merging driver after members complete"
        );
    }

    #[test]
    fn test_member_merge_failed() {
        let msg = member_merge_failed("driver", "driver.1", "Merge conflicts detected");
        assert!(
            msg.contains("Member spec merge failed"),
            "should describe member failure"
        );
        assert!(msg.contains("driver"), "should include driver spec ID");
        assert!(msg.contains("driver.1"), "should include failed member ID");
        assert!(
            msg.contains("chant merge driver.1"),
            "should suggest merging member first"
        );
        assert!(
            msg.contains("chant merge driver"),
            "should suggest retrying driver after"
        );
    }

    #[test]
    fn test_generic_merge_failed() {
        let msg = generic_merge_failed("001-abc", "chant/001-abc", "main", "some error");
        assert!(
            msg.contains("Merge failed for spec 001-abc"),
            "should state merge failed"
        );
        assert!(
            msg.contains("chant merge 001-abc --rebase"),
            "should suggest rebase option"
        );
        assert!(
            msg.contains("git merge --no-ff chant/001-abc"),
            "should provide manual merge command"
        );
    }

    #[test]
    fn test_rebase_conflict() {
        let files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        let msg = rebase_conflict("001-abc", "chant/001-abc", &files);
        assert!(
            msg.contains("Rebase conflict"),
            "should describe rebase conflict"
        );
        assert!(
            msg.contains("src/main.rs"),
            "should list first conflicting file"
        );
        assert!(
            msg.contains("src/lib.rs"),
            "should list second conflicting file"
        );
        assert!(
            msg.contains("chant merge 001-abc --rebase --auto"),
            "should suggest auto-resolve"
        );
    }

    #[test]
    fn test_merge_stopped() {
        let msg = merge_stopped("001-abc");
        assert!(
            msg.contains("Merge stopped at spec 001-abc"),
            "should identify where merge stopped"
        );
        assert!(
            msg.contains("--continue-on-error"),
            "should suggest continue-on-error flag"
        );
    }

    #[test]
    fn test_rebase_stopped() {
        let msg = rebase_stopped("001-abc");
        assert!(
            msg.contains("rebase conflict"),
            "should describe rebase conflict"
        );
        assert!(
            msg.contains("--rebase --auto"),
            "should suggest auto-resolve flags"
        );
    }

    #[test]
    fn test_conflict_type_display() {
        assert_eq!(format!("{}", ConflictType::FastForward), "fast-forward");
        assert_eq!(format!("{}", ConflictType::Content), "content");
        assert_eq!(format!("{}", ConflictType::Tree), "tree");
        assert_eq!(format!("{}", ConflictType::Unknown), "unknown");
    }

    #[test]
    fn test_classify_conflict_type_fast_forward() {
        let stderr = "fatal: Not possible to fast-forward, aborting.";
        assert_eq!(
            classify_conflict_type(stderr, None),
            ConflictType::FastForward
        );

        let stderr2 = "error: cannot fast-forward";
        assert_eq!(
            classify_conflict_type(stderr2, None),
            ConflictType::FastForward
        );
    }

    #[test]
    fn test_classify_conflict_type_tree() {
        let stderr = "CONFLICT (rename/delete): file.rs renamed in HEAD";
        assert_eq!(classify_conflict_type(stderr, None), ConflictType::Tree);

        let stderr2 = "CONFLICT (modify/delete): file.rs deleted in branch";
        assert_eq!(classify_conflict_type(stderr2, None), ConflictType::Tree);

        // Test via status output
        let status = "DU src/deleted.rs\n";
        assert_eq!(classify_conflict_type("", Some(status)), ConflictType::Tree);
    }

    #[test]
    fn test_classify_conflict_type_content() {
        let stderr = "CONFLICT (content): Merge conflict in file.rs";
        assert_eq!(classify_conflict_type(stderr, None), ConflictType::Content);

        // Test via status output
        let status = "UU src/main.rs\nUU src/lib.rs\n";
        assert_eq!(
            classify_conflict_type("", Some(status)),
            ConflictType::Content
        );
    }

    #[test]
    fn test_classify_conflict_type_unknown() {
        let stderr = "some other error";
        assert_eq!(classify_conflict_type(stderr, None), ConflictType::Unknown);
    }

    #[test]
    fn test_parse_conflicting_files() {
        let status = "UU src/main.rs\nUU src/lib.rs\nM  src/other.rs\n";
        let files = parse_conflicting_files(status);
        assert_eq!(files.len(), 2, "should find exactly 2 conflicting files");
        assert!(
            files.contains(&"src/main.rs".to_string()),
            "should include src/main.rs"
        );
        assert!(
            files.contains(&"src/lib.rs".to_string()),
            "should include src/lib.rs"
        );
    }

    #[test]
    fn test_parse_conflicting_files_tree_conflicts() {
        let status = "DD deleted.rs\nAU added_unmerged.rs\nUD unmerged_deleted.rs\n";
        let files = parse_conflicting_files(status);
        assert_eq!(files.len(), 3, "should find exactly 3 tree conflicts");
        assert!(
            files.contains(&"deleted.rs".to_string()),
            "should include deleted.rs"
        );
        assert!(
            files.contains(&"added_unmerged.rs".to_string()),
            "should include added_unmerged.rs"
        );
        assert!(
            files.contains(&"unmerged_deleted.rs".to_string()),
            "should include unmerged_deleted.rs"
        );
    }

    #[test]
    fn test_merge_conflict_detailed_content() {
        let files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        let msg = merge_conflict_detailed(
            "001-abc",
            "chant/001-abc",
            "main",
            ConflictType::Content,
            &files,
        );

        assert!(
            msg.contains("Content conflicts detected"),
            "should describe conflict type"
        );
        assert!(msg.contains("001-abc"), "should include spec ID");
        assert!(msg.contains("chant/001-abc"), "should include branch name");
        assert!(msg.contains("main"), "should include target branch");
        assert!(
            msg.contains("Conflict type: content"),
            "should label conflict type"
        );
        assert!(
            msg.contains("src/main.rs"),
            "should list first conflicting file"
        );
        assert!(
            msg.contains("src/lib.rs"),
            "should list second conflicting file"
        );
        assert!(
            msg.contains("Next steps:"),
            "should provide next steps section"
        );
        assert!(msg.contains("1."), "should have numbered step 1");
        assert!(msg.contains("2."), "should have numbered step 2");
        assert!(msg.contains("3."), "should have numbered step 3");
        assert!(
            msg.contains("git merge --continue"),
            "should suggest continuing merge"
        );
        assert!(
            msg.contains("chant merge 001-abc --rebase --auto"),
            "should suggest auto-resolve"
        );
        assert!(msg.contains("git merge --abort"), "should suggest aborting");
        assert!(msg.contains("Example"), "should provide example workflow");
    }

    #[test]
    fn test_merge_conflict_detailed_tree() {
        let files = vec!["src/renamed.rs".to_string()];
        let msg = merge_conflict_detailed(
            "001-abc",
            "chant/001-abc",
            "main",
            ConflictType::Tree,
            &files,
        );

        assert!(
            msg.contains("Tree conflicts detected"),
            "should describe tree conflict"
        );
        assert!(
            msg.contains("Conflict type: tree"),
            "should label conflict as tree type"
        );
        assert!(
            msg.contains("src/renamed.rs"),
            "should list conflicting file"
        );
    }

    #[test]
    fn test_merge_conflict_detailed_fast_forward() {
        let files: Vec<String> = vec![];
        let msg = merge_conflict_detailed(
            "001-abc",
            "chant/001-abc",
            "main",
            ConflictType::FastForward,
            &files,
        );

        assert!(
            msg.contains("Cannot fast-forward"),
            "should describe fast-forward failure"
        );
        assert!(
            msg.contains("Conflict type: fast-forward"),
            "should label conflict as fast-forward"
        );
        assert!(
            msg.contains("chant merge 001-abc --no-ff"),
            "should suggest --no-ff option"
        );
        assert!(
            msg.contains("chant merge 001-abc --rebase"),
            "should suggest --rebase option"
        );
    }

    #[test]
    fn test_merge_conflict_detailed_empty_files() {
        let files: Vec<String> = vec![];
        let msg = merge_conflict_detailed(
            "001-abc",
            "chant/001-abc",
            "main",
            ConflictType::Content,
            &files,
        );

        assert!(
            msg.contains("unable to determine conflicting files"),
            "should indicate when files cannot be determined"
        );
    }
}
