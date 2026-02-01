# Work Isolation

## The Problem

Multiple agents working simultaneously can conflict:
- Same file modified by two agents
- Uncommitted changes from one agent visible to another
- Merge conflicts at commit time

## Current Implementation

Chant uses **worktree + branch** isolation for parallel execution. Each spec gets:
- A dedicated worktree at `/tmp/chant-{spec-id}`
- A dedicated branch named `chant/{spec-id}`

```bash
# What chant does internally for parallel execution
git worktree add -b chant/2026-01-22-001-x7m /tmp/chant-2026-01-22-001-x7m
```

```
/tmp/
├── chant-2026-01-22-001-x7m/   ← Agent 1's isolated worktree
├── chant-2026-01-22-002-q2n/   ← Agent 2's isolated worktree
└── chant-2026-01-22-003-abc/   ← Agent 3's isolated worktree
```

On completion:
1. Agent commits to branch in worktree
2. CLI merges branch to main
3. Worktree removed

## Alternative Approaches (Design Reference)

These approaches are documented for context but are **not implemented**.

### 1. No Isolation (Locking Only)

Simplest. One agent per spec, PID locks prevent concurrent work on same spec.

```
Agent A: works on spec 1 → locks spec 1
Agent B: works on spec 2 → locks spec 2
Both modify different files → no conflict
```

**Problem**: If specs touch overlapping files, conflicts at commit.

**When it works**: Well-decomposed specs with clear file boundaries.

### 2. Branch Per Spec (No Worktree)

Agent works on dedicated branch in the same working directory:

```bash
git checkout -b chant/2026-01-22-001-x7m
# Agent works
git commit
# Merge back to main
```

**Pros**:
- Standard git workflow
- Easy to review changes

**Cons**:
- Still working in same directory
- Uncommitted changes visible to other agents
- No true isolation for parallel work

### 3. Shallow Clone

> **Status: Not Implemented**
>
> This approach was considered but not implemented. Worktree + branch provides
> better performance (no network fetch) while maintaining isolation.

Fresh clone per spec:

```bash
git clone --depth=1 --branch=main <repo> .chant/.clones/2026-01-22-001-x7m
cd .chant/.clones/2026-01-22-001-x7m
git checkout -b chant/2026-01-22-001-x7m
# Agent works
git push origin chant/2026-01-22-001-x7m
```

**Pros**:
- Clean slate every time
- Works with submodules

**Cons**:
- Network fetch for each spec
- More disk space (separate .git per clone)
- Slower than worktree creation

### Comparison

| Approach | Network | Disk | Complexity | Implemented |
|----------|---------|------|------------|-------------|
| No isolation | None | None | Low | N/A |
| Branch only | None | None | Low | Single-spec only |
| Shallow clone | Per spec | Medium | Low | No |
| **Worktree + branch** | None | Low | Medium | **Yes** |

## Inspecting Active Worktrees

Use `chant worktree status` to inspect active chant worktrees:

```bash
chant worktree status
```

This displays all chant-related worktrees with their associated spec, branch, disk usage, and health status. Useful for:
- Debugging worktree state during parallel execution
- Identifying orphaned worktrees
- Monitoring disk usage

See [CLI Reference: Worktree](../reference/cli.md#worktree) for full command documentation.

## Automatic Cleanup

```rust
fn cleanup_worktree(spec_id: &str) -> Result<()> {
    let worktree_path = format!("/tmp/chant-{}", spec_id);
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
/tmp/
├── chant-2026-01-22-001-x7m/
├── chant-2026-01-22-002-q2n/
└── chant-2026-01-22-003-abc/
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

## Sparse Checkout (Scale)

> **Status: Not Implemented**
>
> Sparse checkout for monorepos is planned but not yet implemented.

For monorepos, full checkout per worktree is expensive. Future support:

```yaml
# config.md (planned)
scale:
  worktree:
    sparse: true
    pattern: "packages/{{project}}/"
```

## Worktree Pool (Scale)

> **Status: Not Implemented**
>
> Worktree pooling is planned but not yet implemented.

Creating/destroying worktrees per spec adds overhead. Future support for pooling:

```yaml
scale:
  worktree:
    pool_size: 10    # Pre-created worktrees
```
