# Errors

## Error Categories

| Category | Examples | Severity |
|----------|----------|----------|
| **Parse** | Invalid YAML, missing fields | Blocking |
| **State** | Locked, blocked, wrong status | Blocking |
| **Execution** | Agent failed, tests failed | Spec fails |
| **Git** | Merge conflict, dirty worktree | Blocking |
| **System** | Disk full, permissions | Fatal |

## Error Catalog

### Parse Errors

**PARSE_INVALID_YAML**
```
Error: Invalid YAML in spec frontmatter
File: 2026-01-22-001-x7m.md
Line: 3
  status pending   ← Missing colon

Fix: Add colon after 'status'
```

**PARSE_MISSING_FIELD**
```
Error: Missing required field 'status'
File: 2026-01-22-001-x7m.md

Fix: Add 'status: pending' to frontmatter
```

**PARSE_INVALID_VALUE**
```
Error: Invalid status value 'open'
File: 2026-01-22-001-x7m.md
Allowed: pending, in_progress, completed, failed

Fix: Change status to valid value
```

### State Errors

**STATE_LOCKED**
```
Error: Spec is locked
Spec: 2026-01-22-001-x7m
Locked by: PID 12345 (alex@macbook.local)
Since: 2026-01-22T15:30:00Z

Options:
  - Wait for completion
  - chant unlock 2026-01-22-001-x7m (if stale)
```

**STATE_BLOCKED**
```
Error: Spec is blocked by dependencies
Spec: 2026-01-22-001-x7m

Waiting on:
  ✗ 2026-01-22-002-q2n (pending)
  ✓ 2026-01-22-003-abc (completed)

Fix: Complete pending dependencies first
```

**STATE_HAS_MEMBERS**
```
Error: Driver spec has pending members
Spec: 2026-01-22-001-x7m

Members:
  - 2026-01-22-001-x7m.1 (pending)
  - 2026-01-22-001-x7m.2 (pending)

Options:
  - chant work 2026-01-22-001-x7m.1
  - chant work 2026-01-22-001-x7m --parallel
```

**STATE_ALREADY_COMPLETE**
```
Error: Spec is already completed
Spec: 2026-01-22-001-x7m
Completed: 2026-01-22T15:30:00Z

Use --force to re-run (not recommended)
```

### Execution Errors

**EXEC_AGENT_FAILED**
```
Error: Agent execution failed
Spec: 2026-01-22-001-x7m
Exit code: 1

Last output:
  [15:30:45] Running tests...
  [15:30:52] FAILED: 2 assertions failed

Spec marked as 'failed'. See spec file for details.
```

**EXEC_TIMEOUT**
```
Error: Agent execution timed out
Spec: 2026-01-22-001-x7m
Timeout: 30m

Spec marked as 'failed'.
Options:
  - Increase timeout: chant work --timeout 60m
  - Break into smaller specs
```

### Git Errors

**GIT_DIRTY**
```
Error: Working directory has uncommitted changes
Files:
  M src/api/handler.go
  ? src/api/new.go

Options:
  - git stash
  - git commit
  - chant work --allow-dirty (not recommended)
```

**GIT_CONFLICT**
```
Error: Merge conflict after spec completion
Spec: 2026-01-22-001-x7m
Branch: chant/2026-01-22-001-x7m

Conflicts:
  - src/api/handler.go

Options:
  - Resolve manually and commit
  - chant retry 2026-01-22-001-x7m (re-run on current main)
```

**GIT_WORKTREE_FAILED**
```
Error: Failed to create worktree
Spec: 2026-01-22-001-x7m
Path: .chant/.worktrees/2026-01-22-001-x7m

Cause: Path already exists

Fix: rm -rf .chant/.worktrees/2026-01-22-001-x7m
```

### Dependency Errors

**DEP_CYCLE**
```
Error: Dependency cycle detected

Cycle:
  2026-01-22-001-x7m
  → 2026-01-22-002-q2n
  → 2026-01-22-003-abc
  → 2026-01-22-001-x7m

Fix: Remove one dependency to break cycle
```

**DEP_NOT_FOUND**
```
Error: Dependency not found
Spec: 2026-01-22-001-x7m
Missing: 2026-01-22-999-zzz

Fix: Remove invalid dependency or create missing spec
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Parse error |
| 3 | State error (locked, blocked) |
| 4 | Execution failed |
| 5 | Git error |
| 10 | Lint errors found |

## Error in Spec File

On failure, error details saved to spec:

```yaml
---
status: failed
failed_at: 2026-01-22T15:31:02Z
error: "EXEC_AGENT_FAILED: 2 test assertions failed"
log: |
  [15:30:01] Starting execution
  [15:30:45] Running tests...
  [15:30:52] FAILED: 2 assertions failed
---
```

## JSON Output

For tooling:

```bash
chant work 2026-01-22-001-x7m --json 2>&1
```

```json
{
  "error": {
    "code": "STATE_BLOCKED",
    "message": "Spec is blocked by dependencies",
    "spec": "2026-01-22-001-x7m",
    "details": {
      "waiting_on": ["2026-01-22-002-q2n"]
    }
  }
}
```

## Recovery Commands

| Error | Recovery |
|-------|----------|
| Stale lock | `chant unlock <id>` |
| Failed spec | `chant retry <id>` |
| Merge conflict | `chant resolve <id>` |
| Cycle | `chant deps --check` |
