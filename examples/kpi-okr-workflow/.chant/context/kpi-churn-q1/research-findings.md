# Q1 Churn Analysis — Research Findings

Spec: 001-research-churn-drivers
Completed: 2026-01-14

## Executive Summary

Three drivers account for an estimated 6.2 percentage points of the 8%
monthly churn. Addressing all three could reduce churn to approximately
3.5%, exceeding the 5% target.

## Top 3 Churn Drivers

### 1. Failed Onboarding (Impact: ~3.5pp)

**Evidence:**
- 63% of churn occurs in the first 30 days (Datadog)
- 36% of churned customers filed onboarding tickets (Zendesk)
- 33% of exit survey cited "too hard to get started" (Survey)
- 78% of churned users never used project templates (Datadog)

**Root cause:** New users land on a blank page after signup. No guided
setup, no templates suggested, no team invite prompt.

**Recommended intervention:** Onboarding wizard with step-by-step setup
(create project from template, invite team, connect integration).

**Expected impact:** Reducing first-30-day churn by 50% = ~3.5pp reduction.

### 2. Missing Integrations (Impact: ~1.5pp)

**Evidence:**
- 82% of churned users had zero integrations (Datadog)
- "No Slack integration" cited as dealbreaker (Survey)
- 45 tickets asking about Trello/Asana import (Zendesk)

**Root cause:** No Slack integration exists. Import from competitors
requires manual CSV export.

**Recommended intervention:** Slack notification integration and one-click
import from Trello/Asana.

**Expected impact:** ~1.5pp churn reduction based on integration adoption
correlation.

### 3. Team Discovery Friction (Impact: ~1.2pp)

**Evidence:**
- 65% of churned users never invited a teammate (Datadog)
- "Can't figure out how to invite my team" — 29 tickets (Zendesk)
- Team invite setting "buried" in settings (Survey)

**Root cause:** Team invite is nested under Settings > Organization >
Members. Not surfaced during onboarding or in main navigation.

**Recommended intervention:** Surface team invite in onboarding wizard
and add persistent "Invite Team" button to sidebar.

**Expected impact:** ~1.2pp churn reduction based on team-size retention
correlation.

## Priority Matrix

| # | Intervention | Impact | Effort | Priority |
|---|-------------|--------|--------|----------|
| 1 | Onboarding wizard | ~3.5pp | Medium | P0 |
| 2 | Slack + import integrations | ~1.5pp | Medium | P1 |
| 3 | Team invite UX | ~1.2pp | Low | P1 |

## Recommendation

Implement all three. Start with the onboarding wizard (highest impact)
and team invite UX (lowest effort) in parallel. Follow with integrations.

Combined expected impact: 8% → ~3.5% (exceeds 5% target).
