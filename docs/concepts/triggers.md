# Triggers

## Overview

Triggers allow specs to wait for external conditions before becoming ready. A spec with triggers starts as `status: waiting` instead of `pending`.

```yaml
---
status: waiting
triggers:
  - type: pr_merged
    pr: "#123"
---
```

When all triggers are satisfied, the spec automatically transitions to `pending` and becomes eligible for execution.

## Trigger Types

### pr_merged

Wait for a pull request to merge:

```yaml
triggers:
  - type: pr_merged
    pr: "#123"                    # PR number
  - type: pr_merged
    pr: "owner/repo#456"          # Cross-repo PR
```

### file_changed

Watch for file changes (via git or filesystem):

```yaml
triggers:
  - type: file_changed
    path: "src/api/schema.graphql"
  - type: file_changed
    path: "packages/*/package.json"   # Glob patterns
    branch: main                       # Only on specific branch
```

### schedule

Cron-based scheduling:

```yaml
triggers:
  - type: schedule
    cron: "0 2 * * *"              # Daily at 2am
  - type: schedule
    cron: "0 0 * * 0"              # Weekly on Sunday
    timezone: America/New_York
```

### spec_completed

Wait for another spec to complete:

```yaml
triggers:
  - type: spec_completed
    spec: "2026-01-22-001-x7m"
  - type: spec_completed
    spec: "2026-01-22-001-x7m"
    require: success               # success | any (default: success)
```

Note: This differs from `depends_on`. Dependencies block execution order; triggers control when a spec becomes ready at all.

### label_added

Wait for a label on a GitHub/GitLab issue:

```yaml
triggers:
  - type: label_added
    label: "ready-for-automation"
    source: github                 # github | gitlab
```

### webhook

External system calls in:

```yaml
triggers:
  - type: webhook
    endpoint: /hooks/deploy-ready
    secret: ${WEBHOOK_SECRET}      # Optional validation
```

Webhook URL: `https://chant.example.com/hooks/deploy-ready?spec=2026-01-22-001-x7m`

### manual

Explicit trigger via CLI:

```yaml
triggers:
  - type: manual
    description: "Trigger after QA approval"
```

```bash
$ chant trigger 001
Spec 001 triggered manually.
Status: waiting → pending
```

## Multiple Triggers

### All (default)

All triggers must be satisfied:

```yaml
triggers:
  mode: all                        # default
  conditions:
    - type: pr_merged
      pr: "#123"
    - type: spec_completed
      spec: "2026-01-22-001-x7m"
```

### Any

Any trigger satisfies:

```yaml
triggers:
  mode: any
  conditions:
    - type: schedule
      cron: "0 2 * * *"
    - type: manual
```

## Spec States with Triggers

```
waiting → pending → in_progress → completed
    │                           ↘ failed
    │
    └── triggers not yet satisfied
```

- **waiting**: Has triggers, not all satisfied
- **pending**: No triggers OR all triggers satisfied, ready for execution

## Configuration

### Global Defaults

```yaml
# config.md
triggers:
  # Poll interval for checking triggers
  poll_interval: 60s

  # Webhook server (if using webhook triggers)
  webhook:
    enabled: true
    port: 8081
    path_prefix: /hooks

  # GitHub integration for pr_merged, label_added
  github:
    token: ${GITHUB_TOKEN}
    webhook_secret: ${GITHUB_WEBHOOK_SECRET}
```

### Daemon Support

Triggers require the daemon for efficient monitoring:

```bash
chant daemon start --triggers
```

Without daemon, triggers are checked on CLI commands (less responsive).

## CLI Commands

```bash
# List specs with triggers
chant list --waiting

# Show trigger status
chant show 001 --triggers
Triggers for spec 001:
  ✓ pr_merged: #123 (merged 2026-01-22)
  ○ spec_completed: 002 (in_progress)
Status: waiting (1/2 triggers satisfied)

# Manually trigger a spec
chant trigger 001

# Check all trigger statuses
chant triggers status
Waiting specs: 5
  001: 1/2 triggers satisfied
  002: 0/1 triggers satisfied
  ...
```

## Examples

### Deploy After PR Merge

```yaml
---
status: waiting
triggers:
  - type: pr_merged
    pr: "#123"
labels: [deploy]
---

# Deploy v2.1.0

Deploy the merged changes to production.
```

### Nightly Maintenance

```yaml
---
status: waiting
triggers:
  - type: schedule
    cron: "0 3 * * *"
labels: [maintenance, autonomous]
---

# Nightly dependency updates

Check for outdated dependencies and create update PRs.
```

### Pipeline Stage

```yaml
---
status: waiting
triggers:
  - type: spec_completed
    spec: "2026-01-22-001-x7m"    # Build spec
  - type: spec_completed
    spec: "2026-01-22-002-q2n"    # Test spec
---

# Deploy to staging

Deploy after build and test complete.
```

### External System Integration

```yaml
---
status: waiting
triggers:
  - type: webhook
    endpoint: /hooks/jira-approved
---

# Implement JIRA-1234

Implement feature after Jira ticket is approved.

Webhook will be called by Jira automation when ticket
moves to "Approved" status.
```

## Comparison: Triggers vs Dependencies

| Aspect | Triggers | Dependencies |
|--------|----------|--------------|
| Purpose | When spec becomes ready | Execution order |
| Status effect | `waiting` → `pending` | Blocks `pending` → `in_progress` |
| External events | Yes (PR, webhook, schedule) | No (specs only) |
| Checked by | Daemon continuously | At execution time |

Use triggers for: "Don't even consider this until X happens"
Use dependencies for: "Do this after spec Y completes"

## Design Philosophy

Chant triggers are deliberately simple:

- YAML config (no code)
- Daemon monitors (no resource usage while waiting)
- Simple predicates (not complex conditions)

For complex conditions, use a spec that checks conditions and creates follow-up specs.
