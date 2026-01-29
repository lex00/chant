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
  $ chant replay 001-ana

---

Spec: 2026-01-20-001-drv (Arctic analysis pipeline)
Status: DRIFT DETECTED (via member)

Member 2026-01-20-001-drv.1 has upstream drift
Cascade: drv.1 → drv.2 → drv.3, drv.4

Recommendation: Re-run entire pipeline
  $ chant replay 001-drv
```

## Replaying Analysis

Sarah re-runs the pipeline to incorporate new data:

```bash
$ chant replay 001-drv
```

```
Replaying: 2026-01-20-001-drv (Arctic analysis pipeline)

Original completion: 2026-01-20
Data changed: 2026-03-15 (+708 observations)

Executing pipeline...

Phase 1: 2026-01-20-001-drv.1 (Data cleaning)
  New observations: 708
  Excluded by quality flags: 12
  Net added: 696

Phase 2: 2026-01-20-001-drv.2 (Statistical analysis)
  Comparing to original results...

Phase 3: 2026-01-20-001-drv.3, drv.4 (parallel)
  Sensitivity analysis updated
  Figures regenerated

Replay complete.
```

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
  $ chant replay 001-lit
```

## Selective Replay

Sarah can replay just the affected spec:

```bash
$ chant replay 001-lit
```

```
Replaying: 2026-01-15-001-lit (Synthesize Arctic warming literature)

Original completion: 2026-01-15
New paper: petrov-2026-siberian-warming.pdf

Agent synthesizing new paper with existing review...

Update to literature-review.md:
  Section 1.1: Added Petrov 2026 findings on Siberian amplification
  Table 1: Added new study (26th paper)

Update to research-gaps.md:
  Gap 1: Petrov 2026 partially addresses post-2015 acceleration
  Note: My analysis still provides station-level detail not in Petrov

Replay complete.
```

## Scheduled Drift Checks

Sarah configures automated drift detection:

**File: `.chant/config.md`**

```markdown
## schedule

### drift-check

- frequency: weekly
- notify: email
- auto-replay: false
```

Every Monday, chant checks for drift and emails Sarah if any specs are stale.

## Drift Types Summary

| Drift Type | Trigger | Example | Response |
|------------|---------|---------|----------|
| **origin** | Data files change | New temperature readings | Re-run analysis |
| **informed_by** | Source materials change | New paper published | Re-run synthesis |
| **tracks** | Tracked code changes | Analysis script updated | Re-run documentation |

## Recurring Research

For ongoing monitoring, Sarah uses scheduled specs:

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

This spec runs automatically each month when new data arrives.

## Audit Trail

Every drift detection and replay is logged:

```bash
$ chant log 001-ana --drift
```

```
Drift History: 2026-01-16-001-ana
=================================

2026-01-16  Spec created and completed
2026-03-15  DRIFT: +708 observations in origin files
2026-03-15  REPLAY: Analysis updated
2026-03-15  FINDING: Trend strengthened (+0.02°C/decade)

2026-05-01  DRIFT: +420 observations in origin files
2026-05-02  REPLAY: Analysis updated
2026-05-02  FINDING: No significant change

Current status: Up to date (last check: 2026-05-15)
```

## Dissertation Defense

When Sarah defends her dissertation, she can demonstrate:

1. **Reproducibility**: Every analysis has a spec that can be re-run
2. **Provenance**: Every finding traces to specific data files and versions
3. **Currency**: Drift detection ensures findings reflect latest data
4. **Methodology**: Specs document the exact procedures used

Her advisor's reproducibility audit is straightforward:

```bash
# Show all specs with their completion dates and data versions
$ chant list --completed --verbose

# Verify no drift in final results
$ chant drift --all

# Re-run entire pipeline from original data
$ chant replay 001-drv --from-scratch
```

## What's Next

This completes the academic path. See the [artifacts](artifacts/) directory for complete spec examples.

To explore the parallel developer path, start at:

**[Developer Scenario](../developer/01-scenario.md)** — Follow Alex Torres investigating a microservices migration
