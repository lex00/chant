# Week 4+: Drift Detection

Research doesn't end when analysis completes. New data arrives, papers are published, and findings may become stale. This phase shows how chant detects drift and triggers re-verification.

## The Maintenance Pattern

Drift detection monitors three types of changes:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Drift Detection Types                         │
└─────────────────────────────────────────────────────────────────┘

     origin:                  informed_by:              tracks:
   ┌──────────┐             ┌──────────────┐         ┌──────────┐
   │  Data    │             │   Sources    │         │  Code    │
   │  Files   │             │  (Papers)    │         │ (Scripts)│
   └────┬─────┘             └──────┬───────┘         └────┬─────┘
        │                          │                      │
        ▼                          ▼                      ▼
   ┌──────────┐             ┌──────────────┐         ┌──────────┐
   │  New     │             │   Updated    │         │   Doc    │
   │  Data    │             │   Research   │         │ Changes  │
   │  Rows    │             │              │         │          │
   └────┬─────┘             └──────┬───────┘         └────┬─────┘
        │                          │                      │
        └──────────────────────────┴──────────────────────┘
                                   │
                                   ▼
                        ┌─────────────────────┐
                        │   chant drift       │
                        │  Detects changes,   │
                        │  suggests re-run    │
                        └─────────────────────┘
```

## Scenario: New Data Arrives

Two months after completing her analysis, Sarah receives updated temperature data. The monitoring stations added January and February 2025 observations.

```bash
# New data added to existing files
data/temperature/barrow-alaska.csv       # +59 rows
data/temperature/resolute-canada.csv     # +59 rows
# ... 10 more stations
```

Sarah runs drift detection:

```bash
$ chant drift
```

Output:

```
Drift Report
============

Spec: 2026-01-16-001-ana (Analyze Arctic temperature acceleration)
Status: DRIFT DETECTED

Origin files changed since completion (2026-01-16):
  - data/temperature/barrow-alaska.csv
      Modified: 2026-03-15
      Rows added: 59
  - data/temperature/resolute-canada.csv
      Modified: 2026-03-15
      Rows added: 59
  [... 10 more files ...]

Total new observations: 708

Recommendation: Re-run spec to incorporate new data
  $ chant reset 001-ana && chant work 001-ana

---

Spec: 2026-01-20-001-drv (Arctic analysis pipeline)
Status: DRIFT DETECTED (via member)

Member 2026-01-20-001-drv.1 has upstream drift
Cascade: drv.1 → drv.2 → drv.3, drv.4

Recommendation: Re-run entire pipeline
  $ chant reset 001-drv && chant work 001-drv
```

## Re-running Analysis

Sarah re-runs the pipeline to incorporate new data:

```bash
$ chant reset 001-drv
$ chant work 001-drv
```

The agent re-executes the pipeline with the new data, updating all analysis outputs.

## Comparison Report

After replay, chant generates a comparison:

**File: `analysis/replay-comparison.md`** (generated)

```markdown
# Replay Comparison Report

## Summary

| Metric | Original (Jan 2026) | Updated (Mar 2026) | Change |
|--------|---------------------|---------------------|--------|
| Observations | 131,484 | 132,180 | +696 |
| Data end date | 2024-12-31 | 2025-02-28 | +59 days |

## Key Finding Changes

### Overall Trend

| Metric | Original | Updated | Change |
|--------|----------|---------|--------|
| Warming rate | +0.67°C/decade | +0.69°C/decade | +0.02 |
| Acceleration p-value | 0.003 | 0.002 | More significant |
| Breakpoint | 2005 | 2005 | Unchanged |

Interpretation: Additional data strengthens the acceleration finding.

### Station-Level Changes

| Station | Original Trend | Updated Trend | Change |
|---------|---------------|---------------|--------|
| Barrow | +0.78 | +0.81 | +0.03 |
| Tiksi | +0.92 | +0.94 | +0.02 |
| [others] | ... | ... | ... |

### Seasonal Pattern

Winter 2025 data shows continued strong warming:
- DJF 2024-2025 anomaly: +2.3°C (above 1994-2000 baseline)
- Consistent with acceleration trend

## Conclusion

New data strengthens original findings. No methodological changes required.
```

## Scenario: New Paper Published

A month later, a relevant paper is published. Sarah adds it to her collection:

```bash
# New paper added
papers/acceleration/petrov-2026-siberian-warming.pdf
```

She updates the paper index and runs drift:

```bash
$ chant drift 001-lit
```

```
Drift Report
============

Spec: 2026-01-15-001-lit (Synthesize Arctic warming literature)
Status: DRIFT DETECTED

Informed-by files changed since completion (2026-01-15):
  - papers/acceleration/petrov-2026-siberian-warming.pdf
      Added: 2026-04-10
      Status: New file

Recommendation: Re-run spec to incorporate new paper
  $ chant reset 001-lit && chant work 001-lit
```

## Re-running Selective Analysis

Sarah can re-run just the affected spec:

```bash
$ chant reset 001-lit
$ chant work 001-lit
```

The agent re-synthesizes the literature review incorporating the new paper.

## Periodic Drift Checks

Sarah runs drift detection periodically to check for stale findings:

```bash
$ chant drift
```

This shows all specs with detected drift, allowing her to decide which to re-run.

## Drift Types Summary

| Drift Type | Trigger | Example | Response |
|------------|---------|---------|----------|
| **origin** | Data files change | New temperature readings | Re-run analysis |
| **informed_by** | Source materials change | New paper published | Re-run synthesis |
| **tracks** | Tracked code changes | Analysis script updated | Re-run documentation |

## Recurring Research

For ongoing monitoring, Sarah documents the intended cadence:

**File: `.chant/specs/2026-03-01-001-mon.md`**

```yaml
---
type: research
status: pending
schedule: monthly
origin:
  - data/temperature/*.csv
target_files:
  - reports/monthly/temperature-update.md
---
```

```markdown
# Monthly temperature update

Generate monthly update on Arctic temperature trends.

## Acceptance Criteria

- [ ] New data incorporated
- [ ] Trend recalculated
- [ ] Comparison to previous month
- [ ] Alert if significant change detected
```

The `schedule: monthly` field documents the intended recurrence. Sarah manually runs `chant work 001-mon` each month when new data arrives.

## Audit Trail

Git commits track the history of each spec execution:

```bash
$ git log --oneline --grep="001-ana"
```

```
abc1234 chant(001-ana): update analysis with May data
def5678 chant(001-ana): update analysis with March data
9ab0cde chant(001-ana): initial Arctic temperature analysis
```

Each re-run creates a new commit, preserving the full audit trail.

## Dissertation Defense

When Sarah defends her dissertation, she can demonstrate:

1. **Reproducibility**: Every analysis has a spec that can be re-run
2. **Provenance**: Every finding traces to specific data files and versions
3. **Currency**: Drift detection ensures findings reflect latest data
4. **Methodology**: Specs document the exact procedures used

Her advisor's reproducibility audit is straightforward:

```bash
# Show all specs with their completion dates
$ chant list

# Verify no drift in final results
$ chant drift

# Re-run entire pipeline to verify reproducibility
$ chant reset 001-drv && chant work 001-drv
```

## What's Next

This completes the academic path. See the [artifacts](artifacts/) directory for complete spec examples.

To explore the parallel developer path, start at:

**[Developer Scenario](../developer/01-scenario.md)** — Follow Alex Torres investigating a microservices migration
