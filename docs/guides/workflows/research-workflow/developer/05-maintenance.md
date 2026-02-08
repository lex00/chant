# Week 4+: Maintenance

With the POC complete, Alex sets up ongoing maintenance. This phase shows how drift detection keeps documentation and analysis current as the codebase evolves.

## The Maintenance Pattern

Code changes constantly. Documentation and analysis must keep up:

```
┌──────────────────────────────────────────────────────────────────┐
│                    Drift Detection Types                          │
└──────────────────────────────────────────────────────────────────┘

     tracks:                  origin:                  informed_by:
   ┌──────────┐            ┌──────────────┐         ┌──────────────┐
   │  Source  │            │   Metrics    │         │   Analysis   │
   │   Code   │            │    Data      │         │   Results    │
   └────┬─────┘            └──────┬───────┘         └──────┬───────┘
        │                         │                        │
        ▼                         ▼                        ▼
   ┌──────────┐            ┌──────────────┐         ┌──────────────┐
   │  Code    │            │ New Metrics  │         │  Upstream    │
   │ Changes  │            │  Collected   │         │  Findings    │
   │          │            │              │         │   Updated    │
   └────┬─────┘            └──────┬───────┘         └──────┬───────┘
        │                         │                        │
        └─────────────────────────┴────────────────────────┘
                                  │
                                  ▼
                       ┌─────────────────────┐
                       │    chant drift      │
                       │  Detects changes,   │
                       │  suggests re-run    │
                       └─────────────────────┘
```

## Three Types of Drift

### 1. Documentation Drift (tracks:)

When tracked source code changes, documentation becomes stale.

**Scenario:** A developer adds a new endpoint to the reporting service:

```python
# services/reporting/src/routes.py - new endpoint added
@router.get("/reports/realtime")
async def get_realtime_report(request: Request):
    """New endpoint not in docs"""
    ...
```

Running drift detection:

```bash
$ chant drift 001-doc
```

```
Drift Report
============

Spec: 2026-01-28-001-doc (Document system architecture)
Status: DRIFT DETECTED

Tracked files changed since completion (2026-01-28):
  - services/reporting/src/routes.py
      Modified: 2026-03-10
      Lines changed: +35, -0
      New function: get_realtime_report

Documentation may be outdated for:
  - docs/architecture/modules.md (Reporting Module section)

Recommendation: Re-run spec to update documentation
  $ chant reset 001-doc && chant work 001-doc
```

### 2. Analysis Drift (origin:)

When origin data files change, analysis may be outdated.

**Scenario:** New production metrics collected:

```bash
# Updated metrics file
.chant/context/migration-research/production-metrics.md (modified)
```

Running drift detection:

```bash
$ chant drift 001-cpl
```

```
Drift Report
============

Spec: 2026-01-27-001-cpl (Analyze codebase coupling)
Status: DRIFT DETECTED

Informed-by files changed since completion (2026-01-27):
  - .chant/context/migration-research/production-metrics.md
      Modified: 2026-03-15
      Content: New Q1 metrics added

Analysis may need updating for:
  - analysis/coupling/extraction-candidates.md (rankings may shift)

Recommendation: Re-run spec with updated metrics
  $ chant reset 001-cpl && chant work 001-cpl
```

### 3. Cascade Drift (depends_on:)

When upstream specs drift, downstream specs may also be affected.

```bash
$ chant drift --all
```

```
Drift Report
============

Cascade detected:

2026-01-27-001-cpl (coupling analysis)
  └─ DRIFT: production-metrics.md changed
      │
      ├─> 2026-01-28-001-doc (documentation)
      │     Status: CASCADE DRIFT (upstream stale)
      │
      └─> 2026-02-01-001-ext (extraction)
            Status: CASCADE DRIFT (upstream stale)

Recommendation: Re-run upstream spec first
  1. `chant reset 001-cpl && chant work 001-cpl` — Refresh coupling analysis
  2. `chant reset 001-doc && chant work 001-doc` — Update architecture docs
```

## Periodic Drift Monitoring

Alex runs drift checks periodically to catch stale specs:

```bash
$ chant drift
```

This shows all specs with detected drift, helping decide which to re-run.

## Selective Re-runs

Alex can re-run just what's needed:

```bash
$ chant reset 001-doc
$ chant work 001-doc
```

The agent regenerates documentation incorporating recent code changes.

## Weekly Coupling Reports

For ongoing monitoring, Alex creates a scheduled research spec:

**File: `.chant/specs/2026-03-01-001-wkly.md`**

```yaml
---
type: research
status: pending
schedule: weekly
informed_by:
  - src/**/*.py
origin:
  - .chant/context/migration-research/production-metrics.md
target_files:
  - reports/weekly/coupling-status.md
---
```

```markdown
# Weekly coupling status

## Purpose

Track coupling changes week-over-week as the codebase evolves.

## Methodology

1. Recalculate import dependency matrix
2. Compare to previous week
3. Flag any new circular dependencies
4. Track progress on extraction candidates

## Output Format

```markdown
# Coupling Status - Week of [DATE]

## Changes Since Last Week
- New dependencies: N
- Removed dependencies: N
- New circular: Y/N

## Extraction Progress
| Candidate | Last Week Score | This Week | Trend |
|-----------|-----------------|-----------|-------|
| reporting | 8.5 | 8.7 | ↑ |
| notifications | 7.2 | 7.2 | → |
| billing | 5.8 | 5.6 | ↓ |

## Alerts
[Any coupling concerns]
```

## Acceptance Criteria

- [ ] Dependency matrix recalculated
- [ ] Comparison to previous week
- [ ] Extraction scores updated
- [ ] Report generated
```

## Metrics Dashboard Integration

Alex integrates drift status with the team dashboard:

**File: `.chant/context/migration-research/drift-status.md`** (auto-updated)

```markdown
# Drift Status Dashboard

Last updated: 2026-03-18T09:00:00Z

## Spec Health

| Spec | Type | Last Run | Status | Days Since |
|------|------|----------|--------|------------|
| 001-cpl | research | 2026-01-27 | DRIFT | 50 |
| 001-doc | documentation | 2026-01-28 | DRIFT | 49 |
| 001-ext | driver | 2026-02-01 | OK | 45 |
| 001-wkly | research | 2026-03-18 | OK | 0 |

## Tracked File Changes

| Path | Last Modified | Tracked By | Needs Replay |
|------|--------------|------------|--------------|
| src/auth/routes.py | 2026-03-10 | 001-doc | Yes |
| src/reporting/routes.py | 2026-03-08 | 001-doc | Yes |
| production-metrics.md | 2026-03-15 | 001-cpl | Yes |

## Recommended Actions

1. `chant replay 001-cpl` — Refresh coupling analysis
2. `chant replay 001-doc` — Update architecture docs
```

## Maintenance Benefits

| Benefit | How It Helps |
|---------|-------------|
| **Proactive alerts** | Know when docs are stale before customers complain |
| **Traceable changes** | See exactly which file changes caused drift |
| **Cascade awareness** | Understand how changes ripple through specs |
| **Automated monitoring** | Weekly reports without manual checking |

## Complete Workflow Summary

Alex's research workflow used all spec types:

```
Week 1: Investigation
  └─ research spec (informed_by: src/**) → Coupling analysis

Week 2: Documentation
  └─ documentation spec (tracks: src/**) → Architecture docs

Week 3: Implementation
  └─ driver spec (members: .1-.4) → POC extraction

Week 4+: Maintenance
  └─ scheduled specs + drift detection → Ongoing updates
```

This completes the developer path. See the [artifacts](artifacts/) directory for complete spec examples.

## What's Next

Return to the main guide:

**[Research Workflow Overview](../README.md)** — Compare both paths and see the universal pattern
