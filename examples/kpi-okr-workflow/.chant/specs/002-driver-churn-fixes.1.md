---
type: implementation
status: completed
labels:
  - kpi-churn
  - q1-2026
  - onboarding
driver: 002-driver-churn-fixes
completed_at: 2026-01-20T11:30:00Z
model: claude-haiku-4-5-20251001
---

# Build onboarding wizard for new users

Part of driver spec 002-driver-churn-fixes.

## Context

Research showed 78% of churned users never used project templates, and 33%
cited "too hard to get started" as their exit reason. New users land on a
blank page with no guidance.

Expected impact: ~3.5pp churn reduction.

## Implementation

Build a 3-step onboarding wizard that activates on first login:

1. **Create from template** - Show 5 popular project templates
2. **Invite team** - Inline team invite form
3. **Connect integration** - Quick setup for Slack, GitHub, or email

## Acceptance Criteria

- [x] Onboarding wizard component created
- [x] Wizard triggers on first login (user.onboarded === false flag)
- [x] Template selection saves user's first project
- [x] Team invite form sends email invitations
- [x] Integration setup connects to at least one service
- [x] Skip button allows bypassing wizard
- [x] Wizard dismissal sets user.onboarded = true
- [x] A/B test tracking integrated (50% rollout)
- [x] Tests passing for onboarding flow
- [x] Deployed to staging

## Implementation Notes

- Wizard uses modal overlay pattern
- Template data pulled from existing templates API
- Team invites reuse existing invite system
- Integration setup uses OAuth flows
- Skip button tracks "wizard_skipped" event for analytics
