# Metrics

> **Status: Not Implemented** ❌
>
> Prometheus metrics require the Daemon to be implemented. Since daemon mode is not yet available,
> metrics are not currently available. See [Roadmap](../roadmap/roadmap.md) - Phase 6 for implementation plans.

## Overview

Daemon exposes Prometheus metrics for monitoring and Grafana dashboards.

```bash
chant daemon start --metrics-port 9090
curl http://localhost:9090/metrics
```

## Metrics Catalog

### Spec Metrics

```prometheus
# Total specs by status
chant_specs_total{status="pending"} 142
chant_specs_total{status="in_progress"} 8
chant_specs_total{status="completed"} 1847
chant_specs_total{status="failed"} 12

# Specs by project (scale deployments)
chant_specs_total{status="pending",project="auth"} 23
chant_specs_total{status="pending",project="payments"} 45

# Ready specs (pending with deps satisfied)
chant_specs_ready_total 89
chant_specs_ready_total{project="auth"} 15
```

### Agent Metrics

```prometheus
# Currently active agents
chant_agents_active 8
chant_agents_active{project="auth"} 3

# Agent completions
chant_agent_completions_total 1523
chant_agent_completions_total{result="success"} 1498
chant_agent_completions_total{result="failure"} 25
```

### Duration Metrics

```prometheus
# Spec duration histogram (seconds)
chant_spec_duration_seconds_bucket{le="60"} 450
chant_spec_duration_seconds_bucket{le="300"} 1200
chant_spec_duration_seconds_bucket{le="900"} 1450
chant_spec_duration_seconds_bucket{le="1800"} 1495
chant_spec_duration_seconds_bucket{le="+Inf"} 1523
chant_spec_duration_seconds_sum 425000
chant_spec_duration_seconds_count 1523
```

### Queue Metrics

```prometheus
# Queue depth
chant_queue_depth 42
chant_queue_depth{project="auth"} 12

# Queue wait time (seconds spec has been ready)
chant_queue_wait_seconds_bucket{le="60"} 30
chant_queue_wait_seconds_bucket{le="300"} 38
chant_queue_wait_seconds_bucket{le="+Inf"} 42
```

### Lock Metrics

```prometheus
# Active locks
chant_locks_active 8

# Lock acquisitions
chant_lock_acquisitions_total 2500
chant_lock_releases_total 2492
chant_lock_conflicts_total 23   # Acquisition failures
```

### Daemon Metrics

```prometheus
# Daemon uptime
chant_daemon_uptime_seconds 9240

# Index stats
chant_index_documents_total 2009
chant_index_updates_total 15234
chant_index_queries_total 89234
```

## Grafana Dashboard

Example dashboard panels:

### Spec Overview

```json
{
  "title": "Specs by Status",
  "type": "piechart",
  "targets": [{
    "expr": "chant_specs_total",
    "legendFormat": "{{status}}"
  }]
}
```

### Completion Rate

```json
{
  "title": "Completions per Hour",
  "type": "graph",
  "targets": [{
    "expr": "rate(chant_agent_completions_total{result='success'}[1h]) * 3600",
    "legendFormat": "Completed"
  }]
}
```

### Agent Activity

```json
{
  "title": "Active Agents",
  "type": "gauge",
  "targets": [{
    "expr": "chant_agents_active"
  }],
  "thresholds": [
    { "value": 0, "color": "red" },
    { "value": 1, "color": "yellow" },
    { "value": 5, "color": "green" }
  ]
}
```

### Spec Duration

```json
{
  "title": "Spec Duration (p95)",
  "type": "stat",
  "targets": [{
    "expr": "histogram_quantile(0.95, rate(chant_spec_duration_seconds_bucket[1h]))"
  }]
}
```

## Alerting Examples

Prometheus alerting rules:

```yaml
groups:
  - name: chant
    rules:
      - alert: ChantQueueBacklog
        expr: chant_queue_depth > 100
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Chant queue backlog"
          description: "{{ $value }} specs waiting"

      - alert: ChantHighFailureRate
        expr: rate(chant_agent_completions_total{result="failure"}[1h]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High spec failure rate"

      - alert: ChantDaemonDown
        expr: up{job="chant"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Chant daemon is down"
```

## Configuration

```yaml
# config.md
scale:
  daemon:
    metrics_port: 9090      # 0 to disable
    metrics_path: /metrics  # Endpoint path
```

## Labels

All metrics support filtering by:

| Label | Description |
|-------|-------------|
| `status` | Spec status (pending, in_progress, etc.) |
| `project` | Project prefix (scale deployments) |
| `result` | Completion result (success, failure) |

## Cardinality

Metrics are low-cardinality by design:
- No per-spec metrics (would explode)
- Project label only at scale (configured)
- Status is finite set
