# The Engineering Context

## Alex Torres

Alex Torres is a Staff Engineer at TechCorp, a mid-sized SaaS company. They've been asked to evaluate migrating the company's monolithic backend to microservices.

| Attribute | Value |
|-----------|-------|
| Title | Staff Engineer |
| Company | TechCorp |
| Team Size | 12 engineers across 3 teams |
| Tenure | 4 years |
| Timeline | 6-week evaluation |

## The Codebase

TechCorp's backend is a Python monolith that has grown over 7 years:

| Metric | Value |
|--------|-------|
| Lines of code | 80,000 |
| Python packages | 42 |
| Database tables | 156 |
| API endpoints | 234 |
| Test coverage | 68% |
| Average deployment | Weekly |

## Migration Question

Leadership wants to know: **Should we migrate to microservices, and if so, how?**

The question has layers:
- Which components should be extracted first?
- What are the coupling hotspots?
- How will this affect the 3 existing teams?
- What's the risk profile?

Alex needs to research the codebase systematically, not just skim and guess.

## Current Pain Points

The monolith has symptoms that suggest extraction might help:

| Symptom | Impact |
|---------|--------|
| Slow CI/CD | Full test suite takes 45 minutes |
| Deployment coupling | Team A's changes require Team B's review |
| Scaling bottlenecks | Auth service needs 10x capacity of reporting |
| Onboarding time | New engineers take 3 months to be productive |
| Bug blast radius | One module failure can crash everything |

## Why Research First?

Alex has seen migrations fail when teams skip investigation:

> "We extracted the auth service first because it felt obvious. Turns out it had deep coupling to user preferences, billing, and audit logging. Should have mapped the dependencies first."

Chant provides structured investigation before implementation.

## Project Setup

Alex initializes chant in the monolith repository:

```bash
# Initialize chant
chant init --agent claude

# Create directories for investigation outputs
mkdir -p docs/architecture
mkdir -p analysis/coupling
mkdir -p analysis/metrics

# Create context directory for external inputs
mkdir -p .chant/context/migration-research
```

Directory structure:

```
techcorp-backend/
├── .chant/
│   ├── specs/              # Research and implementation specs
│   ├── context/            # External data (metrics, diagrams)
│   │   └── migration-research/
│   └── config.md
├── src/                    # 80K LOC Python monolith
│   ├── auth/
│   ├── billing/
│   ├── reporting/
│   ├── users/
│   └── ...
├── tests/                  # Test suite
├── docs/                   # Documentation outputs
│   └── architecture/
└── analysis/              # Analysis outputs
```

## Investigation Timeline

Alex plans a four-week research phase:

```
Week 1          Week 2              Week 3            Week 4
┌──────────┐   ┌───────────────┐   ┌──────────────┐   ┌──────────────┐
│ Codebase │   │ Architecture  │   │    POC       │   │   Ongoing    │
│ Analysis │──>│ Documentation │──>│ Extraction   │──>│ Maintenance  │
│(Coupling)│   │  (Decisions)  │   │  (Driver)    │   │   (Drift)    │
└──────────┘   └───────────────┘   └──────────────┘   └──────────────┘
```

## Spec Workflow Preview

Alex's investigation will use these spec types:

| Week | Spec Type | Purpose |
|------|-----------|---------|
| 1 | `research` with `informed_by:` | Analyze codebase coupling |
| 2 | `documentation` with `tracks:` | Document architecture decisions |
| 3 | `driver` with members | Coordinate POC extraction |
| 4+ | Drift detection | Keep docs current as code evolves |

## Team Structure

Unlike solo academic work, Alex coordinates with teams:

- **Alex** creates specs, reviews findings, presents to leadership
- **Chant agents** analyze coupling, generate documentation
- **Team leads** review architecture decisions
- **CI/CD** runs weekly coupling reports

## Metrics Context

Alex gathers baseline metrics from production:

**File: `.chant/context/migration-research/production-metrics.md`**

```markdown
# Production Metrics Baseline

Source: Datadog APM, January 2026
Period: Last 30 days

## Request Volume by Module

| Module | Requests/day | P99 Latency | Error Rate |
|--------|-------------|-------------|------------|
| auth | 2,400,000 | 45ms | 0.01% |
| users | 890,000 | 120ms | 0.05% |
| billing | 340,000 | 380ms | 0.12% |
| reporting | 45,000 | 2,400ms | 0.08% |
| notifications | 180,000 | 89ms | 0.02% |

## Resource Utilization

| Module | CPU % | Memory % | DB Queries/req |
|--------|-------|----------|----------------|
| auth | 15% | 8% | 2.1 |
| users | 22% | 18% | 4.7 |
| billing | 45% | 35% | 12.3 |
| reporting | 78% | 52% | 28.6 |
| notifications | 8% | 5% | 1.4 |

## Observations

- Reporting consumes disproportionate resources
- Auth has highest volume but lowest resource use
- Billing has highest error rate and query count
```

## What's Next

With the project initialized and metrics gathered, Alex begins codebase investigation:

**[Investigation](02-investigation.md)** — Analyzing coupling and dependencies using research specs with `informed_by:`
