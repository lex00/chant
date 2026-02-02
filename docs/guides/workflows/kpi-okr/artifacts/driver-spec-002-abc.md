---
type: driver
status: completed
labels:
  - kpi-churn
  - q1-2026
depends_on:
  - 2026-01-13-001-xyz
members:
  - 2026-01-16-002-abc-1
  - 2026-01-16-002-abc-2
  - 2026-01-16-002-abc-3
completed_at: 2026-01-22T14:12:00Z
model: claude-haiku-4-5-20251001
---

# Reduce Q1 churn: implement approved interventions

Based on research findings in spec 2026-01-13-001-xyz.

## Interventions

1. **Onboarding wizard** — Step-by-step setup flow for new users (P0)
2. **Slack integration GA** — Promote beta to general availability (P1)
3. **Team invite UX** — Surface invite flow in onboarding and sidebar (P1)

## Acceptance Criteria

- [x] All member specs completed
- [x] Combined interventions deployed to staging
- [x] Churn tracking baseline established
