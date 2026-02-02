# KPI/OKR Workflow Example

## Overview

This example demonstrates using Chant to tackle a business KPI: reducing customer churn from 8% to 5% in Q1 2026. It shows human-agent collaboration where humans curate data and agents analyze it, using context files, research specs, and coordinated parallel implementation through a driver spec pattern.

## Structure

The workflow consists of two main specs:

1. **001-research-churn-drivers.md** - Research spec that analyzes context files to identify churn drivers and produces research findings
2. **002-driver-churn-fixes.md** - Driver spec coordinating three parallel implementations:
   - **002-driver-churn-fixes.1.md** - Build onboarding wizard (~3.5pp churn impact)
   - **002-driver-churn-fixes.2.md** - Promote Slack integration to GA (~1.5pp impact)
   - **002-driver-churn-fixes.3.md** - Improve team invite UX (~1.2pp impact)

Context files in `.chant/context/kpi-churn-q1/`:
- `datadog-churn-metrics.md` - Churn rates, timing, feature adoption
- `zendesk-support-patterns.md` - Support ticket analysis
- `user-survey-summary.md` - Exit survey verbatims
- `research-findings.md` - Agent-produced analysis (output from spec 001)

## Usage

Execute the research spec with the driver pattern:
```bash
cd examples/kpi-okr-workflow
chant work 001  # Run research spec first
chant work 002  # Run driver spec (coordinates members 1-3)
```

Or work member specs independently in parallel:
```bash
chant work 002.1  # Onboarding wizard
chant work 002.2  # Slack integration
chant work 002.3  # Team invite UX
```

View spec status:
```bash
chant list
chant show 002  # View driver spec and member status
```

## Testing

Review the workflow by examining:
1. Context files in `.chant/context/kpi-churn-q1/` - See how external data is structured
2. Research spec `001-research-churn-drivers.md` - Uses `informed_by:` to reference context
3. Research output `research-findings.md` - Agent analysis of the data
4. Driver spec `002-driver-churn-fixes.md` - Coordinates parallel work with `members:` field
5. Member specs - Each has detailed acceptance criteria for independent execution

Expected outcome: Combined interventions targeting 6.2pp churn reduction (8% → ~3.5%, exceeding 5% target).

## See Also

- [Enterprise KPI/OKR Workflow Guide](../../docs/guides/enterprise/kpi-okr/README.md) — Complete walkthrough with business context
