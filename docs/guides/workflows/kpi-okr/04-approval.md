# Week 2: Approval Gate

The research spec is complete. Before any code is written, Sarah and the team review the findings.

## Review the Research

```bash
chant show 2026-01-13-001-xyz
```

```
ID:      2026-01-13-001-xyz
Title:   Analyze Q1 churn drivers
Type:    research
Status:  completed

Approval: PENDING
Labels:   kpi-churn, q1-2026

Acceptance Criteria:
  [x] Top 3 churn drivers identified with supporting data
  [x] Each driver linked to specific metrics from context files
  [x] Recommended interventions with expected impact estimates
  [x] Priority ranking based on effort vs. impact
  [x] Findings written to research-findings.md
```

Sarah reads the full findings in `.chant/context/kpi-churn-q1/research-findings.md` and shares with the team.

## First Review: Rejection

After discussion, Sarah rejects the spec. The integration estimates seem optimistic without more data:

```bash
chant reject 2026-01-13-001-xyz --by sarah \
  --reason "Integration impact estimate (1.5pp) needs validation. \
We have integration usage data from the beta program — agent should \
factor that in. Also need cost estimate for import work."
```

```
Spec 2026-01-13-001-xyz REJECTED by sarah

Reason: Integration impact estimate (1.5pp) needs validation.
We have integration usage data from the beta program — agent should
factor that in. Also need cost estimate for import work.
```

The spec file now shows the rejection in the discussion section:

```markdown
## Approval Discussion

**sarah** - 2026-01-15 09:30 - REJECTED
Integration impact estimate (1.5pp) needs validation. We have integration
usage data from the beta program — agent should factor that in. Also
need cost estimate for import work.
```

## Adding More Context

Mike adds the integration beta data to the context directory:

```bash
vim .chant/context/kpi-churn-q1/integration-beta-usage.md
git add .chant/context/kpi-churn-q1/integration-beta-usage.md
git commit -m "Add integration beta usage data for churn research"
```

**File: `.chant/context/kpi-churn-q1/integration-beta-usage.md`**

```markdown
# Integration Beta - Usage Data

Source: Internal beta program (n=200 customers, Oct-Dec 2025)

## Adoption

- 200 customers invited to beta
- 134 enabled integration (67% adoption)
- 98 still active after 30 days (73% retention of adopters)

## Churn Comparison (Beta Period)

| Group | Churn Rate | n |
|-------|-----------|---|
| Integration beta (enabled) | 4.5% | 134 |
| Integration beta (not enabled) | 9.2% | 66 |
| Non-beta (control) | 10.8% | 3,000 |

## Key Insight

Integration correlates with 6.3pp lower churn, but self-selection
bias is likely. Conservative estimate: 2-3pp attributable to integration.
```

## Re-executing Research

Sarah resumes the spec with the new context:

```bash
chant reset 2026-01-13-001-xyz --work
```

The agent re-reads all context files (including the new integration beta data) and updates the findings. The integration section now reflects validated data:

```markdown
### 2. Missing Integrations (Impact: ~2.0pp, revised)

**Updated with integration beta data:**
- Beta users with integration enabled: 4.5% churn vs 10.8% control
- Conservative attributable impact: 2-3pp (accounting for self-selection)
- Revised estimate: ~2.0pp (midpoint of conservative range)

**Cost note:** Additional integrations require API integration work.
Existing integration already exists in beta — promotion to GA is low effort.
```

## Second Review: Approval

Sarah reviews the updated findings. The integration data validates the thesis, and the revised estimates are more credible:

```bash
chant approve 2026-01-13-001-xyz --by sarah
```

```
Spec 2026-01-13-001-xyz APPROVED by sarah
```

The discussion section now shows the full history:

```markdown
## Approval Discussion

**sarah** - 2026-01-15 09:30 - REJECTED
Integration impact estimate (1.5pp) needs validation. We have Slack
usage data from the beta program — agent should factor that in. Also
need cost estimate for Trello/Asana import.

**sarah** - 2026-01-16 14:15 - APPROVED
```

## Why This Matters

The reject-revise-approve cycle is the key value of approval gates:

- **No wasted implementation work** — Code isn't written until the analysis is validated
- **Human judgment preserved** — Sarah caught an optimistic estimate before it drove engineering decisions
- **Audit trail** — The spec file records who rejected, why, and when it was eventually approved
- **Data-driven iteration** — The rejection surfaced the Slack beta data, making the final analysis stronger

## What's Next

With approved findings, Sarah creates implementation specs:

**[Implementation](05-implementation.md)** — Converting research into a driver spec with parallel member specs.
