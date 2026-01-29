---
type: research
status: pending
labels:
  - dissertation
  - arctic-warming
  - monitoring
schedule: monthly
origin:
  - data/temperature/*.csv
informed_by:
  - analysis/temperature-findings.md
target_files:
  - reports/monthly/temperature-update.md
---

# Monthly temperature update

## Problem

Arctic temperature monitoring requires ongoing analysis as new data arrives.
Each month, monitoring stations add new observations that may affect findings.

## Schedule

This spec runs automatically on the first of each month when new data is
detected in the origin files.

## Methodology

1. Load current data files
2. Identify new observations since last run
3. Re-calculate key statistics
4. Compare to previous month's results
5. Flag significant changes

## Acceptance Criteria

- [ ] New data incorporated into analysis
- [ ] Trend recalculated with updated dataset
- [ ] Comparison to previous month documented
- [ ] Alert generated if trend changes >0.05°C/decade
- [ ] Monthly report generated

## Output Format

```markdown
# Monthly Temperature Update - [MONTH YEAR]

## New Data
- Observations added: N
- Stations with updates: X/12

## Updated Statistics
| Metric | Previous | Current | Change |
|--------|----------|---------|--------|
| Trend | X°C/decade | Y°C/decade | ±Z |
| Acceleration p | X | Y | ±Z |

## Alerts
- [None / List any significant changes]

## Conclusion
[Brief summary of whether findings remain stable]
```

## Drift Integration

This scheduled spec complements manual drift detection:
- `chant drift` — Manual check for staleness
- This spec — Automated monthly re-analysis

Both use the same origin files, ensuring consistency.
