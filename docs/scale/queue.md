# Queue Architecture

> **Status: Partially Implemented** ⚠️
>
> The daemon-free queue (Tiers 1-2) is implemented through file-based coordination.
> Advanced queue backends (Tier 3+: Tantivy, PostgreSQL, Redis) are not yet implemented.
> See [Roadmap](../roadmap/roadmap.md) - Phase 6 for queue tier details.

## Core Insight

**Spec files ARE the queue.** The queue is derived, not stored separately.

```
Ready specs = specs WHERE status=pending AND deps_met AND not_locked
```

No daemon required for basic coordination. Teams can work from repo files alone.

## Daemon-Free Mode (Default)

Small teams work without a daemon:

```
┌─────────────────────────────────────────────┐
│              Git Repository                 │
│                                             │
│  .chant/specs/*.md    ← Spec state          │
│  .chant/.locks/*.pid  ← Who's working       │
│                                             │
└─────────────────────────────────────────────┘
        ↑           ↑           ↑
     Alice        Bob        Carol
   (chant work) (chant work) (chant work)
```

**How it works:**

```rust
fn find_ready_specs() -> Vec<Spec> {
    scan_specs(".chant/specs/")
        .filter(|s| s.status == "pending")
        .filter(|s| deps_met(s))
        .filter(|s| !is_locked(s))  // Check .locks/
        .collect()
}

fn claim_spec(spec_id: &str) -> Result<()> {
    // Atomic lock file creation
    let lock_path = format!(".chant/.locks/{}.pid", spec_id);
    create_exclusive(&lock_path)?;  // Fails if exists
    write_pid(&lock_path)?;
    Ok(())
}
```

**Coordination via git:**
- Push/pull syncs spec state
- Lock files prevent double-work
- Git conflicts = coordination signal

**Works for:**
- Small teams (2-10 people)
- Async collaboration (different timezones)
- No infrastructure required

## Daemon-Free Team Workflow

### Scenario: Alice and Bob work on same repo

```
Alice's machine                    Bob's machine
      │                                  │
      ▼                                  ▼
┌─────────────┐                   ┌─────────────┐
│ git pull    │                   │ git pull    │
│ chant list  │                   │ chant list  │
│ chant work  │                   │ chant work  │
│ git push    │                   │ git push    │
└─────────────┘                   └─────────────┘
      │                                  │
      └──────────┬───────────────────────┘
                 ▼
         ┌──────────────┐
         │ Git Remote   │
         │ (GitHub, etc)│
         └──────────────┘
```

### How Double-Work is Prevented

1. **PID Lock Files** (local machine)
   ```
   .chant/.locks/2026-01-22-001-x7m.pid
   ```
   Prevents two local processes from working same spec.

2. **Status in Frontmatter** (synced via git)
   ```yaml
   status: in_progress
   started_by: alice
   started_at: 2026-01-22T10:00:00Z
   ```
   When Alice starts, she commits this. Bob sees it on pull.

3. **Git as Coordination**
   - Alice: `chant work 001` → status changes → `git push`
   - Bob: `git pull` → sees 001 is in_progress → picks different spec

### Race Condition Handling

What if Alice and Bob start same spec simultaneously?

```
Alice: chant work 001 → changes status → git push ✓
Bob:   chant work 001 → changes status → git push ✗ (conflict)
```

Bob's push fails. He must:
```bash
git pull --rebase
# Sees conflict in spec file
# Realizes Alice is working on it
chant work 002  # Pick different spec
```

**Git conflicts are a feature, not a bug.** They signal coordination needed.

### Lock File Details

```rust
// .chant/.locks/2026-01-22-001-x7m.pid
struct LockFile {
    pid: u32,
    hostname: String,
    user: String,
    started: DateTime,
}

// Check if lock is stale
fn is_stale(lock: &LockFile) -> bool {
    // Different machine = can't check PID
    if lock.hostname != current_hostname() {
        return lock.started < now() - Duration::hours(4);
    }
    // Same machine = check if process exists
    !process_exists(lock.pid)
}
```

Lock files are gitignored (local only). Status in frontmatter syncs via git.

### When to Add Daemon

Add daemon when:
- Scanning files is too slow (>100ms for list)
- Want real-time visibility across team
- Running multiple local workers
- Need queue prioritization

Daemon is **optimization**, not requirement

## Requirements (When Daemon Used)

| Requirement | Why |
|-------------|-----|
| Exactly-once delivery | Don't run same spec twice |
| Persistence | Survive daemon restart |
| Priority support | Urgent tasks first |
| Filtering | By project, labels, etc. |
| Visibility | Who's working on what |
| Distributed (optional) | Multi-node workers |

**Not required:**
- Ultra-high throughput (specs take minutes, not milliseconds)
- Strict ordering (parallel execution by design)
- Complex routing (simple work queue pattern)

## Queue Tiers (With Daemon)

### Tier 1: Derived from Files (Team)

No separate queue storage. Daemon caches spec state in memory, rebuilds from files.

```rust
struct DerivedQueue {
    // In-memory cache, rebuilt from .chant/specs/ on startup
    specs: HashMap<SpecId, SpecState>,
    // File watcher keeps it current
    watcher: FileWatcher,
}

impl DerivedQueue {
    fn ready(&self) -> Vec<&Spec> {
        self.specs.values()
            .filter(|s| s.is_ready())
            .collect()
    }
}
```

- No separate storage
- Rebuilds instantly from files
- Source of truth is always markdown

**Good for**: Teams, single machine, daemon for speed

### Tier 2: Tantivy Index (Scale)

We already have Tantivy for search. Use it for queue queries too.

```rust
fn ready_specs_from_index(index: &TantivyIndex) -> Vec<SpecId> {
    index.search("status:pending AND ready:true")
        .sort_by("priority", Descending)
        .collect()
}
```

- Already have the index
- Fast queries at scale
- Persistent across restarts

**Good for**: Larger repos, need fast queries

### Tier 3: PostgreSQL (Distributed)

```sql
-- Use SKIP LOCKED for distributed queue
BEGIN;
SELECT spec_id FROM queue
WHERE status = 'ready'
ORDER BY priority DESC, created_at ASC
LIMIT 1
FOR UPDATE SKIP LOCKED;

UPDATE queue SET status = 'claimed', claimed_by = $1
WHERE spec_id = $2;
COMMIT;
```

- True distributed coordination
- Multiple daemons can coordinate
- External infrastructure required

**Good for**: Multi-node, K8s, high reliability

### Tier 4: Redis (Distributed, Simpler)

```redis
# Sorted set by priority
ZADD chant:ready <priority> <spec_id>

# Atomic pop
BZPOPMAX chant:ready 0
```

- Fast, distributed
- Simpler ops than PostgreSQL
- Good ecosystem

**Good for**: Multi-node, K8s, want simplicity

## When to Use What

| Scale | Daemon | Queue Backend |
|-------|--------|---------------|
| Solo | No | Files only |
| Small team | No | Files + locks |
| Team (speed) | Yes | Derived/Tantivy |
| Org | Yes | Tantivy |
| Enterprise | Yes | PostgreSQL or Redis |

## Why Not Kafka?

Kafka is designed for:
- Millions of messages/second
- Event streaming
- Log aggregation
- Pub/sub patterns

Chant tasks are:
- 10-1000 per day
- Minutes to complete
- Simple work queue pattern
- No replay/streaming needed

**Kafka is overkill.** Files work for most. PostgreSQL/Redis for enterprise.

## Configuration

```yaml
# config.md
scale:
  daemon:
    enabled: false         # Default: no daemon needed

  # Only relevant if daemon enabled
  queue:
    backend: derived       # derived, tantivy, postgres, redis

    # PostgreSQL (enterprise)
    postgres:
      url: ${DATABASE_URL}
      table: chant_queue
      pool_size: 10

    # Redis (enterprise)
    redis:
      url: ${REDIS_URL}
      prefix: chant
```

Most teams don't need to configure this. Files + locks just work.

## Queue Interface

```rust
trait QueueBackend {
    /// Add spec to queue
    fn enqueue(&self, spec_id: &str, priority: i32) -> Result<()>;

    /// Claim next ready spec (atomic)
    fn claim(&self, worker_id: &str, filter: Option<&Filter>) -> Result<Option<SpecId>>;

    /// Mark spec complete (remove from queue)
    fn complete(&self, spec_id: &str) -> Result<()>;

    /// Release claim (spec failed, return to queue)
    fn release(&self, spec_id: &str) -> Result<()>;

    /// Get queue stats
    fn stats(&self) -> Result<QueueStats>;

    /// List claimed tasks (visibility)
    fn list_claimed(&self) -> Result<Vec<ClaimedTask>>;
}

struct Filter {
    project: Option<String>,
    labels: Option<Vec<String>>,
    max_priority: Option<i32>,
}

struct QueueStats {
    ready: usize,
    claimed: usize,
    by_project: HashMap<String, usize>,
}
```

## Daemon Queue Flow

```rust
fn daemon_main() {
    let queue = match config.queue.backend {
        "memory" => Box::new(InMemoryQueue::new()),
        "file" => Box::new(FileQueue::new(&config.queue.file.path)),
        "lmdb" => Box::new(LmdbQueue::new(&config.queue.lmdb.path)),  // Pure Rust via heed
        "postgres" => Box::new(PostgresQueue::new(&config.queue.postgres.url)),
        "redis" => Box::new(RedisQueue::new(&config.queue.redis.url)),
    };

    // Watch for new ready specs
    watch_specs().for_each(|event| {
        if event.became_ready() {
            queue.enqueue(&event.spec_id, event.priority);
        }
    });

    // Serve worker requests
    serve_workers(queue);
}
```

## Worker Claim Flow

```rust
fn worker_claim_next(daemon: &Daemon, filter: Option<&Filter>) -> Option<Spec> {
    // 1. Claim from queue (atomic)
    let spec_id = daemon.queue.claim(&worker_id, filter)?;

    // 2. Acquire lock (belt + suspenders)
    daemon.locks.acquire(&spec_id)?;

    // 3. Return spec
    Some(daemon.get_spec(&spec_id))
}
```

## Failure Handling

### Worker Crash

Daemon detects via:
1. Heartbeat timeout
2. Lock expiry
3. TCP disconnect

On detection:
```rust
fn handle_worker_crash(worker_id: &str) {
    for spec_id in queue.claimed_by(worker_id) {
        queue.release(&spec_id);  // Return to ready
        locks.release(&spec_id);
    }
}
```

### Daemon Crash

On restart:
```rust
fn daemon_startup() {
    // Rebuild queue from spec files
    for spec in scan_specs() {
        if spec.is_ready() {
            queue.enqueue(&spec.id, spec.priority);
        }
    }

    // Check for orphaned claims (stale)
    for claim in queue.list_claimed() {
        if claim.is_stale() {
            queue.release(&claim.spec_id);
        }
    }
}
```

## Priority

```yaml
# Spec frontmatter
---
status: pending
priority: high    # low (0), normal (50), high (100), critical (200)
---
```

Or numeric:
```yaml
priority: 150
```

Higher number = processed first.

## Multi-Daemon (Distributed)

With PostgreSQL or Redis, multiple daemons can run:

```
[Daemon A]  ──┐
              ├──→ [PostgreSQL/Redis] ←──┬── [Worker 1]
[Daemon B]  ──┘                          ├── [Worker 2]
                                         └── [Worker N]
```

Each daemon:
- Watches local spec files
- Enqueues to shared backend
- Can serve any worker

Useful for:
- Multi-region
- High availability
- Monorepo with multiple entry points

## Summary

| Mode | Daemon | Backend | Use Case |
|------|--------|---------|----------|
| Files only | No | Spec files + PID locks | Solo, small team |
| Derived | Yes | In-memory from files | Team wanting speed |
| Tantivy | Yes | Search index | Large repos |
| PostgreSQL | Yes | External DB | Multi-node enterprise |
| Redis | Yes | External cache | Multi-node enterprise |

**Default path**: No daemon, files only. Works for most teams.

**Key insight**: The markdown files are the source of truth. Everything else is optimization.
