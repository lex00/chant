# Week 3: Implementation

The research is approved. Sarah now converts the three recommended interventions into a driver spec with member specs for parallel execution.

## Creating the Driver Spec

```bash
chant add "Reduce Q1 churn: implement approved interventions" --type driver
```

This creates spec `2026-01-16-002-abc`. Sarah edits it to define the members:

**File: `.chant/specs/2026-01-16-002-abc.md`**

```yaml
---
type: driver
status: ready
labels:
  - kpi-churn
  - q1-2026
depends_on:
  - 2026-01-13-001-xyz
members:
  - 2026-01-16-002-abc-1
  - 2026-01-16-002-abc-2
  - 2026-01-16-002-abc-3
---

# Reduce Q1 churn: implement approved interventions

Based on research findings in spec 2026-01-13-001-xyz.

## Interventions

1. **Onboarding wizard** — Step-by-step setup flow for new users (P0)
2. **Integration GA** — Promote beta to general availability (P1)
3. **Team invite UX** — Surface invite flow in onboarding and sidebar (P1)

## Acceptance Criteria

- [ ] All member specs completed
- [ ] Combined interventions deployed to staging
- [ ] Churn tracking baseline established
```

## Member Specs

Sarah creates three focused member specs. Each one targets a single intervention.

### Member 1: Onboarding Wizard

```bash
chant add "Add onboarding wizard for new user setup"
```

**File: `.chant/specs/2026-01-16-002-abc-1.md`**

```yaml
---
type: code
status: ready
labels:
  - kpi-churn
  - q1-2026
  - onboarding
parent: 2026-01-16-002-abc
informed_by:
  - .chant/context/kpi-churn-q1/research-findings.md
target_files:
  - src/components/onboarding/wizard.tsx
  - src/components/onboarding/steps.tsx
  - tests/onboarding/wizard.test.tsx
---

# Add onboarding wizard for new user setup

## Problem

78% of churned users never used project templates. New users land on a
blank page after signup with no guidance.

## Solution

Add a multi-step onboarding wizard that appears on first login:
1. Create first project (from template)
2. Invite team members
3. Connect an integration

## Acceptance Criteria

- [ ] Wizard appears on first login for new accounts
- [ ] Step 1: Template selection with 3 starter templates
- [ ] Step 2: Team invite with email input
- [ ] Step 3: Integration connection
- [ ] "Skip" option available on each step
- [ ] Wizard state persisted (resume if closed early)
- [ ] Tests passing
```

### Member 2: Integration GA

```bash
chant add "Promote integration from beta to GA"
```

**File: `.chant/specs/2026-01-16-002-abc-2.md`**

```yaml
---
type: code
status: ready
labels:
  - kpi-churn
  - q1-2026
  - integrations
parent: 2026-01-16-002-abc
target_files:
  - src/integrations/config.ts
  - src/integrations/feature-flag.ts
  - tests/integrations/integration.test.ts
---

# Promote integration from beta to GA

## Problem

Integration beta users show 4.5% churn vs 10.8% control. Integration exists
but is gated behind a beta flag.

## Solution

Remove the beta feature flag and enable integration for all users.
Add integration card to the onboarding wizard and settings page.

## Acceptance Criteria

- [ ] Beta feature flag removed
- [ ] Integration visible to all users in settings
- [ ] Integration card added to integrations page
- [ ] Existing beta users unaffected (no re-setup required)
- [ ] Tests passing
```

### Member 3: Team Invite UX

```bash
chant add "Surface team invite in sidebar and onboarding"
```

**File: `.chant/specs/2026-01-16-002-abc-3.md`**

```yaml
---
type: code
status: ready
labels:
  - kpi-churn
  - q1-2026
  - team-ux
parent: 2026-01-16-002-abc
target_files:
  - src/components/sidebar/invite-button.tsx
  - src/components/onboarding/team-step.tsx
  - tests/sidebar/invite.test.tsx
---

# Surface team invite in sidebar and onboarding

## Problem

65% of churned users never invited a teammate. The invite flow is buried
under Settings > Organization > Members.

## Solution

Add a persistent "Invite Team" button to the sidebar and integrate
the invite step into the onboarding wizard.

## Acceptance Criteria

- [ ] "Invite Team" button visible in sidebar for accounts with <3 members
- [ ] Button opens invite modal with email input
- [ ] Invite step integrated into onboarding wizard (Step 2)
- [ ] Button hides after team reaches 3+ members
- [ ] Tests passing
```

## Parallel Execution

All three member specs share the `kpi-churn` label. Sarah executes them in parallel:

```bash
chant work --parallel --label kpi-churn
```

```
Starting parallel execution (3 specs, label: kpi-churn)

[002-abc-1] Starting: Add onboarding wizard...
[002-abc-2] Starting: Promote integration...
[002-abc-3] Starting: Surface team invite...

[002-abc-3] Completed (4 files changed)
[002-abc-2] Completed (3 files changed)
[002-abc-1] Completed (6 files changed)

All 3 specs completed successfully.
```

Each spec executes in its own worktree, so there are no conflicts between agents.

## Monitoring Progress

While agents run, Sarah checks progress:

```bash
chant log 2026-01-16-002-abc-1
```

```
[14:02] Reading research findings...
[14:03] Analyzing existing component structure...
[14:05] Creating wizard component with 3 steps...
[14:08] Writing tests...
[14:10] All tests passing. Committing changes.
```

## Merging Results

After all member specs complete, Sarah merges the worktrees back to main:

```bash
chant merge --all --rebase
```

```
Merging 3 completed specs...

  002-abc-1 (onboarding wizard): Merged ✓
  002-abc-2 (integration):       Merged ✓
  002-abc-3 (team invite UX):    Merged ✓

All specs merged to main.
```

The driver spec `002-abc` auto-completes when all its members are merged.

## What's Next

With all interventions merged, the team sets up tracking to measure impact:

**[Reporting](06-reporting.md)** — Daily KPI tracking, activity feeds, and dashboards.
