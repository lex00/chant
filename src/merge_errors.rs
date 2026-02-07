//! Actionable error messages for merge operations.
//!
//! Provides structured error messages with context, diagnosis,
//! and concrete next steps to help users recover from merge failures.

use std::fmt;

/// Merge error kind - the type of merge failure
#[derive(Debug, Clone, PartialEq)]
pub enum MergeErrorKind {
    FastForwardConflict,
    MergeConflict,
    BranchNotFound,
    MainBranchNotFound,
    SpecStatusNotMergeable,
    NoBranchForSpec,
    WorktreeAlreadyExists,
    NoCommitsFound,
    DriverMembersIncomplete,
    MemberMergeFailed,
    GenericMergeFailed,
    RebaseConflict,
    MergeStopped,
    RebaseStopped,
}

/// Context information for a merge error
#[derive(Debug, Clone, Default)]
pub struct MergeContext {
    pub spec_id: String,
    pub spec_branch: Option<String>,
    pub main_branch: Option<String>,
    pub status: Option<String>,
    pub stderr: Option<String>,
    pub conflicting_files: Vec<String>,
    pub incomplete_members: Vec<String>,
    pub member_id: Option<String>,
    pub error_message: Option<String>,
    pub worktree_path: Option<String>,
    pub driver_id: Option<String>,
}

/// Structured merge error with kind and context
#[derive(Debug, Clone)]
pub struct MergeError {
    pub kind: MergeErrorKind,
    pub context: MergeContext,
}

impl MergeError {
    pub fn new(kind: MergeErrorKind, context: MergeContext) -> Self {
        Self { kind, context }
    }

    pub fn fast_forward_conflict(
        spec_id: &str,
        spec_branch: &str,
        main_branch: &str,
        stderr: &str,
    ) -> Self {
        Self::new(
            MergeErrorKind::FastForwardConflict,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(spec_branch.to_string()),
                main_branch: Some(main_branch.to_string()),
                stderr: Some(stderr.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn merge_conflict(spec_id: &str, spec_branch: &str, main_branch: &str) -> Self {
        Self::new(
            MergeErrorKind::MergeConflict,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(spec_branch.to_string()),
                main_branch: Some(main_branch.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn branch_not_found(spec_id: &str, spec_branch: &str) -> Self {
        Self::new(
            MergeErrorKind::BranchNotFound,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(spec_branch.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn main_branch_not_found(main_branch: &str) -> Self {
        Self::new(
            MergeErrorKind::MainBranchNotFound,
            MergeContext {
                main_branch: Some(main_branch.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn spec_status_not_mergeable(spec_id: &str, status: &str) -> Self {
        Self::new(
            MergeErrorKind::SpecStatusNotMergeable,
            MergeContext {
                spec_id: spec_id.to_string(),
                status: Some(status.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn no_branch_for_spec(spec_id: &str) -> Self {
        Self::new(
            MergeErrorKind::NoBranchForSpec,
            MergeContext {
                spec_id: spec_id.to_string(),
                ..Default::default()
            },
        )
    }

    pub fn worktree_already_exists(spec_id: &str, worktree_path: &str, branch: &str) -> Self {
        Self::new(
            MergeErrorKind::WorktreeAlreadyExists,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(branch.to_string()),
                worktree_path: Some(worktree_path.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn no_commits_found(spec_id: &str, branch: &str) -> Self {
        Self::new(
            MergeErrorKind::NoCommitsFound,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(branch.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn driver_members_incomplete(driver_id: &str, incomplete: &[String]) -> Self {
        Self::new(
            MergeErrorKind::DriverMembersIncomplete,
            MergeContext {
                driver_id: Some(driver_id.to_string()),
                incomplete_members: incomplete.to_vec(),
                ..Default::default()
            },
        )
    }

    pub fn member_merge_failed(driver_id: &str, member_id: &str, error: &str) -> Self {
        Self::new(
            MergeErrorKind::MemberMergeFailed,
            MergeContext {
                driver_id: Some(driver_id.to_string()),
                member_id: Some(member_id.to_string()),
                error_message: Some(error.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn generic_merge_failed(
        spec_id: &str,
        branch: &str,
        main_branch: &str,
        error: &str,
    ) -> Self {
        Self::new(
            MergeErrorKind::GenericMergeFailed,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(branch.to_string()),
                main_branch: Some(main_branch.to_string()),
                error_message: Some(error.to_string()),
                ..Default::default()
            },
        )
    }

    pub fn rebase_conflict(spec_id: &str, branch: &str, conflicting_files: &[String]) -> Self {
        Self::new(
            MergeErrorKind::RebaseConflict,
            MergeContext {
                spec_id: spec_id.to_string(),
                spec_branch: Some(branch.to_string()),
                conflicting_files: conflicting_files.to_vec(),
                ..Default::default()
            },
        )
    }

    pub fn merge_stopped(spec_id: &str) -> Self {
        Self::new(
            MergeErrorKind::MergeStopped,
            MergeContext {
                spec_id: spec_id.to_string(),
                ..Default::default()
            },
        )
    }

    pub fn rebase_stopped(spec_id: &str) -> Self {
        Self::new(
            MergeErrorKind::RebaseStopped,
            MergeContext {
                spec_id: spec_id.to_string(),
                ..Default::default()
            },
        )
    }
}

impl fmt::Display for MergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MergeErrorKind::*;

        match &self.kind {
            FastForwardConflict => {
                let spec_id = &self.context.spec_id;
                let spec_branch = self.context.spec_branch.as_deref().unwrap_or("");
                let main_branch = self.context.main_branch.as_deref().unwrap_or("");
                let stderr = self.context.stderr.as_deref().unwrap_or("").trim();

                write!(
                    f,
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
                    stderr,
                    spec_id,
                    main_branch,
                    spec_id,
                    spec_branch,
                    spec_branch
                )
            }
            MergeConflict => {
                let spec_id = &self.context.spec_id;
                let spec_branch = self.context.spec_branch.as_deref().unwrap_or("");
                let main_branch = self.context.main_branch.as_deref().unwrap_or("");

                write!(
                    f,
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
            BranchNotFound => {
                let spec_id = &self.context.spec_id;
                let spec_branch = self.context.spec_branch.as_deref().unwrap_or("");

                write!(
                    f,
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
            MainBranchNotFound => {
                let main_branch = self.context.main_branch.as_deref().unwrap_or("");

                write!(
                    f,
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
            SpecStatusNotMergeable => {
                let spec_id = &self.context.spec_id;
                let status = self.context.status.as_deref().unwrap_or("");

                write!(
                    f,
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
            NoBranchForSpec => {
                let spec_id = &self.context.spec_id;

                write!(
                    f,
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
            WorktreeAlreadyExists => {
                let spec_id = &self.context.spec_id;
                let worktree_path = self.context.worktree_path.as_deref().unwrap_or("");
                let branch = self.context.spec_branch.as_deref().unwrap_or("");

                write!(
                    f,
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
            NoCommitsFound => {
                let spec_id = &self.context.spec_id;
                let branch = self.context.spec_branch.as_deref().unwrap_or("");

                write!(
                    f,
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
            DriverMembersIncomplete => {
                let driver_id = self.context.driver_id.as_deref().unwrap_or("");
                let incomplete = &self.context.incomplete_members;

                write!(
                    f,
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
            MemberMergeFailed => {
                let driver_id = self.context.driver_id.as_deref().unwrap_or("");
                let member_id = self.context.member_id.as_deref().unwrap_or("");
                let error = self.context.error_message.as_deref().unwrap_or("");

                write!(
                    f,
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
            GenericMergeFailed => {
                let spec_id = &self.context.spec_id;
                let branch = self.context.spec_branch.as_deref().unwrap_or("");
                let main_branch = self.context.main_branch.as_deref().unwrap_or("");
                let error = self.context.error_message.as_deref().unwrap_or("").trim();

                write!(
                    f,
                    "Error: Merge failed for spec {}\n\n\
                     Context:\n\
                     \x20 - Branch: {}\n\
                     \x20 - Target: {}\n\
                     \x20 - Error: {}\n\n\
                     Next Steps:\n\
                     \x20 1. Try with rebase:  chant merge {} --rebase\n\
                     \x20 2. Or auto-resolve:  chant merge {} --rebase --auto\n\
                     \x20 3. Manual merge:  git merge --no-ff {}\n\
                     \x20 4. Debug:  git log {} --online -5\n\n\
                     Documentation: See 'chant merge --help' for more options",
                    spec_id, branch, main_branch, error, spec_id, spec_id, branch, branch
                )
            }
            RebaseConflict => {
                let spec_id = &self.context.spec_id;
                let branch = self.context.spec_branch.as_deref().unwrap_or("");
                let conflicting_files = &self.context.conflicting_files;

                write!(
                    f,
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
            MergeStopped => {
                let spec_id = &self.context.spec_id;

                write!(
                    f,
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
            RebaseStopped => {
                let spec_id = &self.context.spec_id;

                write!(
                    f,
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
        }
    }
}

// Legacy functions for backward compatibility
pub fn fast_forward_conflict(
    spec_id: &str,
    spec_branch: &str,
    main_branch: &str,
    stderr: &str,
) -> String {
    MergeError::fast_forward_conflict(spec_id, spec_branch, main_branch, stderr).to_string()
}

pub fn merge_conflict(spec_id: &str, spec_branch: &str, main_branch: &str) -> String {
    MergeError::merge_conflict(spec_id, spec_branch, main_branch).to_string()
}

pub fn branch_not_found(spec_id: &str, spec_branch: &str) -> String {
    MergeError::branch_not_found(spec_id, spec_branch).to_string()
}

pub fn main_branch_not_found(main_branch: &str) -> String {
    MergeError::main_branch_not_found(main_branch).to_string()
}

pub fn spec_status_not_mergeable(spec_id: &str, status: &str) -> String {
    MergeError::spec_status_not_mergeable(spec_id, status).to_string()
}

pub fn no_branch_for_spec(spec_id: &str) -> String {
    MergeError::no_branch_for_spec(spec_id).to_string()
}

pub fn worktree_already_exists(spec_id: &str, worktree_path: &str, branch: &str) -> String {
    MergeError::worktree_already_exists(spec_id, worktree_path, branch).to_string()
}

pub fn no_commits_found(spec_id: &str, branch: &str) -> String {
    MergeError::no_commits_found(spec_id, branch).to_string()
}

pub fn driver_members_incomplete(driver_id: &str, incomplete: &[String]) -> String {
    MergeError::driver_members_incomplete(driver_id, incomplete).to_string()
}

pub fn member_merge_failed(driver_id: &str, member_id: &str, error: &str) -> String {
    MergeError::member_merge_failed(driver_id, member_id, error).to_string()
}

pub fn generic_merge_failed(spec_id: &str, branch: &str, main_branch: &str, error: &str) -> String {
    MergeError::generic_merge_failed(spec_id, branch, main_branch, error).to_string()
}

pub fn rebase_conflict(spec_id: &str, branch: &str, conflicting_files: &[String]) -> String {
    MergeError::rebase_conflict(spec_id, branch, conflicting_files).to_string()
}

pub fn merge_stopped(spec_id: &str) -> String {
    MergeError::merge_stopped(spec_id).to_string()
}

pub fn rebase_stopped(spec_id: &str) -> String {
    MergeError::rebase_stopped(spec_id).to_string()
}

/// Conflict type classification for merge operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    FastForward,
    Content,
    Tree,
    Unknown,
}

impl fmt::Display for ConflictType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictType::FastForward => write!(f, "fast-forward"),
            ConflictType::Content => write!(f, "content"),
            ConflictType::Tree => write!(f, "tree"),
            ConflictType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detailed merge conflict error with file list and recovery steps.
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
pub fn classify_conflict_type(stderr: &str, status_output: Option<&str>) -> ConflictType {
    let stderr_lower = stderr.to_lowercase();

    if stderr_lower.contains("not possible to fast-forward")
        || stderr_lower.contains("cannot fast-forward")
        || stderr_lower.contains("refusing to merge unrelated histories")
    {
        return ConflictType::FastForward;
    }

    if stderr_lower.contains("conflict (rename/delete)")
        || stderr_lower.contains("conflict (modify/delete)")
        || stderr_lower.contains("deleted in")
        || stderr_lower.contains("renamed in")
        || stderr_lower.contains("conflict (add/add)")
    {
        return ConflictType::Tree;
    }

    if let Some(status) = status_output {
        if status.lines().any(|line| {
            let prefix = line.get(..2).unwrap_or("");
            matches!(prefix, "DD" | "AU" | "UD" | "UA" | "DU")
        }) {
            return ConflictType::Tree;
        }

        if status.lines().any(|line| {
            let prefix = line.get(..2).unwrap_or("");
            matches!(prefix, "UU" | "AA")
        }) {
            return ConflictType::Content;
        }
    }

    if stderr_lower.contains("conflict") || stderr_lower.contains("merge conflict") {
        return ConflictType::Content;
    }

    ConflictType::Unknown
}

/// Parse conflicting files from git status --porcelain output.
pub fn parse_conflicting_files(status_output: &str) -> Vec<String> {
    let mut files = Vec::new();

    for line in status_output.lines() {
        if line.len() >= 3 {
            let status = &line[0..2];
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
    }

    #[test]
    fn test_parse_conflicting_files() {
        let status = "UU src/main.rs\nUU src/lib.rs\nM  src/other.rs\n";
        let files = parse_conflicting_files(status);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"src/lib.rs".to_string()));
    }
}
