# Week 2: Research Phase

With context digests committed, Sarah creates a research spec for chant to analyze the churn data and identify actionable drivers.

## Creating the Research Spec

```bash
chant add "Analyze Q1 churn drivers from support, metrics, and survey data"
```

This creates spec `2026-01-13-001-xyz`. Sarah edits it to reference the context digests and define clear research questions.

## Complete Research Spec

**File: `.chant/specs/2026-01-13-001-xyz.md`**

```yaml
---
type: research
status: ready
labels:
  - kpi-churn
  - q1-2026
needs_approval: true
approval:
  required: true
  status: pending
informed_by:
  - .chant/context/kpi-churn-q1/datadog-churn-metrics-2026-01.md
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

- [ ] What are the top 3 churn drivers by impact?
- [ ] Which drivers are addressable through product changes?
- [ ] What is the expected churn reduction for each intervention?
- [ ] What is the recommended implementation priority?

## Methodology

1. Cross-reference churn timing data with support ticket patterns
2. Map feature adoption gaps to exit survey reasons
3. Quantify impact of each driver using available metrics
4. Rank interventions by expected churn reduction and implementation effort

## Acceptance Criteria

- [ ] Top 3 churn drivers identified with supporting data
- [ ] Each driver linked to specific metrics from context files
- [ ] Recommended interventions with expected impact estimates
- [ ] Priority ranking based on effort vs. impact
- [ ] Findings written to research-findings.md
```

## Executing the Research

```bash
chant work 2026-01-13-001-xyz
```

The agent reads the three context digests, cross-references the data, and produces findings.

## Research Output

After execution, the agent creates the findings document:

**File: `.chant/context/kpi-churn-q1/research-findings.md`**

```markdown
# Q1 Churn Analysis — Research Findings

Spec: 2026-01-13-001-xyz
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
```

## Spec Status After Research

The agent marks all research questions and acceptance criteria as checked:

```yaml
---
type: research
status: completed
labels:
  - kpi-churn
  - q1-2026
needs_approval: true
approval:
  required: true
  status: pending
completed_at: 2026-01-14T16:45:00Z
model: claude-haiku-4-5-20251001
target_files:
  - .chant/context/kpi-churn-q1/research-findings.md
---
```

The research is complete, but approval is still pending. The agent has done its analysis — now the humans decide whether to act on it.

## What's Next

Sarah and the team review the findings and decide whether to approve, reject, or refine:

**[Approval Gate](04-approval.md)** — Human review of research findings before implementation begins.
