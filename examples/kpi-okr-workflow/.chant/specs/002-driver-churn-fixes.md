---
type: driver
status: completed
labels:
  - kpi-churn
  - q1-2026
depends_on:
  - 001-research-churn-drivers
members:
  - 002-driver-churn-fixes.1
  - 002-driver-churn-fixes.2
  - 002-driver-churn-fixes.3
completed_at: 2026-01-22T14:12:00Z
model: claude-haiku-4-5-20251001
---

# Reduce Q1 churn: implement approved interventions

Based on research findings in spec 001-research-churn-drivers.

## Interventions

1. **Onboarding wizard** — Step-by-step setup flow for new users (P0)
2. **Slack integration GA** — Promote beta to general availability (P1)
3. **Team invite UX** — Surface invite flow in onboarding and sidebar (P1)

## Acceptance Criteria

- [x] All member specs completed
- [x] Combined interventions deployed to staging
- [x] Churn tracking baseline established
