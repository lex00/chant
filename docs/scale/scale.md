# Chant at Scale

## The Question

Is "Chant at scale" just a deployment pattern, or does it need built-in features?

**Answer: Both.**

- **Built-in features** enable scale (IDs, daemon, metrics)
- **Deployment patterns** leverage those features (K8s, Grafana)

Chant doesn't build an orchestrator. Chant exposes primitives that orchestrators can use.

## Scale Tiers

| Tier | Specs | Agents | Deployment | Features Needed |
|------|-------|--------|------------|-----------------|
| **Solo** | <50 | 1 | CLI | Base design |
| **Team** | 50-500 | 1-10 | Pipelines | Daemon mode |
| **Org** | 500-5000 | 10-100 | K8s | Project IDs, metrics |
| **Monorepo** | 5000+ | 100+ | K8s + custom | All scale features |

### Team Tier: Pipelines

Not full K8s, but automated execution via:

- **GitHub Actions** - Workflow triggers `chant work --parallel`
- **GitLab CI** - Pipeline jobs run agents
- **Cron** - Scheduled batch execution
- **Simple scripts** - `./run-agents.sh`

Example GitHub Action:

```yaml
# .github/workflows/chant.yml
name: Chant Agents
on:
  workflow_dispatch:    # Manual trigger
  schedule:
    - cron: '0 * * * *' # Hourly

jobs:
  work:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        agent: [1, 2, 3, 4]  # 4 parallel agents
    steps:
      - uses: actions/checkout@v4
      - run: chant daemon start --background
      - run: chant work --next --timeout 30m
```

Example script:

```bash
#!/bin/bash
# run-agents.sh - Run N agents in parallel

chant daemon start --background

for i in $(seq 1 $NUM_AGENTS); do
  chant agent worker &
done

wait
```

Pipelines work when:
- Execution is batch (not continuous)
- Scale is moderate (1-10 agents)
- Don't need real-time orchestration

## Built-in Features for Scale

### 1. Pools / Resource Management

Pools limit concurrent execution by resource type:

```yaml
# config.md
pools:
  high-capability:
    description: "Expensive, use sparingly"
    max_concurrent: 3
    cost_limit_daily: $50

  standard:
    max_concurrent: 10

  local:
    max_concurrent: 20
    # No cost limit - local

  default:
    max_concurrent: 5
```

**Spec assignment to pool:**

```yaml
# Spec frontmatter
---
pool: high-capability
---
```

**Or auto-assign by model:**

```yaml
# config.md
pools:
  auto_assign:
    by_model:
      "high-*": high-capability
      "standard-*": standard
      "local-*": local
```

**Pool exhaustion handling:**

| Behavior | Description |
|----------|-------------|
| Queue | Spec waits until slot available (default) |
| Priority | Higher priority tasks get slots first |
| Preempt | Cancel lower priority spec (optional) |
| Fail | Reject if pool exhausted |

```yaml
pools:
  high-capability:
    max_concurrent: 3
    on_exhausted: queue           # queue | fail
    priority_field: priority      # frontmatter field for priority
```

**CLI:**

```bash
$ chant pools
Pool             Active  Max   Queued  Cost Today
high-capability  2/3     3     5       $23.45
standard         8/10    10    12      $8.20
local            15/20   20    0       $0.00

$ chant pools --pool high-capability
Pool: high-capability
  Active: 2/3
  Queued: 5
  Daily limit: $50.00
  Used today: $23.45

  Active tasks:
    001 - Add auth (running 5m)
    002 - Fix bug (running 2m)

  Queued:
    003 - Update docs (waiting 10m)
    004 - Add tests (waiting 8m)
    ...
```

### 2. Project-Prefixed IDs

At scale, need to namespace specs by project:

```
auth-2026-01-22-001-x7m
payments-2026-01-22-002-q2n
```

**Config:**
```yaml
# config.md
project:
  name: auth
  prefix: auth      # Spec IDs prefixed with this
```

**Or auto-detect from monorepo structure:**
```yaml
scale:
  id_prefix:
    from: path
    pattern: "packages/([^/]+)/"   # packages/auth/ → auth-
```

This is a **built-in feature**. No external tooling needed.

### 2. Daemon Mode

At scale, can't parse thousands of spec files per command. Need persistent indexes.

```bash
chant daemon start
```

Daemon provides:
- Tantivy index (keyword search, always hot)
- arroy index (semantic search, optional)
- Lock table (in-memory, fast)
- File watcher (instant updates)
- Unix socket API

**Search infrastructure:**

```
Daemon process
├── Tantivy (keyword)     # Memory-mapped, scales to millions
├── arroy (semantic)      # Memory-mapped via LMDB, same scale
├── fastembed model       # Loaded once, ~200MB RAM
└── File watcher          # Instant index updates
```

Both Tantivy and arroy use memory-mapped storage. Same scaling patterns. Both handle millions of documents on a single machine.

**Built-in feature.** CLI connects to daemon if running, falls back to direct mode.

```rust
fn get_ready_specs() -> Vec<Spec> {
    if let Ok(daemon) = connect_daemon() {
        daemon.query("status:pending AND ready:true")
    } else {
        // Fallback: parse files directly
        load_and_filter_specs()
    }
}
```

### 3. Metrics Endpoint

Expose Prometheus metrics for Grafana:

```bash
chant daemon start --metrics-port 9090
```

```
# HELP chant_specs_total Total tasks by status
# TYPE chant_specs_total gauge
chant_specs_total{status="pending"} 142
chant_specs_total{status="in_progress"} 8
chant_specs_total{status="completed"} 1847

# HELP chant_agents_active Currently running agents
# TYPE chant_agents_active gauge
chant_agents_active 8

# HELP chant_spec_duration_seconds Spec completion time
# TYPE chant_spec_duration_seconds histogram
chant_spec_duration_seconds_bucket{le="60"} 45
chant_spec_duration_seconds_bucket{le="300"} 120
```

**Built-in feature.** Grafana dashboards are deployment configuration.

### 4. Lock API

Daemon exposes lock operations:

```bash
# CLI commands (for scripts/automation)
chant lock acquire <spec-id>
chant lock release <spec-id>
chant lock list

# Or via Unix socket (for custom tooling)
echo '{"cmd":"lock","spec":"auth-001"}' | nc -U /tmp/chant.sock
```

**Built-in feature.** Orchestrators use this API.

### 5. Queue Primitives

Daemon can maintain a work queue:

```bash
chant queue next                    # Get next ready spec
chant queue next --project auth     # Filter by project
chant queue stats                   # Queue depth, wait times
```

**Built-in feature.** Orchestrators poll this, assign to agents.

## Deployment Patterns

### Pattern 1: K8s Deployment

```yaml
# chant-daemon.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chant-daemon
spec:
  replicas: 1    # Single daemon per repo
  template:
    spec:
      containers:
      - name: chant
        image: chant:latest
        command: ["chant", "daemon", "start", "--metrics-port", "9090"]
        volumeMounts:
        - name: repo
          mountPath: /repo
        ports:
        - containerPort: 9090   # Metrics
        - containerPort: 8080   # API
```

```yaml
# chant-agent.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chant-agents
spec:
  replicas: 10   # Agent pool size
  template:
    spec:
      containers:
      - name: agent
        image: chant:latest
        command: ["chant", "agent", "worker"]
        env:
        - name: CHANT_DAEMON
          value: "chant-daemon:8080"
```

Chant provides the containers. K8s provides orchestration.

### Pattern 2: Agent Worker Mode

New command for scale deployments:

```bash
chant agent worker
```

Worker cycle:
1. Connect to daemon
2. Request next spec from queue
3. Acquire lock
4. Execute spec
5. Release lock
6. Repeat

```rust
fn worker_cycle() {
    let daemon = connect_daemon()?;

    loop {
        // Get next ready spec
        let spec = daemon.queue_next()?;

        if let Some(spec) = spec {
            // Acquire lock
            daemon.lock_acquire(&spec.id)?;

            // Execute
            let result = execute_spec(&spec);

            // Release lock
            daemon.lock_release(&spec.id)?;

            // Report result
            daemon.report_completion(&spec.id, result)?;
        } else {
            // No work, wait
            sleep(Duration::from_secs(5));
        }
    }
}
```

**Built-in feature.** K8s scales the workers.

### Pattern 3: Grafana Dashboard

Chant exposes metrics. Grafana visualizes.

```yaml
# grafana-dashboard.json (simplified)
{
  "panels": [
    {
      "title": "Tasks by Status",
      "targets": [{
        "expr": "chant_specs_total"
      }]
    },
    {
      "title": "Active Agents",
      "targets": [{
        "expr": "chant_agents_active"
      }]
    },
    {
      "title": "Completion Rate",
      "targets": [{
        "expr": "rate(chant_specs_total{status='completed'}[5m])"
      }]
    }
  ]
}
```

Grafana is deployment config, not built into Chant.

### Pattern 4: Sparse Checkout Worktrees

For monorepos, agents use sparse checkout:

```bash
# Worker setup (once per agent pod)
git worktree add --sparse /workdir
cd /workdir
git sparse-checkout init --cone
```

```bash
# Per-spec (agent worker does this)
git sparse-checkout set packages/$PROJECT
chant work $TASK_ID
```

**Built-in support:** `chant work` can manage sparse checkout:

```yaml
# config.md
scale:
  worktree:
    sparse: true
    pattern: "packages/{{project}}/"
```

## What Chant Builds vs What You Deploy

| Component | Chant Builds | You Deploy |
|-----------|--------------|------------|
| Spec format | ✓ Markdown spec | |
| Daemon | ✓ Binary | K8s pod |
| Metrics | ✓ Prometheus format | Grafana |
| Lock API | ✓ Unix socket/HTTP | |
| Queue API | ✓ Primitives | |
| Worker mode | ✓ `chant agent worker` | K8s replicas |
| Orchestration | | K8s / custom |
| Dashboard | | Grafana |
| Alerting | | PagerDuty / etc |

## Minimal Scale Config

```yaml
# config.md for scale deployment
---
project:
  name: monorepo

scale:
  id_prefix:
    from: path
    pattern: "packages/([^/]+)/"

  daemon:
    enabled: true
    metrics_port: 9090
    api_port: 8080

  worktree:
    sparse: true
    pool_size: 10

  limits:
    max_agents: 100
    max_per_project: 10
    spec_timeout: 30m
---

# Monorepo Scale Configuration

This repo uses Chant at scale with:
- K8s deployment for agents
- Grafana for monitoring
- Project-prefixed spec IDs
```

## Summary

**Chant at scale = built-in primitives + deployment patterns**

Built-in:
- Project-prefixed IDs
- Daemon mode (indexes, locks, queue)
- Search infrastructure (Tantivy + arroy, both memory-mapped)
- Metrics endpoint
- Worker mode
- Sparse worktree support

All Rust-native. No SQLite. No external services.

| Component | Library | Storage | Scale |
|-----------|---------|---------|-------|
| Keyword search | Tantivy | Memory-mapped | Millions |
| Semantic search | arroy | LMDB (memory-mapped) | Millions |
| Embeddings | fastembed-rs | N/A | Local, ~200MB RAM |

Deployment (you provide):
- K8s for orchestration
- Grafana for dashboards
- Alerting integration
- Custom automation

Chant stays focused. Infrastructure does infrastructure.
