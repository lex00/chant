# Week 1: Data Ingestion

The first phase is entirely human-driven. Mike (Data Analyst) gathers data from external systems and creates markdown digests that chant agents can read.

## Why Human-First?

Chant agents don't have access to Datadog, Zendesk, or survey tools. The human-in-the-loop pattern is:

```
External Systems ──> Human Digest ──> .chant/context/ ──> Agent Analysis
(Datadog, Zendesk)    (Markdown)       (Git-tracked)      (Research spec)
```

Mike translates raw data into structured markdown. This is intentional — it forces a human to curate what matters before handing off to an agent.

## Data Sources

Mike pulls from three systems:

| Source | What | Format |
|--------|------|--------|
| Datadog | Churn metrics, cohort analysis | Dashboard exports |
| Zendesk | Support ticket patterns | Ticket exports, tags |
| Typeform | User exit survey responses | Survey results |

## Digest 1: Churn Metrics

Mike exports Datadog dashboards and distills them into a markdown digest:

```bash
# Create the digest file
vim .chant/context/kpi-churn-q1/datadog-churn-metrics-2026-01.md
```

**File: `.chant/context/kpi-churn-q1/datadog-churn-metrics-2026-01.md`**

```markdown
# Churn Metrics - January 2026

Source: Datadog dashboard "Customer Health"
Exported: 2026-01-06

## Monthly Churn by Segment

| Segment | Oct | Nov | Dec | Trend |
|---------|-----|-----|-----|-------|
| Startup | 9% | 10% | 11% | Rising |
| Mid-Market | 5% | 4% | 4% | Stable |
| Enterprise | 1% | 1% | 1% | Stable |
| Overall | 7% | 7.5% | 8% | Rising |

## Churn Timing (Days After Signup)

- 0-7 days: 35% of all churns
- 8-30 days: 28% of all churns
- 31-90 days: 22% of all churns
- 90+ days: 15% of all churns

Key insight: 63% of churn happens in the first 30 days.

## Feature Adoption at Churn

Customers who churned in Dec had NOT used:
- Project templates: 78% never used
- Team invites: 65% never invited a teammate
- Integrations: 82% had zero integrations

Customers retained 90+ days had used:
- Project templates: 91% used in first week
- Team invites: 88% invited 2+ teammates
- Integrations: 74% connected at least one
```

## Digest 2: Support Patterns

Mike reviews Zendesk ticket trends for churned customers:

**File: `.chant/context/kpi-churn-q1/zendesk-support-patterns.md`**

```markdown
# Support Ticket Patterns - Churned Customers

Source: Zendesk export, Dec 2025 churns (n=400)
Exported: 2026-01-07

## Top Ticket Categories (Churned Customers)

| Category | Count | % of Churns | Avg Response Time |
|----------|-------|------------|-------------------|
| Onboarding confusion | 142 | 36% | 4.2 hours |
| Missing features | 89 | 22% | 6.1 hours |
| Billing questions | 67 | 17% | 2.8 hours |
| Performance issues | 54 | 14% | 3.5 hours |
| Other | 48 | 12% | 5.0 hours |

## Common Phrases in Tickets (Pre-Churn)

- "How do I set up my first project?" (87 tickets)
- "Can I import from Trello/Asana?" (45 tickets)
- "Where is the dashboard?" (38 tickets)
- "My team can't see my projects" (29 tickets)

## Ticket-to-Churn Correlation

- Customers with 3+ tickets in first week: 42% churn rate
- Customers with 0 tickets in first week: 6% churn rate
- Unresolved tickets at churn: 68% had at least one open ticket
```

## Digest 3: Exit Survey

Mike summarizes exit survey responses:

**File: `.chant/context/kpi-churn-q1/user-survey-summary.md`**

```markdown
# Exit Survey Summary - Q4 2025

Source: Typeform exit survey (n=156 responses, 39% response rate)
Exported: 2026-01-08

## Primary Reason for Leaving

| Reason | Count | % |
|--------|-------|---|
| Too hard to get started | 52 | 33% |
| Missing key feature | 34 | 22% |
| Switched to competitor | 28 | 18% |
| Price too high | 22 | 14% |
| Other | 20 | 13% |

## Verbatim Highlights

> "I spent 2 hours trying to set up my first project and gave up."

> "Couldn't figure out how to invite my team. The settings menu is buried."

> "No Slack integration was a dealbreaker for us."

> "Love the concept but the onboarding wizard just drops you on a blank page."
```

## Commit the Context

Mike commits all digests to the repo:

```bash
git add .chant/context/kpi-churn-q1/
git commit -m "Add Q1 churn KPI context digests"
```

The data is now git-tracked and available for chant agents to reference.

## What's Next

With context digested, Sarah creates a research spec for chant to analyze the data:

**[Research Phase](03-research.md)** — Chant agent reads these digests and identifies actionable churn drivers.
