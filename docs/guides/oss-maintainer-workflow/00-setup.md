# Setup for OSS Maintainers

## Quick Setup

For open source maintainers who want to use chant on shared repositories while keeping specs personal and local:

```bash
# Initialize chant in your repository
chant init

# Enable silent mode (keeps .chant/ local, not tracked in git)
chant silent
```

That's it. You're ready to use chant for your personal workflow while keeping the repository clean for collaborators.

## Why Silent Mode?

**Silent mode is essential for OSS maintainers** who want to use chant on shared repositories. Here's why:

### Personal Workflow on Shared Repos

Open source repositories are collaborative spaces. Your personal specs—research notes, investigation logs, work-in-progress implementations—shouldn't clutter the team's git history. Silent mode lets you:

- Use chant's full workflow locally
- Keep your specs and research notes private
- Avoid adding `.chant/` to the shared repository
- Maintain your own investigation trail without affecting others

### How It Works

When you enable silent mode:

1. `.chant/` is added to `.git/info/exclude` (git ignores it locally)
2. Spec files remain local to your working copy
3. Warnings about untracked specs are suppressed
4. All chant functionality works normally
5. Nothing from `.chant/` appears in `git status` or commits

**The result:** Your investigation and planning process stays personal while your final pull requests remain clean and focused on the actual changes.

## Complete Setup Example

```bash
# Clone the repository
git clone https://github.com/org/project
cd project

# Initialize chant for your personal workflow
chant init

# Enable silent mode to keep specs local
chant silent

# Verify silent mode is active
chant status
```

Now when you use chant's workflow (comprehension → research → implementation), all your specs and research artifacts stay local. Your eventual pull request will contain only the final code changes, tests, and documentation—no `.chant/` artifacts.

## Configuration Reference

Your `.chant/config.yaml` might look like this:

```yaml
defaults:
  silent: true          # Keep .chant/ local (not tracked in git)
  branch: false         # Work directly on current branch
  main_branch: "main"   # Target for merges (if branch: true)

# Optional: GitHub configuration for fork workflow
github:
  user: your-username
  fork: your-username/project
```

**Key settings for OSS maintainers:**

- **`silent: true`** — Essential for shared repos. Keeps your workflow private.
- **`branch: false`** — Work directly on your fix/feature branch instead of creating worktree branches.
- **`main_branch`** — Usually "main" or "master". Set to your fix branch if working on a specific issue branch.

## When to Use Silent Mode

**Use silent mode when:**
- Contributing to open source projects where you don't control `.gitignore`
- Working on repositories with multiple maintainers
- You want a personal spec workflow without affecting the team
- The repository shouldn't include `.chant/` in version control

**Don't use silent mode when:**
- You're on a team that wants to track specs in git (shared workflow)
- You're the sole maintainer and want specs archived in the repository
- The team has agreed to include `.chant/` in version control

## Global vs. Project Silent Mode

Enable silent mode for a single project:
```bash
cd project
chant silent
```

Enable silent mode globally for all projects:
```bash
chant silent --global
```

The global setting applies to all repositories where you use chant. You can override it per-project if needed.

## Next Steps

Now that you've set up chant for OSS maintenance, learn the complete workflow:

1. **[Comprehension Research](01-comprehension.md)** — Understand what the issue is about
2. **[Reproducibility](02-reproduction.md)** — Create failing tests (auto/assisted)
3. **[Root Cause Research](03-root-cause.md)** — Determine what needs to be fixed
4. **[Codebase Sprawl Research](04-sprawl.md)** — Expand view based on root cause
5. **[Fork Fix + Staging PR](05-fork-fix.md)** — Fix in fork with fork-internal PR
6. **[Upstream PR](06-upstream-pr.md)** — Human gate before creating real PR

Or jump to the [workflow overview](index.md) to see how all the phases connect.
