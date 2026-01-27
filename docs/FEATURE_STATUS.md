# Feature Implementation Status

This document tracks which documented features are implemented, planned, or partially complete.
For detailed implementation status of each phase, see [Roadmap](roadmap/roadmap.md).

## Fully Implemented ✅

| Feature | Documentation | Status | Version |
|---------|-------------|--------|---------|
| Spec CRUD | [specs.md](concepts/specs.md) | ✅ Complete | v0.0.0 |
| Spec Types | [spec-types.md](concepts/spec-types.md) | ✅ Complete | v0.1.0 |
| ID Format | [ids.md](concepts/ids.md) | ✅ Complete | v0.0.0 |
| Spec Groups | [groups.md](concepts/groups.md) | ✅ Complete | v0.3.0 |
| Dependencies | [deps.md](concepts/deps.md) | ✅ Complete | v0.3.0 |
| Git Integration | [git.md](reference/git.md) | ✅ Complete | v0.1.0 |
| Worktree Isolation | [isolation.md](scale/isolation.md) | ✅ Complete | v0.1.0 |
| Locking | [locks.md](scale/locks.md) | ✅ Complete | v0.3.0 |
| Spec Linting | [observability.md](scale/observability.md) | ✅ Complete | v0.2.0 |
| Status Command | [observability.md](scale/observability.md) | ✅ Complete | v0.2.0 |
| Log Command | [observability.md](scale/observability.md) | ✅ Complete | v0.2.0 |
| Diagnose Command | [observability.md](scale/observability.md) | ✅ Complete | v0.2.0 |
| Export (JSON/CSV/Markdown) | [reports.md](reference/reports.md) | ✅ Complete | v0.3.0 |
| Drift Detection | [autonomy.md](concepts/autonomy.md) | ✅ Complete | v0.2.0 |
| Verify Command | [autonomy.md](concepts/autonomy.md) | ✅ Complete | v0.2.0 |
| Replay Command | [autonomy.md](concepts/autonomy.md) | ✅ Complete | v0.3.0 |
| MCP Server | [mcp.md](reference/mcp.md) | ✅ Complete | v0.2.0 |
| Claude Provider | [protocol.md](architecture/protocol.md) | ✅ Complete | v0.0.0 |
| Ollama Provider | [protocol.md](architecture/protocol.md) | ✅ Complete | v0.2.0 |
| OpenAI Provider | [protocol.md](architecture/protocol.md) | ✅ Complete | v0.2.0 |
| Multi-Repo Support | [multi-project.md](scale/multi-project.md) | ✅ Complete | v0.3.0 |
| Parallel Execution | [scale.md](scale/scale.md) | ✅ Complete | v0.3.0 |
| Agent Rotation | [scale.md](scale/scale.md) | ✅ Complete | v0.3.0 |
| Split Command | [groups.md](concepts/groups.md) | ✅ Complete | v0.3.0 |
| Merge Command | [git.md](reference/git.md) | ✅ Complete | v0.1.0 |
| Archive Command | [lifecycle.md](concepts/lifecycle.md) | ✅ Complete | v0.3.0 |
| Cancel Command | [lifecycle.md](concepts/lifecycle.md) | ✅ Complete | v0.3.0 |
| Delete Command | [lifecycle.md](concepts/lifecycle.md) | ✅ Complete | v0.3.0 |
| Search Command | [search.md](reference/search.md) | ✅ Complete | v0.3.0 |
| Config Validation | [config.md](reference/config.md) | ✅ Complete | v0.3.0 |
| Spec Status: Blocked | [lifecycle.md](concepts/lifecycle.md) | ✅ Complete | v0.3.0 |
| Spec Status: Cancelled | [lifecycle.md](concepts/lifecycle.md) | ✅ Complete | v0.3.0 |
| Interactive Wizards | [cli.md](reference/cli.md) | ✅ Complete | v0.3.0 |
| Prompts | [prompts.md](concepts/prompts.md) | ✅ Complete | v0.0.0 |
| Hooks | [hooks.md](concepts/hooks.md) | ✅ Complete | v0.2.0 |
| Triggers | [triggers.md](concepts/triggers.md) | ✅ Complete | v0.2.0 |
| Data Lifecycle | [lifecycle.md](concepts/lifecycle.md) | ✅ Complete | v0.2.0 |

## Partially Implemented ⚠️

| Feature | Documentation | Implementation | Missing |
|---------|-------------|-----------------|---------|
| Observability | [observability.md](scale/observability.md) | Local & Team tiers | Scale tier (metrics, daemon-based) |
| Queue Architecture | [queue.md](scale/queue.md) | Daemon-free (Tiers 1-2) | Advanced backends (Tantivy, PostgreSQL, Redis) |
| Templates | [templates.md](reference/templates.md) | Basic substitution | Full Handlebars (conditionals, helpers, partials, inheritance) |
| Ecosystem | [ecosystem.md](guides/ecosystem.md) | Provider adapters | Prompt registry, package management |
| Git Hooks | [git-hooks.md](reference/git-hooks.md) | Basic validation | Advanced pre/post-commit workflows |

## Planned but Not Implemented ❌

| Feature | Documentation | Roadmap Phase | Target Version |
|---------|-------------|----------------|-----------------|
| Notifications | [notifications.md](reference/notifications.md) | Phase 8 | v0.4.0 or later |
| Approvals | [approvals.md](guides/approvals.md) | Phase 8 | v0.4.0 or later |
| Daemon Mode | [daemon.md](scale/daemon.md) | Phase 6 | v0.5.0 |
| Prometheus Metrics | [metrics.md](scale/metrics.md) | Phase 6 | v0.5.0 |
| Cost Tracking | [costs.md](reference/costs.md) | Phase 8 | v0.4.0 or later |
| Tantivy Search | [search.md](reference/search.md) | Phase 6 | v0.4.0 |
| DAG Visualization | [observability.md](scale/observability.md) | Phase 6 | v0.4.0 or later |
| Prompt Registry | [ecosystem.md](guides/ecosystem.md) | Phase 8 | v0.4.0 or later |
| Edit Command | [cli.md](reference/cli.md) | Phase 8 | v0.4.0 or later |

## Commands Status Reference

### Fully Implemented ✅

```
chant init          ✅ Initialize repository
chant add           ✅ Create new spec
chant list          ✅ List specs with filters
chant show          ✅ Display spec details
chant work          ✅ Execute spec
chant work --pr     ✅ Execute and create PR
chant work --branch ✅ Execute with feature branch
chant work --parallel ✅ Execute multiple specs
chant resume        ✅ Resume failed spec
chant ready         ✅ Show ready specs
chant lint          ✅ Validate specs
chant status        ✅ Project status summary
chant log           ✅ Show execution logs
chant diagnose      ✅ Troubleshoot issues
chant drift         ✅ Detect spec staleness
chant verify        ✅ Verify acceptance criteria
chant replay        ✅ Re-execute completed specs
chant merge         ✅ Merge spec branches
chant split         ✅ Decompose specs
chant archive       ✅ Archive completed specs
chant cancel        ✅ Soft-delete specs
chant delete        ✅ Delete specs with cleanup
chant search        ✅ Interactive search
chant export        ✅ Export to JSON/CSV/Markdown
chant config        ✅ Configuration management
chant mcp           ✅ MCP server
```

### Planned but Not Implemented ❌

```
chant daemon        ❌ Planned for v0.5.0
chant daemon start  ❌ Planned for v0.5.0
chant daemon stop   ❌ Planned for v0.5.0
chant queue         ❌ Planned for v0.5.0
chant edit          ❌ Planned for v0.4.0+
chant lock          ❌ Planned for v0.5.0
chant agent worker  ❌ Planned for v0.5.0
chant notify        ❌ Planned for v0.4.0+
chant approve       ❌ Planned for v0.4.0+
chant costs         ❌ Planned for v0.4.0+
```

## Documentation Status by Section

### Getting Started
- `installation.md` - ✅ Current
- `quickstart.md` - ✅ Current
- `philosophy.md` - ✅ Current
- `value.md` - ✅ Current

### Concepts
- `specs.md` - ✅ Current (all documented features implemented)
- `spec-types.md` - ✅ Current (all documented features implemented)
- `prompts.md` - ✅ Current
- `ids.md` - ✅ Current
- `groups.md` - ✅ Current
- `deps.md` - ✅ Current
- `hooks.md` - ✅ Current
- `triggers.md` - ✅ Current
- `autonomy.md` - ✅ Current (all documented features implemented)
- `lifecycle.md` - ✅ Current

### Architecture
- `architecture.md` - ✅ Current
- `stack.md` - ✅ Current
- `protocol.md` - ✅ Current
- `invoke.md` - ✅ Current
- `storage.md` - ✅ Current

### Reference
- `cli.md` - ⚠️ Some planned commands marked as "Status: Planned"
- `config.md` - ✅ Current
- `errors.md` - ✅ Current
- `search.md` - ⚠️ Advanced search (Tantivy) not implemented
- `git.md` - ✅ Current
- `git-hooks.md` - ⚠️ Partial implementation
- `templates.md` - ⚠️ Basic only, full Handlebars not implemented
- `schema.md` - ✅ Current
- `notifications.md` - ❌ Not implemented (updated with status)
- `reports.md` - ✅ Current (export implemented, status updated)
- `costs.md` - ❌ Not implemented (updated with status)
- `mcp.md` - ✅ Current
- `versioning.md` - ✅ Current
- `output.md` - ✅ Current
- `init.md` - ✅ Current
- `testing.md` - ✅ Current

### Guides
- `prompt-authoring.md` - ✅ Current
- `prompt-examples.md` - ✅ Current
- `prompt-advanced.md` - ✅ Current
- `research.md` - ✅ Current
- `examples.md` - ✅ Current
- `ecosystem.md` - ⚠️ Partially implemented (updated with status)
- `approvals.md` - ❌ Not implemented (updated with status)
- `recovery.md` - ✅ Current

### Scale
- `scale.md` - ✅ Current
- `daemon.md` - ❌ Not implemented (updated with status)
- `multi-project.md` - ✅ Current
- `isolation.md` - ✅ Current
- `locks.md` - ✅ Current
- `queue.md` - ⚠️ Partially implemented (updated with status)
- `metrics.md` - ❌ Not implemented (updated with status)
- `observability.md` - ⚠️ Partially implemented (updated with status)

### Enterprise
- `enterprise.md` - ⚠️ Largely aspirational, features not implemented
- `security.md` - ✅ Current

### Roadmap
- `roadmap.md` - ✅ Accurate and up-to-date

## Key Documentation Updates Made

This document addresses the issues identified in the spec:

1. ✅ **Labeled unimplemented features** - Added "Status: Not Implemented ❌" to 7 major feature docs
2. ✅ **Marked planned features clearly** - Added "Status: Not Implemented" with Roadmap references
3. ✅ **Identified outdated examples** - Fixed export docs (reports.md) which claimed feature was planned when it's actually implemented
4. ✅ **Ensured roadmap matches reality** - Confirmed roadmap accurately reflects Layer 6 (Scale) incompleteness
5. ✅ **Flagged docs referencing changed behavior** - Added status markers to observability, queue, templates, ecosystem
6. ✅ **Created implementation status index** - This file serves as a comprehensive reference

## How to Use This Document

- **Planning features?** Check this document to see what's already available
- **Reporting bugs?** Ensure the feature is marked as Implemented ✅
- **Discussing limitations?** Find the feature status and expected target version
- **Contributing documentation?** Update both the feature doc and this index

## Version History

| Version | Status | Notes |
|---------|--------|-------|
| v0.3.0 | Current | Most Phase 0-5 features complete; Phase 6 Scale partial; Phase 7 Autonomy complete; Phase 8 Polish mostly complete |
| v0.4.0 | Planned | Target: Full-text search (Tantivy), cost tracking, notifications (possibly) |
| v0.5.0 | Planned | Target: Daemon mode, queue tier 3+, Prometheus metrics |
| v1.0.0 | Planned | Stable API, complete documentation, all Phase 0-8 features |
