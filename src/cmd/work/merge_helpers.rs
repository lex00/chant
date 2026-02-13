//! Shared merge and finalize operations for parallel work execution

use anyhow::Result;
use std::path::Path;

use chant::config::Config;
use chant::operations::get_commits_for_spec;
use chant::repository::spec_repository::FileSpecRepository;
use chant::spec::{self, SpecStatus};
use chant::worktree;

use crate::cmd::finalize::finalize_spec;

/// Result of a merge-and-finalize operation
#[derive(Debug)]
pub struct MergeAndFinalizeResult {
    pub merged: bool,
    pub finalized: bool,
    pub has_conflict: bool,
    pub error: Option<String>,
}

/// Result of a finalize-only operation
#[derive(Debug)]
pub struct FinalizeResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Merge a branch to main and optionally finalize the spec
///
/// This function encapsulates the common merge-then-finalize flow used across
/// parallel execution modes. It:
/// 1. Merges the branch to main
/// 2. If successful and `should_finalize` is true, finalizes the spec
/// 3. Sets spec status to `failed` on any error
/// 4. Returns structured result for caller handling
pub fn merge_and_finalize(
    spec_id: &str,
    branch: &str,
    specs_dir: &Path,
    config: &Config,
    no_rebase: bool,
    should_finalize: bool,
) -> Result<MergeAndFinalizeResult> {
    let merge_result = worktree::merge_and_cleanup(branch, &config.defaults.main_branch, no_rebase);

    if !merge_result.success {
        // Merge failed - mark spec as failed
        let spec_path = specs_dir.join(format!("{}.md", spec_id));
        if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
            let _ = spec::TransitionBuilder::new(&mut spec)
                .force()
                .to(SpecStatus::Failed);
            let _ = spec.save(&spec_path);
        }

        return Ok(MergeAndFinalizeResult {
            merged: false,
            finalized: false,
            has_conflict: merge_result.has_conflict,
            error: merge_result.error,
        });
    }

    // Merge succeeded - finalize if requested
    if !should_finalize {
        return Ok(MergeAndFinalizeResult {
            merged: true,
            finalized: false,
            has_conflict: false,
            error: None,
        });
    }

    let finalize_result = if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
        let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
        let commits = get_commits_for_spec(spec_id).ok();
        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
        finalize_spec(&mut spec, &spec_repo, config, &all_specs, false, commits)
    } else {
        Err(anyhow::anyhow!("Failed to load spec for finalization"))
    };

    match finalize_result {
        Ok(()) => Ok(MergeAndFinalizeResult {
            merged: true,
            finalized: true,
            has_conflict: false,
            error: None,
        }),
        Err(e) => {
            // Finalization failed - mark spec as failed
            let spec_path = specs_dir.join(format!("{}.md", spec_id));
            if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
                let _ = spec::TransitionBuilder::new(&mut spec)
                    .force()
                    .to(SpecStatus::Failed);
                let _ = spec.save(&spec_path);
            }

            Ok(MergeAndFinalizeResult {
                merged: true,
                finalized: false,
                has_conflict: false,
                error: Some(format!("Finalization failed: {}", e)),
            })
        }
    }
}

/// Finalize a spec after merge and handle failures
///
/// This is a simpler helper for cases where merge has already been done separately.
/// It finalizes the spec and marks it as failed if finalization fails.
pub fn finalize_after_merge(
    spec_id: &str,
    specs_dir: &Path,
    config: &Config,
) -> Result<FinalizeResult> {
    let finalize_result = if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
        let all_specs = spec::load_all_specs(specs_dir).unwrap_or_default();
        let commits = get_commits_for_spec(spec_id).ok();
        let spec_repo = FileSpecRepository::new(specs_dir.to_path_buf());
        finalize_spec(&mut spec, &spec_repo, config, &all_specs, false, commits)
    } else {
        Err(anyhow::anyhow!("Failed to load spec for finalization"))
    };

    match finalize_result {
        Ok(()) => Ok(FinalizeResult {
            success: true,
            error: None,
        }),
        Err(e) => {
            // Finalization failed - mark spec as failed
            let spec_path = specs_dir.join(format!("{}.md", spec_id));
            if let Ok(mut spec) = spec::resolve_spec(specs_dir, spec_id) {
                let _ = spec::TransitionBuilder::new(&mut spec)
                    .force()
                    .to(SpecStatus::Failed);
                let _ = spec.save(&spec_path);
            }

            Ok(FinalizeResult {
                success: false,
                error: Some(format!("Finalization failed: {}", e)),
            })
        }
    }
}
