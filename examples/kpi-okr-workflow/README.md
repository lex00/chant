# KPI/OKR Workflow Example

This example demonstrates using Chant to tackle a business KPI: reducing customer churn from 8% to 5% in Q1 2026.

## Scenario

**Company:** Acme SaaS Corp
**Problem:** 8% monthly churn, concentrated in first 30 days after signup
**Goal:** Reduce churn to 5% by Q1 end
**Team:** Sarah (Engineering Lead), Mike (Data Analyst), agents (Research & Implementation)

## Workflow Phases

### Phase 1: Data Ingestion (Human-Driven)

Mike gathers data from external systems and creates markdown digests:

```
External Systems ──> Human Digest ──> .chant/context/ ──> Agent Analysis
(Datadog, Zendesk)    (Markdown)       (Git-tracked)      (Research spec)
```

**Files created:**
- `datadog-churn-metrics.md` - Churn rates, timing, feature adoption
- `zendesk-support-patterns.md` - Support ticket analysis
- `user-survey-summary.md` - Exit survey verbatims

These digests are committed to git, making them available for agent analysis.

### Phase 2: Research (Agent-Driven)

Sarah creates a research spec (`001-research-churn-drivers.md`) that:
- References the three context files using `informed_by:`
- Asks specific research questions
- Produces `research-findings.md` with actionable recommendations

The agent identifies three churn drivers:
1. **Failed onboarding** (~3.5pp impact) - No guided setup flow
2. **Missing integrations** (~1.5pp impact) - No Slack, difficult imports
3. **Team discovery friction** (~1.2pp impact) - Invite feature buried in settings

### Phase 3: Implementation (Coordinated)

Sarah creates a driver spec (`002-driver-churn-fixes.md`) that coordinates three parallel implementations:

- **Member 1** (`002-driver-churn-fixes.1.md`) - Build onboarding wizard
- **Member 2** (`002-driver-churn-fixes.2.md`) - Promote Slack integration to GA
- **Member 3** (`002-driver-churn-fixes.3.md`) - Improve team invite UX

Each member spec has detailed acceptance criteria and can be worked independently.

## Key Patterns Demonstrated

### Context References
```yaml
informed_by:
  - .chant/context/kpi-churn-q1/datadog-churn-metrics.md
  - .chant/context/kpi-churn-q1/zendesk-support-patterns.md
  - .chant/context/kpi-churn-q1/user-survey-summary.md
```

Agents read these files to ground their analysis in real data.

### Research Output
```yaml
target_files:
  - .chant/context/kpi-churn-q1/research-findings.md
```

The research spec produces a findings document that becomes the basis for implementation decisions.

### Driver Coordination
```yaml
type: driver
members:
  - 2026-01-16-002-abc-1
  - 2026-01-16-002-abc-2
  - 2026-01-16-002-abc-3
```

The driver spec tracks completion of all member specs, enabling parallel work.

## What This Example Shows

1. **Human-agent collaboration** - Humans curate data, agents analyze it
2. **Context ingestion** - Using `.chant/context/` for external data
3. **Research specs** - Producing actionable insights from data
4. **Driver specs** - Coordinating multi-part implementations
5. **Parallel execution** - Independent member specs worked concurrently

## Expected Outcome

Combined interventions targeting 6.2pp churn reduction:
- 8% → ~3.5% (exceeds 5% target)
- Addresses 63% of early-stage churn
- Validated by cross-referencing three data sources

## Running This Example

This is a demonstration example showing completed specs. To replicate the workflow:

1. **Review the context files** in `.chant/context/kpi-churn-q1/`
2. **Examine the research spec** (`001-research-churn-drivers.md`)
3. **See the research output** (`research-findings.md`)
4. **Study the driver spec** (`002-driver-churn-fixes.md`)
5. **Explore member specs** showing parallel implementation pattern

## Files in This Example

```
examples/kpi-okr-workflow/
├── README.md                              # This file
├── .chant/
│   ├── config.md                          # Project configuration
│   ├── context/
│   │   └── kpi-churn-q1/
│   │       ├── datadog-churn-metrics.md   # Mock Datadog data
│   │       ├── zendesk-support-patterns.md # Mock Zendesk data
│   │       ├── user-survey-summary.md     # Mock survey data
│   │       └── research-findings.md       # Research output
│   ├── specs/
│   │   ├── 001-research-churn-drivers.md  # Research spec (completed)
│   │   ├── 002-driver-churn-fixes.md      # Driver spec
│   │   ├── 002-driver-churn-fixes.1.md    # Member: onboarding wizard
│   │   ├── 002-driver-churn-fixes.2.md    # Member: Slack integration
│   │   └── 002-driver-churn-fixes.3.md    # Member: team invite UX
│   └── prompts/
│       └── standard.md                    # Standard prompt template
```
