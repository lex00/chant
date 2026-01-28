# Enterprise KPI/OKR Workflow

A complete walkthrough showing how chant drives a real business KPI from data analysis through implementation.

## The Scenario

**Acme SaaS Corp** is a B2B platform with 5,000 customers. Their Q1 OKR: reduce customer churn from 8% to 5%. This guide follows their team through the full workflow — from gathering data to shipping fixes to tracking results.

### Team

| Role | Person | Responsibility |
|------|--------|---------------|
| VP Product | Sarah | Defines KPIs, approves specs |
| Data Analyst | Mike | Gathers data, creates digests |
| Engineers | (managed by chant) | Implement approved changes |

### KPI Definition

```
Metric:    Monthly customer churn rate
Baseline:  8% (December 2025)
Target:    5% by end of Q1 2026
Timeline:  4 weeks
```

## Workflow Phases

```
Week 1          Week 2              Week 2            Week 3         Week 4
┌──────────┐   ┌───────────────┐   ┌──────────┐   ┌───────────┐   ┌──────────┐
│  Human    │   │    Chant      │   │  Human   │   │   Chant   │   │  Track   │
│  Data     │──>│   Research    │──>│ Approval │──>│  Execute  │──>│ Results  │
│ Ingestion │   │   Phase       │   │  Gate    │   │  Parallel │   │  Daily   │
└──────────┘   └───────────────┘   └──────────┘   └───────────┘   └──────────┘
```

## Guide Pages

1. **[The Business Context](01-scenario.md)** — Acme's product, churn problem, and Q1 OKR
2. **[Data Ingestion](02-data-ingestion.md)** — Week 1: Human investigation and data gathering
3. **[Research Phase](03-research.md)** — Week 2: Chant agent analyzes churn drivers
4. **[Approval Gate](04-approval.md)** — Week 2: Team reviews and approves findings
5. **[Implementation](05-implementation.md)** — Week 3: Parallel execution of fixes
6. **[Reporting](06-reporting.md)** — Week 4: Daily tracking and dashboards

## Key Concepts Demonstrated

- **Context directories** for ingesting external data
- **Research specs** for AI-driven analysis
- **Approval workflow** with reject/approve cycle
- **Driver specs** that decompose into parallel member specs
- **Activity tracking** and reporting for stakeholder visibility

## Prerequisites

Familiarity with [core concepts](../../../concepts/specs.md), [research workflows](../../research.md), and [approval workflows](../../approval-workflow.md).
