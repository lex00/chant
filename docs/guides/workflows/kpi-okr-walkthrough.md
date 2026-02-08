# KPI/OKR Walkthrough: Reducing Churn at Acme SaaS Corp

Command examples and output are illustrative — your exact output will differ.

## The Business Problem

Acme SaaS Corp runs a B2B project management platform with 5,000 customers and $2.5M in monthly recurring revenue. Their December churn rate hit 8% — roughly 400 customers lost per month, with $200K in MRR walking out the door. Acquiring a replacement customer costs five times more than retaining one, so the leadership team sets a Q1 OKR: **reduce monthly churn from 8% to 5%.**

A quick segment breakdown reveals where the pain is:

| Segment | Customers | Churn Rate |
|---------|-----------|-----------|
| Startup (< 50 seats) | 3,200 | 11% |
| Mid-Market (50-500) | 1,400 | 4% |
| Enterprise (500+) | 400 | 1% |

Mid-Market and Enterprise are healthy. The Startup segment is bleeding.

Three people are involved. Sarah (VP Product) owns the OKR and approves work. Mike (Data Analyst) gathers metrics from external systems and translates them into markdown digests. Chant agents handle the analysis and implementation.

Sarah initializes the project:

```bash
$ chant init --agent claude
```

She creates a context directory for the KPI data that Mike will populate:

```bash
$ mkdir -p .chant/context/kpi-churn-q1
```

The `.chant/context/` directory holds human-curated data that agents can reference during research and implementation. It is git-tracked, so every team member — and every agent — sees the same source of truth.

## Week 1: Data Ingestion

The first phase is entirely human-driven. Chant agents do not have access to Datadog, Zendesk, or Typeform. Instead, Mike exports dashboards and ticket data, distills them into structured markdown, and commits the digests to the context directory. This is the human-in-the-loop pattern:

```
External Systems ──> Human Digest ──> .chant/context/ ──> Agent Analysis
(Datadog, Zendesk)    (Markdown)       (Git-tracked)      (Research spec)
```

The forcing function is intentional: a human curates what matters before handing off to an agent.

Mike creates three digests. The first covers churn metrics from Datadog — monthly rates by segment, churn timing after signup, and feature adoption gaps. The data tells a stark story: 63% of churn happens in the first 30 days, and churned customers overwhelmingly never used project templates (78%), never invited a teammate (65%), and had zero integrations (82%). Retained customers are the mirror image.

The second digest covers Zendesk support patterns for the 400 customers who churned in December. Onboarding confusion dominates at 36% of tickets. Common phrases include "How do I set up my first project?" and "My team can't see my projects." Customers with three or more support tickets in their first week churn at 42%; customers with zero tickets churn at 6%.

The third digest summarizes exit survey responses. "Too hard to get started" leads at 33%, followed by missing features (22%) and competitor switches (18%). Verbatim quotes paint the picture: *"I spent 2 hours trying to set up my first project and gave up."*

Mike commits everything:

```bash
$ git add .chant/context/kpi-churn-q1/
$ git commit -m "Add Q1 churn KPI context digests"
```

## Week 2: Research

With context committed, Sarah creates a research spec for chant to analyze the data:

```bash
$ chant add "Analyze Q1 churn drivers from support, metrics, and survey data"
Created spec: 2026-01-13-001-xyz
```

She opens the spec with `chant edit 001` and sets the frontmatter to `type: research`, adds the `kpi-churn` and `q1-2026` labels, enables `approval: required: true`, and lists the three context digests under `informed_by`. She defines four research questions — top churn drivers by impact, which are addressable through product changes, expected reduction per intervention, and recommended priority — along with acceptance criteria requiring findings backed by data from the context files.

Then she hands it to an agent:

```bash
$ chant work 001
Working 001-xyz: Analyze Q1 churn drivers
→ Agent working in worktree /tmp/chant-001-xyz
...
✓ Completed in 3m 20s
```

The agent reads all three context digests, cross-references churn timing with support patterns, maps feature adoption gaps to exit survey reasons, and writes its findings. Three drivers emerge:

1. **Failed onboarding (~3.5pp impact)** — New users land on a blank page. No guided setup, no templates suggested, no team invite prompt.
2. **Missing integrations (~1.5pp impact)** — 82% of churned users had zero integrations. No Slack integration exists; competitor imports require manual CSV.
3. **Team discovery friction (~1.2pp impact)** — The invite flow is buried under Settings > Organization > Members.

The agent estimates that addressing all three could bring churn from 8% down to roughly 3.5%, exceeding the 5% target. It writes a priority matrix ranking onboarding as P0 (highest impact, medium effort) and the other two as P1.

The spec moves to `completed` status with `approval: status: pending`. The agent did its analysis — now the humans decide.

## Week 2: Approval Gate

Sarah reviews the findings:

```bash
$ chant show 001

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

After discussion, she rejects it. The integration impact estimate of 1.5pp looks optimistic without validation data:

```bash
$ chant reject 001 --by sarah \
  --reason "Integration impact estimate (1.5pp) needs validation. \
We have integration usage data from the beta program — agent should \
factor that in. Also need cost estimate for import work."

Spec 001-xyz REJECTED by sarah
```

The rejection is recorded in the spec's approval discussion section with a timestamp and the full reason. Mike adds the integration beta usage data to the context directory — a controlled trial of 200 customers showing that beta users with the integration enabled churned at 4.5% versus 10.8% in the control group, though self-selection bias means a conservative estimate is 2-3pp attributable to the integration.

```bash
$ git add .chant/context/kpi-churn-q1/integration-beta-usage.md
$ git commit -m "Add integration beta usage data for churn research"
```

Sarah resets the spec and re-runs it with the new context:

```bash
$ chant reset 001 --work
Spec 001-xyz reset to pending
Working 001-xyz: Analyze Q1 churn drivers
→ Agent working in worktree /tmp/chant-001-xyz
...
✓ Completed in 2m 45s (attempt 2)
```

The agent re-reads all context files, including the new beta data, and revises the integration estimate to ~2.0pp (midpoint of the conservative range). Sarah reviews again and approves:

```bash
$ chant approve 001 --by sarah

Spec 001-xyz APPROVED by sarah
```

The spec file now records the full discussion trail — rejection with reason, then approval — creating an audit record of how the team reached consensus. No implementation work was wasted on unvalidated estimates.

## Week 3: Implementation

With approved findings, Sarah creates a driver spec to coordinate three parallel interventions:

```bash
$ chant add "Reduce Q1 churn: implement approved interventions"
Created spec: 2026-01-16-002-abc
```

She opens it with `chant edit 002` and sets `type: driver` in the frontmatter, adds a `depends_on` reference to the research spec, and lists three member IDs. Then she creates the member specs:

```bash
$ chant add "Add onboarding wizard for new user setup"
Created spec: 2026-01-16-002-abc-1

$ chant add "Promote integration from beta to GA"
Created spec: 2026-01-16-002-abc-2

$ chant add "Surface team invite in sidebar and onboarding"
Created spec: 2026-01-16-002-abc-3
```

Each member spec gets `type: code` in its frontmatter, a `parent` reference to the driver, and focused acceptance criteria. The onboarding wizard spec targets a multi-step setup flow (template selection, team invite, integration connection) with a skip option on each step. The integration spec targets removing the beta feature flag and making the integration visible to all users. The team invite spec targets a persistent "Invite Team" button in the sidebar for accounts with fewer than three members.

All three members are independent — no `depends_on` between them — so they can execute in parallel:

```bash
$ chant work --parallel --label kpi-churn

Starting parallel execution (3 specs, label: kpi-churn)

[002-abc-1] Starting: Add onboarding wizard...
[002-abc-2] Starting: Promote integration...
[002-abc-3] Starting: Surface team invite...

[002-abc-3] Completed (4 files changed)
[002-abc-2] Completed (3 files changed)
[002-abc-1] Completed (6 files changed)

All 3 specs completed successfully.
```

Each spec executes in its own worktree, so there are no conflicts between agents. While they run, Sarah can check progress:

```bash
$ chant log 002-abc-1

[14:02] Reading research findings...
[14:03] Analyzing existing component structure...
[14:05] Creating wizard component with 3 steps...
[14:08] Writing tests...
[14:10] All tests passing. Committing changes.
```

After all members complete, Sarah merges the worktrees back to main:

```bash
$ chant merge --all --rebase

Merging 3 completed specs...

  002-abc-1 (onboarding wizard): Merged
  002-abc-2 (integration):       Merged
  002-abc-3 (team invite UX):    Merged

All specs merged to main.
```

The driver spec `002-abc` auto-completes when all its members are merged.

## Week 4: KPI Tracking

With interventions shipped, the team tracks whether churn actually drops. Sarah creates a reporting spec:

```bash
$ chant add "Daily churn KPI report"
Created spec: 2026-01-22-003-def
```

She edits it to set `type: task` in the frontmatter and adds a `schedule: daily` field. An important detail: the `schedule:` field in specs is documentation-only metadata. It records the team's intended frequency for human readers and CI pipeline authors. Chant does not read it, parse it, or trigger execution from it. Scheduling is handled by external tools.

Mike sets up a cron job (or a GitHub Actions workflow, or whatever automation the team already uses) to run the report daily:

```bash
# crontab -e
0 8 * * * cd /path/to/repo && chant work 003
```

Or in GitHub Actions:

```yaml
# .github/workflows/kpi-report.yml
name: Daily KPI Report
on:
  schedule:
    - cron: '0 8 * * *'
jobs:
  report:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run KPI report
        run: chant work 003
```

After the first week of daily runs, the agent produces reports like:

```
# Churn KPI Report — 2026-01-28

| Metric         | Baseline | Current | Target | Status    |
|----------------|----------|---------|--------|-----------|
| Monthly churn  | 8.0%     | 6.2%   | 5.0%   | On track  |
| 30-day activ.  | 62%      | 68%    | 75%    | Improving |
| Tickets/week   | 340      | 285    | <250   | Improving |

Intervention Adoption (First Week):
  Onboarding wizard completion: 71% of new signups
  Integration GA activations:   312 new connections
  Team invites via sidebar:     189 teams
```

For a quick summary across all KPI-labeled specs:

```bash
$ chant list --label kpi-churn

ID           Type      Status     Title
───────────  ────────  ─────────  ─────────────────────────────────────
001-xyz      research  completed  Analyze Q1 churn drivers
002-abc      driver    completed  Reduce Q1 churn (driver)
002-abc-1    code      completed  Add onboarding wizard
002-abc-2    code      completed  Promote integration to GA
002-abc-3    code      completed  Surface team invite
003-def      task      completed  Daily churn KPI report
```

## End-to-End Recap

The full workflow from OKR to measurable results:

```
Week 1: Mike creates data digests          → .chant/context/
Week 2: Chant analyzes churn drivers       → research spec (rejected, revised, approved)
Week 3: Chant implements 3 fixes parallel  → 3 code specs merged
Week 4: Daily reports track KPI progress   → 8% → 6.2% (on track for 5%)
```

All artifacts — data digests, research findings, approval discussions, implementation specs, and daily reports — are tracked in git through chant specs. Any team member can reconstruct the full decision chain from OKR to code change.

| Concept | What it does |
|---------|-------------|
| **Context directories** | Human-curated data in `.chant/context/` for agent consumption |
| **Research specs** | AI-driven analysis with `type: research` and `informed_by` references |
| **Approval gates** | `approval: required: true` blocks implementation until a human approves |
| **Reject/revise cycle** | `chant reject` with `--reason` surfaces missing data; `chant reset --work` re-runs |
| **Driver specs** | `type: driver` with `members` coordinates parallel implementation |
| **Parallel execution** | `chant work --parallel --label` runs independent specs simultaneously |
| **Worktree isolation** | Each agent works in its own git worktree — no conflicts |
| **Labels** | `--label kpi-churn` filters across all chant commands for initiative tracking |
| **Schedule metadata** | `schedule: daily` documents intent; external tools (cron, CI) trigger execution |

## Reference Implementation

The [artifacts directory](kpi-okr/artifacts/) contains concrete examples from this walkthrough:

- **Context digest** — `datadog-churn-metrics-2026-01.md`: Illustrative Datadog export digest
- **Research spec** — `research-spec-001-xyz.md`: Completed research spec with approval discussion
- **Driver spec** — `driver-spec-002-abc.md`: Driver coordinating three implementation members

## Further Reading

- [Lifecycle Walkthrough](lifecycle-walkthrough.md) — The ten-phase spec lifecycle with a code scenario
- [Research Workflows](../research.md) — Research spec patterns
- [Approval Workflow](../approval-workflow.md) — Approval gates and discussion
