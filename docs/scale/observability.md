# Observability

> **Status: Partially Implemented** ⚠️
>
> Local and Team observability tiers are fully implemented (`chant status`, `chant log`, `chant show`).
> Scale tier features (Prometheus metrics, daemon-based monitoring) are planned for future releases.
> See [Planned Features](../roadmap/planned/README.md) for details.

## Tiers of Observability

| Tier | Daemon | Tools | Use Case |
|------|--------|-------|----------|
| **Local** | No | CLI, spec files, git | Solo dev |
| **Team** | Optional | CLI, logs, git history | Small team |
| **Scale** | Yes | Prometheus, Grafana, logs | Org/Enterprise |

## Local Observability (No Daemon)

### Spec File is the Log

Agent output lives in the spec:

```markdown
---
status: completed
commit: abc123
duration_s: 145
---

# Add authentication

[spec description...]

## Output

```log
[2026-01-22 10:15:32] Reading src/auth/handler.go
[2026-01-22 10:15:33] Found 3 files to modify
[2026-01-22 10:15:45] Implementing JWT validation
[2026-01-22 10:16:12] Running tests: cargo test
[2026-01-22 10:16:45] All tests passed
[2026-01-22 10:16:50] Committing changes
```
```

**View with:**
```bash
chant show 001              # Full spec with output
chant show 001 --output     # Just the output section
chant logs 001              # Alias for --output
```

### Git History

Git tracks everything:

```bash
# Spec history
git log --oneline -- .chant/specs/001.md

# What changed
git diff HEAD~1 -- .chant/specs/001.md

# When was spec completed
git log -1 --format=%ci -- .chant/specs/001.md
```

### CLI Status

```bash
$ chant status
Tasks:
  pending:     12
  in_progress: 2
  completed:   145
  failed:      3

In Progress:
  001 - Add authentication (started 10m ago)
  002 - Fix payment bug (started 5m ago)

Recent Completions:
  003 - Update docs (2h ago, 3m duration)
  004 - Add tests (4h ago, 8m duration)
```

### Spec History

```bash
$ chant history 001
Spec: 001 - Add authentication

Timeline:
  2026-01-22 10:00  Created
  2026-01-22 10:15  Started (attempt 1)
  2026-01-22 10:20  Failed - tests failed
  2026-01-22 10:30  Started (attempt 2)
  2026-01-22 10:45  Completed

Commits:
  abc123  chant(001): implement JWT validation
  def456  chant(001): fix test failures

Duration: 45m total, 15m active
```

### Failure Analysis

```bash
$ chant failures
Recent failures:

001 - Add authentication
  Failed: 2026-01-22 10:20
  Error: Tests failed - 2 assertions
  Attempt: 1 of 2 (later succeeded)

005 - Refactor API
  Failed: 2026-01-22 09:00
  Error: Merge conflict
  Status: Unresolved

$ chant show 005 --error
Error: Merge conflict in src/api/handler.go

Agent output:
  [09:00:12] Changes complete
  [09:00:15] Attempting merge to main
  [09:00:16] CONFLICT: src/api/handler.go
  [09:00:16] Aborting - manual resolution required
```

## Team Observability

### Shared Logs

```yaml
# config.md
logging:
  level: info
  file: .chant/logs/chant.log
  format: json
```

```bash
# View recent activity
tail -f .chant/logs/chant.log | jq

# Filter by spec
grep '"spec_id":"001"' .chant/logs/chant.log | jq

# Filter by event
jq 'select(.event == "spec_completed")' .chant/logs/chant.log
```

### Log Levels

| Level | Content |
|-------|---------|
| `error` | Failures, crashes, unrecoverable |
| `warn` | Recoverable issues, deprecations |
| `info` | Spec starts/stops, state changes |
| `debug` | Detailed operations, file reads |

### Activity Report

```bash
$ chant report --last 7d
Weekly Report (2026-01-15 to 2026-01-22)

Tasks:
  Created:    47
  Completed:  42
  Failed:     5 (3 retried successfully)

By Project:
  auth:       12 completed
  payments:   18 completed
  api:        12 completed

By Person:
  alice:      15 tasks
  bob:        12 tasks
  carol:      15 tasks

Top Failures:
  - Test failures: 3
  - Merge conflicts: 2

Avg Duration: 12m
Total Agent Time: 8.4h
```

### Git-Based Analytics

```bash
# Commits by spec
git log --oneline --grep="^chant(" | head -20

# Tasks per day
git log --format=%ad --date=short -- .chant/specs/ | sort | uniq -c

# Who's working on what
git log --format="%an: %s" --since="1 week ago" -- .chant/specs/
```

## Scale Observability (Daemon)

### Prometheus Metrics

Daemon exposes `/metrics`:

```bash
chant daemon start --metrics-port 9090
curl http://localhost:9090/metrics
```

**Metrics exposed:**

```prometheus
# Spec counts
chant_specs_total{status="pending"} 42
chant_specs_total{status="in_progress"} 8
chant_specs_total{status="completed"} 1847
chant_specs_total{status="failed"} 23

# By project
chant_specs_total{status="pending",project="auth"} 12
chant_specs_total{status="pending",project="payments"} 30

# Agent activity
chant_agents_active 8
chant_agents_total 10

# Spec duration
chant_spec_duration_seconds_bucket{le="60"} 145
chant_spec_duration_seconds_bucket{le="300"} 520
chant_spec_duration_seconds_bucket{le="900"} 780
chant_spec_duration_seconds_sum 45000
chant_spec_duration_seconds_count 800

# Queue
chant_queue_depth 42
chant_queue_wait_seconds_bucket{le="60"} 30
chant_queue_wait_seconds_bucket{le="300"} 40

# Errors
chant_errors_total{type="agent_crash"} 5
chant_errors_total{type="merge_conflict"} 12
chant_errors_total{type="test_failure"} 45

# Cost tracking
chant_tokens_total{provider="default"} 1500000
chant_cost_usd_total{provider="default"} 45.00
```

### Grafana Dashboards

**Dashboard: Chant Overview**

```
┌─────────────────────────────────────────────────────────────┐
│  Specs by Status          │  Completion Rate (24h)         │
│  ┌─────┐ ┌─────┐ ┌─────┐ │  ████████████░░░ 85%           │
│  │ 42  │ │  8  │ │1847 │ │                                 │
│  │pend │ │ wip │ │done │ │  Failed: 15%                    │
│  └─────┘ └─────┘ └─────┘ │                                 │
├─────────────────────────────────────────────────────────────┤
│  Spec Duration (p50, p95, p99)                              │
│  ════════════════════════════════════════                   │
│  p50: 8m  │  p95: 25m  │  p99: 45m                         │
├─────────────────────────────────────────────────────────────┤
│  Active Agents            │  Queue Depth                    │
│       8 / 10              │      42 tasks                   │
│  ████████░░               │  ██████████████                 │
├─────────────────────────────────────────────────────────────┤
│  Errors (24h)                                               │
│  Test failures: 12  │  Merge conflicts: 3  │  Crashes: 1   │
└─────────────────────────────────────────────────────────────┘
```

**Dashboard: Cost Tracking**

```
┌─────────────────────────────────────────────────────────────┐
│  Daily Cost               │  By Provider                    │
│  $45.23                   │  Provider A: $40.00             │
│  ▁▂▃▄▅▆▇█                 │  Provider B: $5.23              │
├─────────────────────────────────────────────────────────────┤
│  Cost per Spec (avg)      │  Cost by Project                │
│  $0.52                    │  auth: $15.00                   │
│                           │  payments: $20.00               │
│                           │  api: $10.23                    │
└─────────────────────────────────────────────────────────────┘
```

### Alerting

```yaml
# Prometheus alerting rules
groups:
  - name: chant
    rules:
      - alert: ChantHighFailureRate
        expr: rate(chant_specs_total{status="failed"}[1h]) > 0.1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High spec failure rate"

      - alert: ChantQueueBacklog
        expr: chant_queue_depth > 100
        for: 30m
        labels:
          severity: warning
        annotations:
          summary: "Large queue backlog"

      - alert: ChantDaemonDown
        expr: up{job="chant"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Chant daemon is down"
```

### Structured Logging

```json
{"ts":"2026-01-22T10:15:30Z","level":"info","event":"task_started","task_id":"001","project":"auth","agent":"worker-1"}
{"ts":"2026-01-22T10:16:45Z","level":"info","event":"task_completed","task_id":"001","project":"auth","duration_s":75,"tokens":12500}
{"ts":"2026-01-22T10:17:00Z","level":"error","event":"task_failed","task_id":"002","project":"payments","error":"tests_failed","details":"2 assertions failed"}
```

**Standard fields:**

| Field | Description |
|-------|-------------|
| `ts` | ISO 8601 timestamp |
| `level` | Log level |
| `event` | Event type (task_started, task_completed, etc.) |
| `spec_id` | Spec ID (if applicable) |
| `project` | Project prefix (if applicable) |
| `worker_id` | Worker ID (scale deployments) |
| `duration_s` | Duration in seconds (for completions) |
| `error` | Error message (for failures) |

### Log Aggregation

Ship logs to your stack:

```yaml
# config.md
logging:
  format: json
  output: stdout             # For K8s log collection

# Or direct shipping
logging:
  ship_to:
    type: loki               # loki | elasticsearch | datadog
    url: http://loki:3100
    labels:
      app: chant
      env: production
```

## Debugging

### Debug Mode

```bash
# Verbose output
CHANT_LOG_LEVEL=debug chant work 001

# Trace agent interaction
chant work 001 --trace

# Dry run (show what would happen)
chant work 001 --dry-run

# Quiet mode (errors only, for scripting)
chant work 001 --quiet
```

### Inspect State

```bash
# Current locks
chant locks list

# Queue state
chant queue show

# Index health
chant index status

# Clone state
chant clones list
```

### Daemon Debug

```bash
# Daemon status
chant daemon status --verbose

# Daemon logs
chant daemon logs --follow

# Internal state dump
chant daemon debug --dump-state
```

## Configuration Reference

```yaml
# config.md
observability:
  # What goes in spec files
  spec_output:
    enabled: true
    max_lines: 1000            # Truncate long output

  # CLI verbosity
  cli:
    default_verbosity: normal  # quiet | normal | verbose

  # File logging
  logging:
    level: info                # debug | info | warn | error
    file: .chant/logs/chant.log
    format: json               # json | text
    rotate: daily              # daily | weekly | size
    retain: 30                 # Keep 30 rotations
    compress: true             # Gzip old logs
    max_size: 100M             # Per file (if rotate: size)

  # Metrics (daemon only)
  metrics:
    enabled: true
    port: 9090
    path: /metrics

  # Cost tracking
  costs:
    track: true
    warn_threshold: 10.00      # Warn if spec costs > $10
    budget:
      daily: 100.00
      monthly: 2000.00
```

---

**Note:** Scale tier observability features (Prometheus metrics, daemon-based monitoring, DAG visualization) are planned for future releases. See [Daemon Mode](../roadmap/planned/daemon.md) and [Metrics](../roadmap/planned/metrics.md) for details.
