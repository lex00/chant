//! CLI entry point and command handlers for chant.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/cli.md
//! - ignore: false

// Internal modules not exposed via library
mod cli;
mod cmd;
mod render;
mod templates;

use anyhow::Result;
use std::path::Path;

use cli::{Cli, Commands, SiteCommands, TemplateCommands, WorktreeCommands};
use cmd::dispatch::Execute;

fn main() -> Result<()> {
    // Spawn the real work on a thread with a larger stack size.
    // Windows defaults to a 1MB stack which is insufficient for this binary
    // in debug builds (Linux/macOS default to 8MB). Using 8MB here matches
    // the Linux default and prevents stack overflows on Windows CI.
    const STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB

    let thread = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(run)
        .expect("failed to spawn main thread");

    match thread.join() {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn run() -> Result<()> {
    use clap::Parser;
    let cli = Cli::parse();

    // Set quiet mode globally if --quiet flag is present
    if cli.quiet {
        std::env::set_var("CHANT_QUIET", "1");
    }

    cli.command.execute()
}

impl cmd::dispatch::Execute for Commands {
    fn execute(self) -> Result<()> {
        match self {
            Commands::Init {
                subcommand,
                name,
                silent,
                force_overwrite,
                minimal,
                agent,
                provider,
                model,
                merge_driver,
            } => cmd::init::run(
                subcommand.as_deref(),
                name,
                silent,
                force_overwrite,
                minimal,
                agent,
                provider,
                model,
                merge_driver,
            ),
            Commands::Add {
                description,
                prompt,
                needs_approval,
                template,
                vars,
            } => {
                if let Some(template_name) = template {
                    cmd::template::cmd_add_from_template(
                        &template_name,
                        &vars,
                        prompt.as_deref(),
                        needs_approval,
                    )
                } else {
                    if description.is_empty() {
                        anyhow::bail!(
                            "Description is required when not using --template.\n\n\
                         Usage:\n  \
                         chant add \"description of work\"\n  \
                         chant add --template <name> [--var key=value...]"
                        );
                    }
                    cmd::spec::cmd_add(&description, prompt.as_deref(), needs_approval)
                }
            }
            Commands::Template { command } => match command {
                TemplateCommands::List => cmd::template::cmd_template_list(),
                TemplateCommands::Show { name } => cmd::template::cmd_template_show(&name),
            },
            Commands::Approve { id, by } => cmd::spec::cmd_approve(&id, &by),
            Commands::Reject { id, by, reason } => cmd::spec::cmd_reject(&id, &by, &reason),
            Commands::List {
                ready,
                label,
                r#type,
                status,
                global,
                repo,
                project,
                approval,
                created_by,
                activity_since,
                mentions,
                count,
                main_only,
                summary,
                watch,
                brief,
                json,
            } => {
                if summary {
                    cmd::spec::cmd_status(global, repo.as_deref(), watch, brief, json)
                } else {
                    // Check that watch, brief, json are not used without --summary
                    if watch || brief || json {
                        anyhow::bail!("Error: --watch, --brief, and --json require --summary flag.\n\nUsage: chant list --summary [--watch] [--brief | --json]");
                    }
                    cmd::spec::cmd_list(
                        ready,
                        &label,
                        r#type.as_deref(),
                        status.as_deref(),
                        global,
                        repo.as_deref(),
                        project.as_deref(),
                        approval.as_deref(),
                        created_by.as_deref(),
                        activity_since.as_deref(),
                        mentions.as_deref(),
                        count,
                        main_only,
                    )
                }
            }
            Commands::Show {
                id,
                body,
                no_render,
                raw,
                clean,
            } => cmd::spec::cmd_show(&id, body, no_render, raw, clean),
            Commands::Edit { id } => cmd::spec::cmd_edit(&id),
            Commands::Search {
                query,
                title_only,
                body_only,
                case_sensitive,
                status,
                type_,
                label,
                since,
                until,
                active_only,
                archived_only,
                global,
                repo,
            } => {
                let opts = cmd::search::build_search_options(
                    query,
                    title_only,
                    body_only,
                    case_sensitive,
                    status,
                    type_,
                    label,
                    since,
                    until,
                    active_only,
                    archived_only,
                    global,
                    repo.as_deref(),
                )?;
                cmd::search::cmd_search(opts)
            }
            Commands::Work {
                ids,
                prompt,
                skip_deps,
                skip_criteria,
                parallel,
                label,
                finalize,
                allow_no_commits,
                max_parallel,
                no_cleanup,
                cleanup,
                skip_approval,
                chain,
                chain_max,
                no_merge,
                no_rebase,
                no_watch,
            } => {
                // Handle --parallel flag: if provided, convert to (true, max_workers)
                // If --max is also provided, it takes precedence (for backwards compat)
                let (parallel_flag, effective_max) = if let Some(n) = parallel {
                    // --parallel was provided
                    if n == 0 {
                        // --parallel with no value, use max_parallel or default
                        (true, max_parallel)
                    } else {
                        // --parallel=N was provided
                        (true, Some(n))
                    }
                } else {
                    // --parallel not provided
                    (false, max_parallel)
                };
                cmd::work::cmd_work(
                    &ids,
                    prompt.as_deref(),
                    skip_deps,
                    skip_criteria,
                    parallel_flag,
                    &label,
                    finalize,
                    allow_no_commits,
                    effective_max,
                    no_cleanup,
                    cleanup,
                    skip_approval,
                    chain,
                    chain_max,
                    no_merge,
                    no_rebase,
                    no_watch,
                )
            }
            Commands::Mcp => chant::mcp::run_server(),
            Commands::Status {
                global,
                repo,
                watch,
                brief,
                json,
                disk,
                worktrees,
            } => {
                if worktrees {
                    cmd::worktree::cmd_worktree_status()
                } else if disk {
                    cmd::disk::cmd_disk()
                } else {
                    cmd::spec::cmd_status(global, repo.as_deref(), watch, brief, json)
                }
            }
            Commands::Refresh { verbose } => cmd::refresh::cmd_refresh(verbose),
            Commands::Lint {
                spec_id,
                format,
                verbose,
            } => {
                let lint_format = match format.to_lowercase().as_str() {
                    "json" => cmd::spec::LintFormat::Json,
                    "text" => cmd::spec::LintFormat::Text,
                    _ => {
                        eprintln!("Error: Invalid format '{}'. Use 'text' or 'json'.", format);
                        std::process::exit(1);
                    }
                };
                cmd::spec::cmd_lint(spec_id.as_deref(), lint_format, verbose)
            }
            Commands::Log {
                id,
                lines,
                no_follow,
                run,
            } => cmd::lifecycle::cmd_log(&id, lines, !no_follow, run.as_deref()),
            Commands::Split {
                id,
                model,
                force_status,
                recursive,
                max_depth,
            } => {
                cmd::lifecycle::cmd_split(&id, model.as_deref(), force_status, recursive, max_depth)
            }
            Commands::Archive {
                id,
                dry_run,
                older_than,
                allow_non_completed,
                commit,
                no_commit,
                no_stage,
            } => {
                let should_commit = commit && !no_commit;
                cmd::lifecycle::cmd_archive(
                    id.as_deref(),
                    dry_run,
                    older_than,
                    allow_non_completed,
                    should_commit,
                    no_stage,
                )
            }
            Commands::Merge {
                ids,
                all,
                all_completed,
                list,
                ready,
                interactive,
                dry_run,
                delete_branch,
                continue_on_error,
                yes,
                rebase,
                auto,
                finalize,
            } => cmd::lifecycle::cmd_merge(
                &ids,
                all,
                all_completed,
                list,
                ready,
                interactive,
                dry_run,
                delete_branch,
                continue_on_error,
                yes,
                rebase,
                auto,
                finalize,
            ),
            Commands::Diagnose { id } => cmd::lifecycle::cmd_diagnose(&id),
            Commands::Drift { id } => cmd::lifecycle::cmd_drift(id.as_deref()),
            Commands::Reset {
                id,
                work,
                prompt,
                branch,
            } => cmd::lifecycle::cmd_reset(&id, work, prompt.as_deref(), branch),
            Commands::Resume {
                id,
                work,
                prompt,
                branch,
            } => cmd::lifecycle::cmd_reset(&id, work, prompt.as_deref(), branch),
            Commands::Pause { id, force } => cmd::pause::cmd_pause(&id, force),
            Commands::Takeover { id, force } => {
                use colored::Colorize;
                let result = cmd::takeover::cmd_takeover(&id, force)?;
                println!("\n{}", "Analysis:".bold());
                println!("{}", result.analysis);
                println!("\n{}", "Suggestion:".bold());
                println!("{}", result.suggestion);
                Ok(())
            }
            Commands::Cancel {
                id,
                skip_checks,
                delete,
                cascade,
                delete_branch,
                dry_run,
                yes,
            } => {
                if delete {
                    cmd::spec::cmd_delete(&id, skip_checks, cascade, delete_branch, dry_run, yes)
                } else {
                    if cascade {
                        anyhow::bail!("--cascade can only be used with --delete");
                    }
                    if delete_branch {
                        anyhow::bail!("--delete-branch can only be used with --delete");
                    }
                    cmd::spec::cmd_cancel(&id, skip_checks, dry_run, yes)
                }
            }
            Commands::Config { validate } => {
                if validate {
                    cmd::config::cmd_config_validate()
                } else {
                    println!("Usage: chant config --validate");
                    Ok(())
                }
            }
            Commands::Silent {
                global,
                off,
                status,
            } => cmd::silent::cmd_silent(global, off, status),
            Commands::Version { verbose } => cmd::util::cmd_version(verbose),
            Commands::Export {
                format,
                status,
                type_,
                label,
                ready,
                from,
                to,
                fields,
                output,
            } => cmd::spec::cmd_export(
                format.as_deref(),
                &status,
                type_.as_deref(),
                &label,
                ready,
                from.as_deref(),
                to.as_deref(),
                fields.as_deref(),
                output.as_deref(),
            ),
            Commands::Activity { by, since, spec } => {
                cmd::activity::cmd_activity(by.as_deref(), since.as_deref(), spec.as_deref())
            }
            Commands::Cleanup {
                dry_run,
                yes,
                worktrees,
            } => cmd::cleanup::cmd_cleanup(dry_run, yes, worktrees),
            Commands::Worktree { command } => match command {
                WorktreeCommands::Status => cmd::worktree::cmd_worktree_status(),
            },
            Commands::Verify {
                id,
                all,
                label,
                exit_code,
                dry_run,
                prompt,
            } => cmd::verify::cmd_verify(
                id.as_deref(),
                all,
                &label,
                exit_code,
                dry_run,
                prompt.as_deref(),
            ),
            Commands::Prep { id, clean } => {
                let specs_dir = cmd::ensure_initialized()?;
                cmd::prep::cmd_prep(&id, clean, &specs_dir)
            }
            Commands::Derive { id, all, dry_run } => cmd::derive::cmd_derive(id, all, dry_run),
            Commands::Finalize { id } => {
                let specs_dir = cmd::ensure_initialized()?;
                cmd::lifecycle::cmd_finalize(&id, &specs_dir)
            }
            Commands::MergeDriver {
                base,
                current,
                other,
            } => cmd_merge_driver(&base, &current, &other),
            Commands::Completion { shell } => cmd::util::cmd_completion(shell),
            Commands::Site { command } => match command {
                SiteCommands::Init { force_overwrite } => cmd::site::cmd_site_init(force_overwrite),
                SiteCommands::Build { output } => cmd::site::cmd_site_build(output.as_deref()),
                SiteCommands::Serve { port, output } => {
                    cmd::site::cmd_site_serve(port, output.as_deref())
                }
            },
            Commands::Watch {
                once,
                dry_run,
                poll_interval,
            } => cmd::watch::run_watch(once, dry_run, poll_interval),
            Commands::Dag {
                detail,
                status,
                label,
                type_,
            } => cmd::spec::cmd_dag(&detail, status.as_deref(), &label, type_.as_deref()),
            Commands::Man { out_dir } => cmd::util::cmd_man(out_dir.as_ref()),
        }
    }
}

/// Run the git merge driver for spec files
fn cmd_merge_driver(base: &Path, current: &Path, other: &Path) -> Result<()> {
    match chant::merge_driver::run_merge_driver(base, current, other) {
        Ok(true) => {
            // Clean merge
            std::process::exit(0);
        }
        Ok(false) => {
            // Merge with conflicts
            eprintln!("Spec merge completed with conflicts - manual resolution needed for body");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Merge driver error: {}", e);
            std::process::exit(2);
        }
    }
}
