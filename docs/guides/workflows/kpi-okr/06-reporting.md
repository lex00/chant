# Week 4: Reporting

With interventions shipped, the team tracks whether churn actually drops. This phase combines chant's activity tracking with a lightweight cron setup for daily KPI reports.

## Activity Tracking

Chant tracks all spec activity. Sarah uses this to monitor the OKR initiative:

```bash
chant activity --label kpi-churn --since 7d
```

```
2026-01-22 14:10  002-abc-1  COMPLETED  Add onboarding wizard
2026-01-22 14:08  002-abc-2  COMPLETED  Promote Slack integration
2026-01-22 14:06  002-abc-3  COMPLETED  Surface team invite
2026-01-22 14:00  002-abc    WORKED     Reduce Q1 churn (parallel)
2026-01-16 14:15  001-xyz    APPROVED   Analyze Q1 churn drivers
2026-01-15 09:30  001-xyz    REJECTED   Analyze Q1 churn drivers
2026-01-14 16:45  001-xyz    COMPLETED  Analyze Q1 churn drivers
2026-01-13 10:00  001-xyz    CREATED    Analyze Q1 churn drivers
```

This gives a complete audit trail from research through implementation.

## Daily KPI Tracking Spec

Sarah creates a recurring spec to track results:

```bash
chant add "Daily churn KPI report" --type task
```

**File: `.chant/specs/2026-01-22-003-def.md`**

```yaml
---
type: task
status: ready
labels:
  - kpi-churn
  - q1-2026
  - reporting
schedule: daily  # Metadata field - documents intended frequency, not a trigger
context:
  - .chant/context/kpi-churn-q1/research-findings.md
target_files:
  - reports/kpi-churn-daily.md
---

# Daily churn KPI report

Generate daily snapshot of churn KPI progress.

## Acceptance Criteria

- [ ] Current churn rate calculated from billing data
- [ ] Comparison to 8% baseline and 5% target
- [ ] Feature adoption metrics (wizard completion, Slack, team invites)
- [ ] Report written to reports/kpi-churn-daily.md
```

## Scheduling with External Tools

Chant does not have built-in scheduling. Instead, you trigger chant commands from your existing automation infrastructure — cron, CI/CD pipelines, or task schedulers. The `schedule:` field in specs is metadata that documents your intended frequency for human readers; it doesn't trigger execution automatically.

Mike sets up automated daily runs using the team's existing tools:

### Standard Crontab

```bash
# Edit crontab with: crontab -e
# Daily KPI report at 8am UTC
0 8 * * * cd /path/to/repo && chant work --parallel --label reporting
```

### GitHub Actions

```yaml
# .github/workflows/kpi-report.yml
name: Daily KPI Report
on:
  schedule:
    - cron: '0 8 * * *'

jobs:
  report:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run KPI report
        run: chant work --parallel --label reporting
      - name: Post to Slack
        run: |
          chant activity --since 1d --format json | \
            jq -r '.[] | "• \(.title): \(.status)"' | \
            curl -X POST "$SLACK_WEBHOOK" \
              -d "{\"text\": \"Daily KPI Update:\n$(cat -)\"}"
```

## Example Daily Report

After the first daily run, the agent produces:

**File: `reports/kpi-churn-daily.md`**

```markdown
# Churn KPI Report — 2026-01-28

## Summary

| Metric | Baseline | Current | Target | Status |
|--------|----------|---------|--------|--------|
| Monthly churn | 8.0% | 6.2% | 5.0% | On track |
| 30-day activation | 62% | 68% | 75% | Improving |
| Support tickets/wk | 340 | 285 | <250 | Improving |

## Intervention Adoption (First Week)

| Intervention | Metric | Value |
|-------------|--------|-------|
| Onboarding wizard | Completion rate | 71% of new signups |
| Slack integration | GA activations | 312 new connections |
| Team invite | Invites sent via sidebar | 189 teams |

## Trend

Week 1 post-launch shows early positive signals. Churn rate dropped
1.8pp from 8.0% to 6.2%. The onboarding wizard has the highest
engagement (71% completion). Need 2-3 more weeks of data to confirm
the trend is sustained.

## Spec Activity (Last 24h)

- No new specs created
- 003-def (this report) completed
```

## Dashboard View

For a quick summary across all KPI-labeled specs:

```bash
chant list --label kpi-churn --format table
```

```
ID           Type      Status     Title
───────────  ────────  ─────────  ─────────────────────────────────────
001-xyz      research  completed  Analyze Q1 churn drivers
002-abc      driver    completed  Reduce Q1 churn (driver)
002-abc-1    code      completed  Add onboarding wizard
002-abc-2    code      completed  Promote Slack integration
002-abc-3    code      completed  Surface team invite
003-def      task      completed  Daily churn KPI report
```

```bash
chant activity --label kpi-churn --since 30d --format summary
```

```
KPI Churn Q1 2026 — 30 Day Summary

Specs:       6 total, 6 completed
Research:    1 spec (approved after 1 rejection)
Code:        3 specs (all merged)
Reports:     1 recurring (daily)

Timeline:
  Week 1: Data ingestion (human)
  Week 2: Research + approval
  Week 3: Parallel implementation
  Week 4: Tracking and reporting

Labels: kpi-churn, q1-2026
```

## End-to-End Recap

The full workflow from OKR to results:

```
Week 1: Mike creates data digests          → .chant/context/
Week 2: Chant analyzes churn drivers       → research spec (rejected, revised, approved)
Week 3: Chant implements 3 fixes in parallel → 3 code specs merged
Week 4: Daily reports track KPI progress   → 8% → 6.2% (on track for 5%)
```

All artifacts — data digests, research findings, approval discussions, implementation specs, and daily reports — are tracked in git through chant specs. Any team member can reconstruct the full decision chain from OKR to code change.

## See Also

- [Research Workflows](../../research.md) — Research spec patterns
- [Approval Workflow](../../approval-workflow.md) — Approval gates and discussion
- [Enterprise Features](../../../enterprise/enterprise.md) — Derived fields, required fields, audit
