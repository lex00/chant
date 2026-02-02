# Roadmap

## Planned Releases

| Version | Focus |
|---------|-------|
| v0.4.0 | Full-text search (Tantivy indexing) |
| v0.5.0 | Daemon mode for background execution |
| v1.0.0 | Stable API, complete documentation |

## Planned Features

### Scale

- **Daemon mode** — Background service for persistent indexing, lock management, and worker coordination. See [Daemon Mode](planned/daemon.md).
- **Tantivy full-text search** — Fast indexed search across all specs. Requires daemon for index maintenance.
- **Queue architecture (advanced tiers)** — PostgreSQL and Redis-backed queue backends for large deployments. See [Queue Architecture](../scale/queue.md).

### Automation

- **Hooks** — Pre/post-execution hooks for custom workflows. See [Hooks](planned/hooks.md).
- **Triggers** — Event-based spec activation (file changes, schedules, webhooks). See [Triggers](planned/triggers.md).

### Observability

- **Prometheus metrics** — Metrics endpoint exposed by daemon. See [Metrics](planned/metrics.md).
- **Cost tracking** — LLM API cost monitoring and budgets. See [Cost Tracking](planned/costs.md).
- **Notifications** — Webhooks, email, and Slack integration. See [Notifications](planned/notifications.md).

### Ecosystem

- **Prompt registry** — Community and private prompt sharing.
- **Template/prompt registry** — Package management for prompt collections.

## Future Considerations

### Prompt Ecosystem

Open source prompt sharing:
- Community registry (GitHub-based)
- Domain-specific collections (security, TDD, docs)
- Framework-specific prompts (React, Rails, etc.)
- Model-specific optimizations

Commercial layer (potential):
- Curated enterprise prompts
- Private team registries
- Usage analytics

### Wetwire Integration

Type-safe config generation from Go structs (internal tool: `wetwire`).

**What wetwire generates:**
- GitHub Actions / GitLab CI
- Prometheus / Alertmanager rules
- Grafana dashboards
- Kubernetes manifests
- Cloud provider configs (AWS, GCP, Azure)

**Chant use cases:**
- `chant init --github` → generates `.github/workflows/chant.yml`
- `chant init --gitlab` → generates `.gitlab-ci.yml`
- `chant init --k8s` → generates daemon deployment manifests
- `chant init --prometheus` → generates alerting rules
- `chant init --grafana` → generates dashboard JSON

Deferred until core CLI stable.

## Non-Goals

Will not build:
- Web UI / dashboard (use Grafana)
- Team collaboration features (use GitHub)
- Time tracking
- Sprint planning
- External issue tracker integration (maybe plugins later)

Chant is a developer tool, not a project management platform.

## Success Metrics

| Metric | Target |
|--------|--------|
| Dogfooding | 100% (use chant to build chant) |
| Test coverage | 80% core, 90% parser |
| CLI response | <100ms (no daemon) |
| Search response | <10ms (with daemon) |
| Documentation | Complete before 1.0 |

## Current Implementation Status

For a comprehensive list of implemented features and their documentation, see [Feature Status](../FEATURE_STATUS.md).
