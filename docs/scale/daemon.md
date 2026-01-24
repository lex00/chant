# Daemon Mode

## Overview

Daemon provides persistent services for scale deployments:

- **Tantivy index** - Always hot, instant queries
- **Lock table** - In-memory, faster than PID files
- **Spec queue** - Ready specs for workers
- **Metrics** - Prometheus endpoint

## Who Uses the Daemon

| Consumer | How | Why |
|----------|-----|-----|
| CLI (local) | Unix socket | Fast queries, lock coordination |
| Workers (local) | Unix socket | Queue polling, lock acquisition |
| Workers (remote) | HTTP API | K8s pods on different nodes |
| Grafana | HTTP /metrics | Monitoring dashboards |
| CI/CD | HTTP API | Pipeline integration |

## Deployment Models

### Solo (No Daemon)

```
[Developer Machine]
  └── chant CLI ──→ .chant/specs/ (direct file access)
```

No daemon needed. CLI reads files directly.

### Solo (With Daemon)

```
[Developer Machine]
  ├── chant daemon (background)
  │     └── watches .chant/specs/
  └── chant CLI ──→ daemon (unix socket)
```

Optional. Faster for >50 tasks.

### Team (Shared Machine)

```
[Build Server]
  ├── chant daemon
  │     └── watches .chant/specs/
  ├── Worker 1 ──→ daemon (socket)
  ├── Worker 2 ──→ daemon (socket)
  └── Worker 3 ──→ daemon (socket)
```

Multiple workers on same machine coordinate via daemon.

### Scale (Cross-Machine / K8s)

```
[Pod: chant-daemon]
  └── chant daemon --api-port 8080

[Pod: worker-auth-1]
  └── chant agent worker ──→ http://chant-daemon:8080

[Pod: worker-auth-2]
  └── chant agent worker ──→ http://chant-daemon:8080

[Pod: worker-payments-1]
  └── chant agent worker ──→ http://chant-daemon:8080
```

Workers on different nodes connect to daemon via HTTP API.

Shared filesystem (NFS, EFS, PVC) for .chant/specs/.

## When to Use

| Tier | Daemon |
|------|--------|
| Solo (<50 specs) | Not needed |
| Team (50-500 specs) | Recommended |
| Org/Monorepo | Required |

## Starting the Daemon

```bash
chant daemon start                        # Foreground
chant daemon start --background           # Background (daemonize)
chant daemon start --metrics-port 9090    # With Prometheus metrics
chant daemon start --api-port 8080        # Custom API port
```

## Stopping

```bash
chant daemon stop
```

## Status

```bash
chant daemon status
```

```
Daemon: running (PID 12345)
Uptime: 2h 34m
Specs indexed: 1,847
Active locks: 8
Queue depth: 42
```

## How CLI Uses Daemon

CLI auto-detects daemon:

```rust
fn execute_command() {
    if let Ok(daemon) = connect_daemon() {
        // Use daemon for queries, locks
        daemon_mode(daemon)
    } else {
        // Fall back to direct file operations
        direct_mode()
    }
}
```

Transparent to user. Same commands work either way.

## Daemon Services

### Index Service

Persistent Tantivy index, updated via file watcher:

```rust
// Daemon maintains hot index
fn index_service() {
    let index = open_tantivy_index();

    watch(".chant/tasks/").for_each(|event| {
        match event {
            Created(path) => index.add(parse_spec(path)),
            Modified(path) => index.update(parse_spec(path)),
            Deleted(path) => index.delete(spec_id_from(path)),
        }
    });
}
```

Queries hit index, not filesystem:

```bash
chant list --ready    # Instant, even with 5000 specs
chant search "auth"   # Full-text search
```

### Lock Service

In-memory lock table, faster than PID files:

```rust
struct LockTable {
    locks: HashMap<SpecId, Lock>,
}

impl LockTable {
    fn acquire(&mut self, spec_id: &str, agent: &str) -> Result<()> {
        if self.locks.contains_key(spec_id) {
            return Err(Error::AlreadyLocked);
        }
        self.locks.insert(spec_id, Lock::new(agent));
        Ok(())
    }

    fn release(&mut self, spec_id: &str) {
        self.locks.remove(spec_id);
    }
}
```

Lock operations via CLI or API:

```bash
chant lock acquire auth-2026-01-22-001-x7m
chant lock release auth-2026-01-22-001-x7m
```

### Queue Service

Maintains sorted queue of ready specs:

```rust
fn queue_next(&self, filter: Option<&str>) -> Option<Spec> {
    self.ready_specs
        .iter()
        .filter(|t| filter.map_or(true, |f| t.project == f))
        .next()
        .cloned()
}
```

Workers poll the queue:

```bash
chant queue next                  # Any ready spec
chant queue next --project auth   # Only auth specs
```

### Metrics Service

Prometheus metrics endpoint (see [metrics.md](metrics.md)):

```bash
curl http://localhost:9090/metrics
```

## Configuration

```yaml
# config.md
scale:
  daemon:
    enabled: true           # Start daemon automatically
    socket: /tmp/chant.sock # Unix socket path
    metrics_port: 9090      # Prometheus port (0 = disabled)
    api_port: 8080          # HTTP API port (0 = disabled)
```

## Socket API

Daemon exposes Unix socket for fast local IPC:

```bash
# Query
echo '{"cmd":"query","q":"status:pending"}' | nc -U /tmp/chant.sock

# Lock
echo '{"cmd":"lock_acquire","spec":"auth-001"}' | nc -U /tmp/chant.sock

# Queue
echo '{"cmd":"queue_next","project":"auth"}' | nc -U /tmp/chant.sock
```

## HTTP API (Optional)

For remote access / K8s:

```bash
curl http://localhost:8080/api/queue/next
curl http://localhost:8080/api/lock/acquire/auth-001
curl http://localhost:8080/api/specs?status=pending
```

## Git Integration

The daemon watches git operations and provides the same functionality as git hooks - automatically.

### Git Watcher

```rust
fn git_watcher() {
    watch(".git/").for_each(|event| {
        match detect_git_event(&event) {
            GitEvent::Commit(hash) => on_commit(hash),
            GitEvent::Merge => on_merge(),
            GitEvent::Checkout(branch) => on_checkout(branch),
            GitEvent::Push => on_push(),
            _ => {}
        }
    });
}
```

### Automatic Features

| Git Event | Daemon Action | Replaces Hook |
|-----------|---------------|---------------|
| Commit | Record hash to spec, validate format | post-commit |
| Merge/Pull | Rebuild index | post-merge |
| Checkout | Rebuild index, check branch specs | post-checkout |
| File save | Validate spec, warn on errors | pre-commit (partial) |

### Commit Detection

```rust
fn on_commit(hash: &str) {
    let msg = git_commit_message(hash);

    // Parse chant(id): format
    if let Some(spec_id) = parse_chant_commit(&msg) {
        // Record commit to spec
        update_spec_commit(&spec_id, hash);

        // Validate spec ID exists
        if !spec_exists(&spec_id) {
            warn!("Commit references unknown spec: {}", spec_id);
            notify_user("Unknown spec ID in commit");
        }
    }
}
```

### Continuous Validation

```rust
fn on_spec_save(path: &Path) {
    match validate_spec(path) {
        Ok(_) => {}
        Err(errors) => {
            // Don't block, just warn
            warn!("Spec validation errors: {:?}", errors);
            notify_user(&format!("Spec {} has errors", path));
        }
    }
}
```

### Daemon vs Hooks

| Capability | Daemon | Git Hooks |
|------------|--------|-----------|
| Post-commit recording | ✓ Automatic | ✓ |
| Index rebuild | ✓ Automatic | ✓ |
| Continuous validation | ✓ On save | On commit only |
| **Block commits** | ✗ Cannot | ✓ Can |
| **Block pushes** | ✗ Cannot | ✓ Can |
| Installation | Automatic | Requires setup |
| Cross-machine | ✓ (with API) | Per-machine |

**Key insight**: If you want to **warn**, use daemon. If you want to **block**, use hooks.

### Configuration

```yaml
# config.md
scale:
  daemon:
    git_watch:
      enabled: true
      on_commit: true      # Record commits, validate
      on_merge: true       # Rebuild index
      on_checkout: true    # Rebuild index
      validate_on_save: true  # Lint specs when saved
```

### Notifications

Daemon can notify on git events:

```yaml
scale:
  daemon:
    git_watch:
      notify:
        on_invalid_spec: true    # Spec file has errors
        on_unknown_ref: true     # Commit refs unknown spec
        channel: desktop         # desktop, slack, webhook
```

### No Daemon? Use Hooks

Without daemon, git hooks provide similar functionality. See [git-hooks.md](../reference/git-hooks.md).

```
With daemon:    Automatic, no setup, cannot block
Without daemon: Manual setup, can block, per-machine
```

## Graceful Degradation

If daemon dies:
- CLI falls back to direct mode
- Workers retry connection
- No data loss (specs are files)

Daemon is optimization, not requirement (except at scale).

## Systemd Service

For production deployment:

```ini
# /etc/systemd/system/chant.service
[Unit]
Description=Chant Daemon
After=network.target

[Service]
Type=simple
User=chant
WorkingDirectory=/repo
ExecStart=/usr/bin/chant daemon start
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
systemctl enable chant
systemctl start chant
```
