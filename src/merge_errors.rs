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
        assert!(msg.contains("001-abc"));
        assert!(msg.contains("chant/001-abc"));
        assert!(msg.contains("main"));
        assert!(msg.contains("Next Steps"));
        assert!(msg.contains("chant merge 001-abc --no-ff"));
        assert!(msg.contains("chant merge 001-abc --rebase"));
    }

    #[test]
    fn test_merge_conflict_contains_recovery_steps() {
        let msg = merge_conflict("001-abc", "chant/001-abc", "main");
        assert!(msg.contains("Merge conflicts detected"));
        assert!(msg.contains("chant merge 001-abc --rebase --auto"));
        assert!(msg.contains("git merge --no-ff chant/001-abc"));
        assert!(msg.contains("Documentation"));
    }

    #[test]
    fn test_branch_not_found_contains_search_steps() {
        let msg = branch_not_found("001-abc", "chant/001-abc");
        assert!(msg.contains("not found"));
        assert!(msg.contains("git branch --list"));
        assert!(msg.contains("chant work 001-abc"));
    }

    #[test]
    fn test_main_branch_not_found() {
        let msg = main_branch_not_found("main");
        assert!(msg.contains("'main' does not exist"));
        assert!(msg.contains("git branch -a"));
        assert!(msg.contains(".chant/config.md"));
    }

    #[test]
    fn test_spec_status_not_mergeable() {
        let msg = spec_status_not_mergeable("001-abc", "Failed");
        assert!(msg.contains("Cannot merge spec 001-abc"));
        assert!(msg.contains("Failed"));
        assert!(msg.contains("chant show 001-abc"));
        assert!(msg.contains("chant finalize 001-abc"));
    }

    #[test]
    fn test_no_branch_for_spec() {
        let msg = no_branch_for_spec("001-abc");
        assert!(msg.contains("No branch found"));
        assert!(msg.contains("001-abc"));
        assert!(msg.contains("git log --oneline --grep"));
    }

    #[test]
    fn test_worktree_already_exists() {
        let msg = worktree_already_exists("001-abc", "/tmp/chant-001-abc", "chant/001-abc");
        assert!(msg.contains("Worktree already exists"));
        assert!(msg.contains("/tmp/chant-001-abc"));
        assert!(msg.contains("git worktree remove"));
        assert!(msg.contains("chant cleanup"));
    }

    #[test]
    fn test_no_commits_found() {
        let msg = no_commits_found("001-abc", "chant/001-abc");
        assert!(msg.contains("No commits found"));
        assert!(msg.contains("chant(001-abc):"));
        assert!(msg.contains("git log chant/001-abc"));
        assert!(msg.contains("--allow-no-commits"));
    }

    #[test]
    fn test_driver_members_incomplete() {
        let incomplete = vec![
            "driver.1 (status: Pending)".to_string(),
            "driver.2 (branch not found)".to_string(),
        ];
        let msg = driver_members_incomplete("driver", &incomplete);
        assert!(msg.contains("Cannot merge driver spec"));
        assert!(msg.contains("driver.1"));
        assert!(msg.contains("driver.2"));
        assert!(msg.contains("chant merge driver"));
    }

    #[test]
    fn test_member_merge_failed() {
        let msg = member_merge_failed("driver", "driver.1", "Merge conflicts detected");
        assert!(msg.contains("Member spec merge failed"));
        assert!(msg.contains("driver"));
        assert!(msg.contains("driver.1"));
        assert!(msg.contains("chant merge driver.1"));
        assert!(msg.contains("chant merge driver"));
    }

    #[test]
    fn test_generic_merge_failed() {
        let msg = generic_merge_failed("001-abc", "chant/001-abc", "main", "some error");
        assert!(msg.contains("Merge failed for spec 001-abc"));
        assert!(msg.contains("chant merge 001-abc --rebase"));
        assert!(msg.contains("git merge --no-ff chant/001-abc"));
    }

    #[test]
    fn test_rebase_conflict() {
        let files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        let msg = rebase_conflict("001-abc", "chant/001-abc", &files);
        assert!(msg.contains("Rebase conflict"));
        assert!(msg.contains("src/main.rs"));
        assert!(msg.contains("src/lib.rs"));
        assert!(msg.contains("chant merge 001-abc --rebase --auto"));
    }

    #[test]
    fn test_merge_stopped() {
        let msg = merge_stopped("001-abc");
        assert!(msg.contains("Merge stopped at spec 001-abc"));
        assert!(msg.contains("--continue-on-error"));
    }

    #[test]
    fn test_rebase_stopped() {
        let msg = rebase_stopped("001-abc");
        assert!(msg.contains("rebase conflict"));
        assert!(msg.contains("--rebase --auto"));
    }
}
