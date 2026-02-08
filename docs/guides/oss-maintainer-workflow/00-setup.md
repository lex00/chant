# Phase 0: Setup

Before starting the investigation, you configure chant for working on a shared open source repository. The key decision: keep your specs local so they don't clutter the project's git history.

## Silent Mode

```bash
$ chant init
Initialized .chant/ directory

$ chant silent
Silent mode enabled. .chant/ added to .git/info/exclude.
```

Silent mode adds `.chant/` to your local git exclude file. Your specs, research documents, and execution logs stay on your machine. When you eventually submit a pull request, it contains only the code changes -- no `.chant/` artifacts.

This matters because your investigation trail is personal workflow. The upstream project doesn't need to see your hypothesis elimination table or your three failed reproduction attempts. They need a clean fix with tests.

## What Silent Mode Does

1. Adds `.chant/` to `.git/info/exclude` (local-only gitignore)
2. Suppresses warnings about untracked spec files
3. All chant functionality works normally
4. Nothing from `.chant/` appears in `git status` or commits

## When Not to Use Silent Mode

If you're the sole maintainer and want specs tracked in the repository, or if your team has agreed to share specs, skip `chant silent`. The default behavior tracks specs in git, which is useful for team workflows where investigation history should be shared.

## Configuration

Your `.chant/config.md` for this workflow:

```yaml
defaults:
  silent: true
  main_branch: "main"
```

If you're working on a fix branch instead of main, set `main_branch` to your branch name so worktree merges land in the right place.

With setup complete, you're ready to start investigating issue #1234.

**Next:** [Comprehension](01-comprehension.md)
