---
type: research
status: completed
labels:
  - kpi-churn
  - q1-2026
needs_approval: true
approval:
  required: true
  status: approved
completed_at: 2026-01-14T16:45:00Z
model: claude-haiku-4-5-20251001
informed_by:
  - .chant/context/kpi-churn-q1/datadog-churn-metrics.md
  - .chant/context/kpi-churn-q1/zendesk-support-patterns.md
  - .chant/context/kpi-churn-q1/user-survey-summary.md
target_files:
  - .chant/context/kpi-churn-q1/research-findings.md
---

# Analyze Q1 churn drivers

## Problem

Acme SaaS Corp has 8% monthly churn, concentrated in the Startup segment
(11%). 63% of churn happens in the first 30 days. We need to identify the
top actionable drivers and recommend specific interventions.

## Research Questions

- [x] What are the top 3 churn drivers by impact?
- [x] Which drivers are addressable through product changes?
- [x] What is the expected churn reduction for each intervention?
- [x] What is the recommended implementation priority?

## Methodology

1. Cross-reference churn timing data with support ticket patterns
2. Map feature adoption gaps to exit survey reasons
3. Quantify impact of each driver using available metrics
4. Rank interventions by expected churn reduction and implementation effort

## Acceptance Criteria

- [x] Top 3 churn drivers identified with supporting data
- [x] Each driver linked to specific metrics from context files
- [x] Recommended interventions with expected impact estimates
- [x] Priority ranking based on effort vs. impact
- [x] Findings written to research-findings.md

## Approval Discussion

**sarah** - 2026-01-15 09:30 - REJECTED
Integration impact estimate (1.5pp) needs validation. We have Slack
usage data from the beta program — agent should factor that in. Also
need cost estimate for Trello/Asana import.

**sarah** - 2026-01-16 14:15 - APPROVED
Revised analysis incorporates Slack beta data. Impact estimate now
grounded in actual usage patterns. Proceeding with implementation.
