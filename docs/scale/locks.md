# Locking & Recovery

## PID Files

Each running spec has a corresponding PID file:

```
.chant/
  specs/
    2026-01-22-001-x7m.md
  .locks/
    2026-01-22-001-x7m.pid
```

PID file contains:
```json
{
  "pid": 12345,
  "started": "2026-01-22T15:30:00Z",
  "host": "macbook.local",
  "user": "alex"
}
```

## Lock States

| PID file | Process alive | State |
|----------|---------------|-------|
| Missing | - | Available |
| Exists | Yes | Locked (in progress) |
| Exists | No | Crashed (needs recovery) |

## Acquiring a Lock

```rust
fn acquire_lock(spec_id: &str) -> Result<Lock> {
    let lock_path = format!(".chant/.locks/{}.pid", spec_id);

    if lock_path.exists() {
        let lock = read_lock(&lock_path)?;
        if process_alive(lock.pid) {
            return Err(Error::SpecLocked(spec_id, lock));
        }
        // Stale lock - previous agent crashed
        warn!("Removing stale lock for {}", spec_id);
        remove_file(&lock_path)?;
    }

    // Create lock
    let lock = Lock {
        pid: std::process::id(),
        started: Utc::now(),
        host: hostname(),
        user: username(),
    };
    write_lock(&lock_path, &lock)?;
    Ok(lock)
}
```

## Release on Completion

```rust
fn release_lock(spec_id: &str) {
    let lock_path = format!(".chant/.locks/{}.pid", spec_id);
    let _ = remove_file(&lock_path);  // Ignore errors
}
```

Lock is released when:
- Spec completes successfully
- Spec fails (status set to `failed`)
- CLI exits (cleanup)

## Crash Recovery

On `chant work`, check for stale locks:

```bash
$ chant work 2026-01-22-001-x7m
warning: Spec has stale lock (PID 12345 not running)
         Last started: 2026-01-22T15:30:00Z by alex@macbook.local

Recover and continue? [y/N]
```

Recovery options:
1. **Continue** - Remove stale lock, resume work
2. **Abort** - Leave spec in `in_progress`, investigate manually

## Concurrent Agents

Multiple agents can work simultaneously on *different* specs:

```
Agent A: chant work 2026-01-22-001-x7m  → acquires lock
Agent B: chant work 2026-01-22-002-q2n  → acquires lock  ✓
Agent C: chant work 2026-01-22-001-x7m  → blocked (locked)  ✗
```

## Distributed Locking

PID files work on a single machine. For distributed teams:

- Each clone is independent (different working copies)
- Conflicts resolved at git merge time
- No cross-machine locking needed

If same spec is worked on two machines:
1. Both complete independently
2. Git merge shows conflict
3. Human resolves

This is rare in practice - spec assignment prevents it.

## Lock Directory

`.chant/.locks/` is gitignored:

```gitignore
# .chant/.gitignore
.locks/
.store/
```

Locks are local state, not shared.

## Status File Alternative

Instead of separate PID files, could embed in spec:

```yaml
---
status: in_progress
_lock:
  pid: 12345
  started: 2026-01-22T15:30:00Z
  host: macbook.local
---
```

Pros: Single file
Cons: Modifies spec file, git noise

Recommendation: Separate `.locks/` directory. Cleaner separation.
