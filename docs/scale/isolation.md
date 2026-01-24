# Work Isolation

## The Problem

Multiple agents working simultaneously can conflict:
- Same file modified by two agents
- Uncommitted changes from one agent visible to another
- Merge conflicts at commit time

## Approaches

### 1. No Isolation (Locking Only)

Simplest. One agent per spec, PID locks prevent concurrent work on same spec.

```
Agent A: works on spec 1 → locks spec 1
Agent B: works on spec 2 → locks spec 2
Both modify different files → no conflict
```

**Problem**: If specs touch overlapping files, conflicts at commit.

**When it works**: Well-decomposed specs with clear file boundaries.

### 2. Git Worktrees

Each agent gets its own worktree:

```bash
git worktree add .chant/.worktrees/2026-01-22-001-x7m
```

```
repo/
├── .git/
├── src/                          ← Main worktree
└── .chant/
    └── .worktrees/
        └── 2026-01-22-001-x7m/   ← Agent's isolated copy
            └── src/
```

**Pros**:
- Full isolation
- Shared .git (efficient)
- Each agent has clean state

**Cons**:
- Disk space (full checkout per agent)
- Merge required after completion

### 3. Branch Per Spec

Agent works on dedicated branch:

```bash
git checkout -b chant/2026-01-22-001-x7m
# Agent works
git commit
# Merge back to main
```

**Pros**:
- Standard git workflow
- Easy to review changes
- Natural PR integration

**Cons**:
- Still working in same directory
- Uncommitted changes visible
- Need worktree for true isolation

### 4. Shallow Clone (Recommended)

Fresh clone per spec. Simpler than worktrees, more reliable.

```bash
# Create shallow clone
git clone --depth=1 --branch=main <repo> .chant/.clones/2026-01-22-001-x7m
cd .chant/.clones/2026-01-22-001-x7m
git checkout -b chant/2026-01-22-001-x7m
# Agent works
git push origin chant/2026-01-22-001-x7m
```

**Pros**:
- Clean slate every time
- No worktree complexity
- Works with submodules
- Robust - fewer edge cases
- Easy cleanup (just delete directory)

**Cons**:
- Network fetch for each spec
- More disk space than worktrees (separate .git)
- Slower clone than worktree creation

**Why shallow?**
- `--depth=1` fetches only latest commit
- Fast clone, minimal disk
- Agent doesn't need history

```rust
fn create_clone(spec_id: &str) -> Result<PathBuf> {
    let clone_path = format!(".chant/.clones/{}", spec_id);
    let remote = get_remote_url()?;

    Command::new("git")
        .args(["clone", "--depth=1", "--branch", "main", &remote, &clone_path])
        .status()?;

    Command::new("git")
        .args(["-C", &clone_path, "checkout", "-b", &format!("chant/{}", spec_id)])
        .status()?;

    Ok(clone_path.into())
}
```

### 5. Worktree + Branch (Alternative)

Combine worktrees with branches. More complex but shares .git:

```bash
# Create branch and worktree together
git worktree add -b chant/2026-01-22-001-x7m .chant/.worktrees/2026-01-22-001-x7m
```

Agent works in isolated worktree on dedicated branch.

**Known issues with worktrees:**
- Submodules require special handling
- Some tools don't detect worktrees properly
- Can get into inconsistent states
- Same branch can't be checked out twice

**When worktrees make sense:**
- Local-only work (no push)
- Very fast iteration
- Large repo where clone is slow

On completion:
1. Agent commits to branch
2. CLI merges branch to main (or creates PR)
3. Worktree removed

### Comparison

| Approach | Network | Disk | Complexity | Robustness |
|----------|---------|------|------------|------------|
| No isolation | None | None | Low | Conflicts |
| Branch only | None | None | Low | Dirty state |
| Shallow clone | Per spec | Medium | Low | High |
| Worktree | None | Low | Medium | Medium |

**Recommendation**: Start with shallow clones. Switch to worktrees only if clone speed is a bottleneck.

> **Research needed**: Worktree approach needs deeper investigation. Previous implementation (Chant v1) encountered issues that led to switching to shallow clones. Need to study:
> - Submodule handling in worktrees
> - Tool compatibility (IDEs, linters, etc.)
> - Edge cases with branch management
> - Sparse checkout + worktree combination
> - Cleanup reliability

## Automatic Cleanup

```rust
fn cleanup_worktree(spec_id: &str) -> Result<()> {
    let worktree_path = format!(".chant/.worktrees/{}", spec_id);
    let branch_name = format!("chant/{}", spec_id);

    // Remove worktree
    Command::new("git")
        .args(["worktree", "remove", &worktree_path])
        .status()?;

    // Delete branch if merged
    Command::new("git")
        .args(["branch", "-d", &branch_name])
        .status()?;

    Ok(())
}
```

## Parallel with Isolation

```bash
chant work --parallel --max 3
```

Creates up to 3 worktrees:

```
.chant/.worktrees/
├── 2026-01-22-001-x7m/
├── 2026-01-22-002-q2n/
└── 2026-01-22-003-abc/
```

Each agent works independently. Merges happen sequentially after completion.

## Merge Conflicts

If parallel agents modify same file:

1. First to complete merges cleanly
2. Second encounters conflict
3. Options:
   - Auto-resolve (if trivial)
   - Flag for human review
   - Re-run agent with updated base

```bash
$ chant work --parallel
[2026-01-22-001-x7m] Complete, merged
[2026-01-22-002-q2n] Complete, merge conflict

Conflict in: src/api/handler.go
  - Manual resolution required
  - Or: chant retry 2026-01-22-002-q2n (re-runs on current main)
```

## Target Files Hint

Specs can declare expected files:

```yaml
---
status: pending
target_files:
  - src/auth/middleware.go
  - src/auth/jwt.go
---
```

CLI warns on overlap:

```bash
$ chant work --parallel 2026-01-22-001-x7m 2026-01-22-002-q2n
Warning: Specs have overlapping target files:
  - src/auth/middleware.go

Continue anyway? [y/N]
```

## Configuration

```yaml
# config.md frontmatter
defaults:
  isolation: clone       # none | branch | clone | worktree
  branch: true           # Create branches
  clone:
    depth: 1             # Shallow clone (default)
    # depth: 0           # Full clone (if history needed)
```

## Sparse Checkout (Scale)

For monorepos, full checkout per worktree is expensive. Use sparse checkout:

```yaml
# config.md
scale:
  worktree:
    sparse: true
    pattern: "packages/{{project}}/"
```

Chant automatically configures sparse checkout:

```rust
fn create_sparse_worktree(spec: &Spec) -> Result<PathBuf> {
    let worktree_path = format!(".chant/.worktrees/{}", spec.id);
    let project = extract_project(&spec.id);

    // Create worktree with sparse checkout
    Command::new("git")
        .args(["worktree", "add", "--sparse", &worktree_path])
        .status()?;

    // Configure sparse checkout for project
    let sparse_path = format!("packages/{}/", project);
    Command::new("git")
        .args(["-C", &worktree_path, "sparse-checkout", "set", &sparse_path])
        .status()?;

    Ok(worktree_path.into())
}
```

Result: Worktree only contains relevant project files.

## Worktree Pool (Scale)

Creating/destroying worktrees is slow. Pool and reuse:

```yaml
scale:
  worktree:
    pool_size: 10    # Pre-created worktrees
```

```
.chant/.worktrees/
  pool-01/   # Available
  pool-02/   # In use (auth-2026-01-22-001-x7m)
  pool-03/   # In use (payments-2026-01-22-002-q2n)
  ...
```

Worker claims worktree from pool, reconfigures sparse checkout, executes, returns to pool.

## Worktree Location

Worktrees are gitignored local state:

```gitignore
# .chant/.gitignore
.locks/
.store/
.worktrees/
```

Not committed. Each clone/pod manages its own.
