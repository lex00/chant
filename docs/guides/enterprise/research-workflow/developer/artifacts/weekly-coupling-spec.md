---
type: research
status: pending
labels:
  - migration
  - monitoring
  - weekly
schedule: weekly
informed_by:
  - src/**/*.py
origin:
  - .chant/context/migration-research/production-metrics.md
target_files:
  - reports/weekly/coupling-status.md
---

# Weekly coupling status

## Purpose

Track coupling changes week-over-week as the codebase evolves.
This scheduled spec runs every Monday to detect:
- New dependencies introduced
- Circular dependency changes
- Progress on extraction candidates

## Schedule

Runs automatically every Monday at 09:00.
Results posted to #engineering-alerts Slack channel.

## Methodology

1. Recalculate import dependency matrix
2. Compare to previous week's matrix
3. Flag any new circular dependencies
4. Recalculate extraction candidate scores
5. Generate trend report

## Output Format

```markdown
# Coupling Status - Week of [DATE]

## Changes Since Last Week

| Metric | Last Week | This Week | Change |
|--------|-----------|-----------|--------|
| Total dependencies | N | M | ±X |
| Circular chains | X | Y | ±Z |
| Avg fan-out | A | B | ±C |

## New Dependencies

| From | To | Added By | File |
|------|------|----------|------|
| module_a | module_b | @dev | path/to/file.py |

## Extraction Progress

| Candidate | Last Week | This Week | Trend |
|-----------|-----------|-----------|-------|
| reporting | 8.5 | 8.7 | ↑ |
| notifications | 7.2 | 7.2 | → |
| billing | 5.8 | 5.6 | ↓ |

## Alerts

- [Warning/Critical/None]

## Recommendations

- [Action items if any]
```

## Acceptance Criteria

- [ ] Dependency matrix recalculated
- [ ] Comparison to previous week documented
- [ ] New dependencies identified with source
- [ ] Extraction scores updated
- [ ] Alerts for concerning patterns
- [ ] Report generated to target file

## Drift Integration

This scheduled spec complements manual drift detection:
- `chant drift` — On-demand staleness check
- This spec — Automated weekly trend analysis

Both track the same codebase, providing complementary views.
