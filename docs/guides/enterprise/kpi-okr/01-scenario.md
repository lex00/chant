# The Business Context

## Acme SaaS Corp

Acme SaaS Corp runs a B2B project management platform. Key numbers:

| Metric | Value |
|--------|-------|
| Customers | 5,000 |
| MRR | $2.5M |
| Monthly churn | 8% (400 customers/month) |
| Revenue at risk | $200K/month |
| Customer segments | Startup, Mid-Market, Enterprise |

The 8% churn rate is unsustainable. Each lost customer costs roughly $500/month in MRR, and acquiring a replacement costs 5x more than retention.

## Q1 OKR

**Objective:** Improve customer retention

**Key Result:** Reduce monthly churn from 8% to 5%

```
Target delta:  -3 percentage points
Revenue saved: ~$75K/month at target
Timeline:      End of Q1 2026 (4 weeks)
```

## KPIs Being Tracked

Sarah (VP Product) defines the key performance indicators that the Q1 OKR targets:

| Metric | Baseline | Target | How Measured |
|--------|----------|--------|-------------|
| Monthly churn rate | 8% | 5% | Billing system exports |
| 30-day activation rate | 62% | 75% | Product analytics |
| Support ticket volume | 340/week | <250/week | Zendesk |
| NPS score | 32 | 40+ | Quarterly survey |

## Churn Breakdown (Current State)

Mike (Data Analyst) pulls initial numbers by segment:

| Segment | Customers | Churn Rate | Lost/Month |
|---------|-----------|-----------|------------|
| Startup (<50 seats) | 3,200 | 11% | 352 |
| Mid-Market (50-500) | 1,400 | 4% | 56 |
| Enterprise (500+) | 400 | 1% | 4 |

The problem is concentrated in the Startup segment. Mid-Market and Enterprise are healthy.

## Team Structure

The project follows chant's orchestrator pattern:

- **Sarah** creates specs, reviews findings, approves work
- **Mike** gathers external data, creates context digests for chant
- **Chant agents** analyze data (research specs) and implement fixes (code specs)
- **CI/CD** runs daily activity reports and KPI tracking

## Project Setup

```bash
# Initialize chant with enterprise features
chant init --agent claude

# Create context directory for KPI data
mkdir -p .chant/context/kpi-churn-q1
```

The `.chant/context/` directory holds human-curated data that agents can reference during research and implementation.

## What's Next

With the OKR defined and project initialized, the team begins the four-phase workflow:

1. **[Data Ingestion](02-data-ingestion.md)** — Mike gathers metrics, support data, and survey results
2. **[Research](03-research.md)** — Chant agent analyzes the data for churn drivers
3. **[Approval](04-approval.md)** — Sarah reviews and approves the analysis
4. **[Implementation](05-implementation.md)** — Chant executes approved fixes in parallel
