---
name: release
purpose: Guide agents through complete release workflow
---

# Release Workflow

You are orchestrating a software release for {{project.name}}.

## Release Process

Follow these steps in order to prepare and publish a new release.

### Step 1: Pre-release Validation

Run `chant status` to check the project state:

```bash
chant status
```

**Requirements:**
- No specs should be `in_progress` (indicates ongoing work)
- No specs should be `failed` (indicates problems to address)
- Review any `pending` specs to determine if they should be completed before release

If there are `in_progress` or `failed` specs, **STOP** and report them. The user must decide whether to:
- Complete/fix them before releasing
- Move them to the next release cycle

### Step 2: Finalize Completed Work

Check for completed specs that haven't been finalized:

```bash
chant list --status completed
```

For each completed spec that needs finalization, run:

```bash
chant finalize <spec-id>
```

This validates that all acceptance criteria are checked. If finalization fails, review the spec and either:
- Check off remaining criteria if they're actually complete
- Mark the spec as `pending` if work remains

### Step 3: Merge Feature Branches

Merge all completed spec branches into main:

```bash
chant merge --all
```

**If merge conflicts occur:**
1. Note which specs have conflicts
2. Use `chant work <spec-id> --prompt merge-conflict` to resolve them
3. Re-run `chant merge --all` after resolution
4. Verify all merges succeeded

Confirm you're on the main branch and it's clean:

```bash
git status
```

### Step 4: Archive Completed Specs

Archive all completed and merged specs:

```bash
chant archive
```

Clean up any stale worktrees:

```bash
chant cleanup
```

Commit the archive changes:

```bash
git add .chant/archive/ .chant/specs/
git commit -m "chant: Archive completed specs for release"
```

### Step 5: Ensure Clean Git State

Verify you're on the main branch:

```bash
git branch --show-current
```

If not on `main`, switch to it:

```bash
git checkout main
```

Check for any uncommitted changes:

```bash
git status
```

If there are uncommitted changes (spec archives, logs, etc.), commit them:

```bash
git add .
git commit -m "chant: Prepare for release"
```

Push all changes to origin:

```bash
git push origin main
```

### Step 6: Version Bump and Release Preparation

**Ask the user which version bump to perform** (major/minor/patch) if not specified.

Read the current version:

```bash
grep '^version = ' Cargo.toml
```

Bump the version in `Cargo.toml`:
- **Major**: Breaking changes (X.0.0)
- **Minor**: New features (0.X.0)
- **Patch**: Bug fixes (0.0.X)

Update `Cargo.toml` with the new version number.

Update `Cargo.lock`:

```bash
cargo build
```

### Step 7: Generate Release Notes

Generate release notes from:

1. **Commits since last release:**
   ```bash
   git log $(git describe --tags --abbrev=0)..HEAD --oneline
   ```

2. **Completed specs in this release:**
   ```bash
   ls .chant/archive/ | grep "^$(date +%Y-%m-%d)"
   ```

3. **CHANGELOG.md content:**
   Read the unreleased section or create a new version section.

Update `CHANGELOG.md`:
- Add a new version section with today's date
- Categorize changes: Added, Changed, Fixed, Tests
- Use clear, user-focused descriptions

### Step 8: Commit Version Bump

Commit the version bump and changelog:

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chant: Release v<VERSION>"
```

Create and push the git tag:

```bash
git tag -a v<VERSION> -m "Release v<VERSION>"
git push origin main --tags
```

### Step 9: Create GitHub Release

Create a GitHub release using the `gh` CLI:

```bash
gh release create v<VERSION> \
  --title "v<VERSION>" \
  --notes-file <(cat <<'EOF'
<release notes content from CHANGELOG>
EOF
)
```

Verify the release was created:

```bash
gh release view v<VERSION>
```

### Step 10: Monitor CI (Optional)

Unless `{{skip_ci_wait}}` is set, monitor CI workflows:

```bash
gh run list --limit 5
```

Watch the latest workflow:

```bash
gh run watch
```

**If CI fails:**
1. Review the failure logs
2. Create a spec to fix the issue: `chant add "Fix CI failure in <workflow>"`
3. Work the spec, then retry the release workflow

Report CI status to the user when complete.

## Error Handling

### Common Issues

**Merge conflicts during `chant merge --all`:**
- Use the merge-conflict prompt: `chant work <spec-id> --prompt merge-conflict`
- Resolve conflicts carefully to preserve both changes
- Re-run merge after resolution

**Finalization fails due to unchecked criteria:**
- Review the spec file
- Verify the criteria are actually complete
- Check them off manually if necessary
- Consider creating a follow-up spec for incomplete work

**Git push rejected (remote has newer commits):**
- Pull changes: `git pull --rebase origin main`
- Resolve any conflicts
- Push again: `git push origin main --tags`

**GitHub release creation fails:**
- Verify `gh` CLI is authenticated: `gh auth status`
- Check that the tag was pushed: `git ls-remote --tags origin`
- Retry the `gh release create` command

**CI workflow fails:**
- Don't panic - this is normal
- Review logs to identify the issue
- Create a spec for the fix
- Work the spec, merge it, and re-run release

## Recovery Steps

**If you need to abort the release:**

1. Don't delete tags or commits already pushed
2. Note where you stopped in the process
3. Report to the user what was completed
4. The release can be resumed from any step

**If the version bump was wrong:**

1. Create a new commit with the correct version
2. Delete the incorrect tag: `git tag -d v<OLD_VERSION>`
3. Create the new tag: `git tag -a v<NEW_VERSION> -m "Release v<NEW_VERSION>"`
4. Force push tags: `git push origin --tags --force`
5. Update or recreate the GitHub release

## Success Criteria

A successful release includes:

- [ ] All specs finalized, merged, and archived
- [ ] Clean git status on main branch
- [ ] Version bumped in `Cargo.toml` and `Cargo.lock`
- [ ] `CHANGELOG.md` updated with release notes
- [ ] Git tag created and pushed
- [ ] GitHub release created with detailed notes
- [ ] CI passing (unless `skip_ci_wait` is set)

## Output Style

Work quietly. Minimize narration of your process.

**Do not output:**
- "Let me...", "Now I'll...", "I'm going to..."
- Step-by-step commentary on tool usage
- Thinking out loud

**Do output:**
- Errors or blockers that need attention
- Key decisions that affect the release
- Status updates at major milestones (e.g., "Merged 15 specs", "Created release v0.14.0")
- A final summary when complete

**Final summary format:**

```
## Release Summary

Released {{project.name}} v<VERSION>

- Merged and archived <N> completed specs
- Updated CHANGELOG.md with <N> items
- Created GitHub release: <URL>
- CI status: <passing/failed/skipped>

Next steps: <any follow-up items or known issues>
```
