# Git Integration

## Configuration

Git worktree settings in `.chant/config.md`:

```yaml
defaults:
  branch_prefix: "chant/"   # Prefix for worktree branches
  main_branch: "main"       # Target branch for merges
```

## Worktree Mode

Chant uses git worktrees for isolation. Each spec executes in its own worktree with a dedicated branch:

- Branch name: `{branch_prefix}{spec_id}` (e.g., `chant/2026-01-22-001-x7m`)
- Worktree location: `.chant/worktrees/{spec_id}/`
- Changes merged back to main branch after completion

> **Note:** Specs serve as the review primitive in chant. Each spec has a title, acceptance criteria, branch, commits, and review workflow — fulfilling the same role as a pull request but working offline and without external dependencies.

## Commit Flow

```
1. Agent commits work
   └── git commit -m "chant(2026-01-22-001-x7m): Add authentication"

2. CLI captures hash
   └── git rev-parse HEAD → a1b2c3d4

3. CLI updates frontmatter
   └── Adds: commit, completed_at, branch (if applicable)

4. CLI commits metadata update
   └── git commit -m "chant: mark 2026-01-22-001-x7m complete"
```

## Concurrency

Worktree isolation ensures each spec executes in its own branch without conflicts. However, **merge races** can occur when multiple specs finish simultaneously and attempt to merge back to main:

```
Terminal 1: chant work spec-a   →  git checkout main && git merge chant/spec-a
Terminal 2: chant work spec-b   →  git checkout main && git merge chant/spec-b
                                    ⚠️  Race: both merge to main at the same time
```

> **Warning:** Running multiple `chant work` processes in separate terminals can cause merge conflicts when specs finish concurrently. Both processes attempt `git checkout main && git merge` in the main repository, leading to race conditions.

### Safe Approach: Use `--parallel`

```bash
chant work --parallel 3    # Sequences all merges safely
```

The `--parallel` flag coordinates merge-back operations across workers, ensuring only one spec merges at a time.

### Watch Mode

Watch mode (`chant watch`) uses a PID lock to ensure only one instance runs, preventing concurrent merges by design.

---

## Custom Merge Driver

Chant includes a custom git merge driver that automatically resolves frontmatter conflicts in spec files.

### What It Does

When merging spec branches back to main, frontmatter conflicts commonly occur. The merge driver:

- Intelligently merges status, completed_at, and model fields
- Preserves implementation content
- Prefers "completed" status over "in_progress"
- Merges lists (commits, labels, target_files) with deduplication

### Installation

1. Add to `.gitattributes` in your repository root:
   ```
   .chant/specs/*.md merge=chant-spec
   ```

2. Configure git:
   ```bash
   git config merge.chant-spec.driver "chant merge-driver %O %A %B"
   git config merge.chant-spec.name "Chant spec merge driver"
   ```

### Verification

```bash
git config --get merge.chant-spec.driver
grep chant-spec .gitattributes
```
