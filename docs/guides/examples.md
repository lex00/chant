# Examples

## Overview

Complete example configurations for different deployment scenarios.

## Solo Developer

Single developer, branch mode disabled, simple workflow.

### Directory Structure

```
my-project/
├── .chant/
│   ├── config.md
│   ├── specs/
│   │   └── 2026-01-22-001-x7m.md
│   └── prompts/
│       └── standard.md
└── src/
```

### Config

```markdown
# .chant/config.md
---
project:
  name: my-project

defaults:
  prompt: standard
  branch: false
---

# My Project

Branch mode disabled. Simple workflow.
```

### Sample Spec

```markdown
# .chant/specs/2026-01-22-001-x7m.md
---
status: pending
created: 2026-01-22
---

# Add user settings page

Create a settings page where users can update their profile.

## Acceptance Criteria

- [ ] Settings page accessible at /settings
- [ ] Users can update display name
- [ ] Users can change email
- [ ] Form validates input
- [ ] Changes persist to database

## Target Files

- src/pages/settings.tsx
- src/api/settings.ts
```

### Workflow

```bash
chant add "Add user settings page"
chant work 2026-01-22-001-x7m
# Agent works, commits directly to main
```

---

## Team Collaboration

Multiple developers, branch-based workflow.

### Directory Structure

```
team-project/
├── .chant/
│   ├── config.md
│   ├── specs/
│   ├── prompts/
│   │   ├── standard.md
│   │   └── tdd.md
│   ├── hooks/
│   │   └── post_work.md
│   └── notifications/
│       ├── on_complete.md
│       └── on_fail.md
└── src/
```

### Config

```markdown
# .chant/config.md
---
project:
  name: team-project

defaults:
  prompt: standard
  branch: true

notifications:
  on_complete: .chant/notifications/on_complete.md
  on_fail: .chant/notifications/on_fail.md

hooks:
  post_work: .chant/hooks/post_work.md
---

# Team Project

All changes via branch. Slack notifications on completion.
```

### Post-Work Hook

```markdown
# .chant/hooks/post_work.md
---
name: post_work
---

Before completing, ensure:

1. All tests pass: `npm test`
2. Linting passes: `npm run lint`
3. Build succeeds: `npm run build`

If any fail, fix the issues before marking complete.
```

### Notification Template

```markdown
# .chant/notifications/on_complete.md
---
channel: slack
webhook: ${SLACK_WEBHOOK_URL}
---

:white_check_mark: *{{spec.id}}* completed

> {{spec.title}}

_{{duration}} · {{prompt}} prompt_
```

### Workflow

```bash
# Developer creates spec
chant add "Implement OAuth login"

# Execute - creates branch
chant work 2026-01-22-001-x7m

# Team reviews changes
# Merge triggers spec completion
# Slack notification sent
```

---

## Enterprise Silent

Personal chant on corporate repo with derived fields.

### Directory Structure

```
corporate-monorepo/
├── .gitignore                    # Includes .chant-local/
├── .chant-local/                 # Gitignored, personal only
│   ├── config.md
│   ├── specs/
│   └── prompts/
└── packages/
    ├── auth/
    └── payments/
```

### Config

```markdown
# .chant-local/config.md
---
project:
  name: corporate-silent

defaults:
  prompt: standard
  branch: true
  branch_prefix: "alex/"      # Personal prefix

enterprise:
  derived:
    jira_key:
      from: branch
      pattern: "([A-Z]+-\\d+)"
    sprint:
      from: branch
      pattern: "sprint/(\\d{4}-Q\\d-W\\d)"
---

# Personal Chant Setup

Silent mode - not visible to team.
Branch naming extracts Jira keys automatically.
```

### Workflow

```bash
# Initialize in silent mode
chant init --silent

# Work normally
chant add "AUTH-123: Fix token refresh"
chant work 2026-01-22-001-x7m
# Creates branch: alex/AUTH-123-fix-token-refresh
# Spec frontmatter auto-populated with jira_key: AUTH-123
```

---

## Scale: Monorepo with K8s

Large monorepo with daemon, workers, metrics.

### Directory Structure

```
monorepo/
├── .chant/
│   ├── config.md
│   ├── specs/
│   │   ├── auth-2026-01-22-001-x7m.md
│   │   └── payments-2026-01-22-001-abc.md
│   ├── prompts/
│   └── providers/
└── packages/
    ├── auth/
    ├── payments/
    └── frontend/
```

### Config

```markdown
# .chant/config.md
---
project:
  name: monorepo

defaults:
  prompt: standard
  branch: true

scale:
  id_prefix:
    from: path
    pattern: "packages/([^/]+)/"

  daemon:
    enabled: true
    metrics_port: 9090
    api_port: 8080

  worktree:
    sparse: true
    pool_size: 10

  limits:
    max_agents: 50
    max_per_project: 10
    spec_timeout: 30m

  costs:
    daily_limit_usd: 500.00
    auth:
      daily_limit_usd: 100.00
    payments:
      daily_limit_usd: 150.00
---

# Monorepo Configuration

Daemon mode enabled. Per-project limits.
```

### K8s Deployment

```yaml
# k8s/daemon.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chant-daemon
spec:
  replicas: 1
  template:
    spec:
      containers:
        - name: daemon
          image: chant:latest
          command: ["chant", "daemon", "start"]
          ports:
            - containerPort: 8080
            - containerPort: 9090
          volumeMounts:
            - name: repo
              mountPath: /repo
      volumes:
        - name: repo
          persistentVolumeClaim:
            claimName: repo-pvc
---
# k8s/worker.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chant-worker-auth
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: worker
          image: chant:latest
          command: ["chant", "agent", "worker", "--project", "auth"]
          env:
            - name: CHANT_DAEMON
              value: "http://chant-daemon:8080"
            - name: AGENT_API_KEY
              valueFrom:
                secretKeyRef:
                  name: chant-secrets
                  key: api-key
```

### Grafana Dashboard

```yaml
# grafana/chant-dashboard.yaml
panels:
  - title: "Specs by Status"
    type: piechart
    query: "chant_specs_total"

  - title: "Active Workers"
    type: gauge
    query: "chant_agents_active"

  - title: "Completions/Hour"
    type: graph
    query: "rate(chant_agent_completions_total[1h]) * 3600"

  - title: "Cost Today"
    type: stat
    query: "chant_costs_usd_total"
```

### Workflow

```bash
# Daemon running in K8s

# Add specs (from CI or developer)
chant add "Implement new auth flow" --project auth
chant add "Fix payment webhook" --project payments

# Workers automatically pick up and execute
# Monitor in Grafana dashboard
# Notifications via Slack
```

---

## Approval-Gated Team Development

Team with approval requirements for spec execution.

### Config

```markdown
# .chant/config.md
---
project:
  name: team-app

defaults:
  prompt: standard
  branch: true

approval:
  rejection_action: dependency
---

# Team App

All risky specs require approval before execution.
Rejected specs automatically create fix specs.
```

### Workflow

```bash
# Developer creates spec requiring approval
chant add "Migrate user table to new schema" --needs-approval

# Tech lead reviews the spec
chant show 001-abc

# Option A: Approve
chant approve 001-abc --by tech-lead
chant work 001-abc

# Option B: Reject with reason
chant reject 001-abc --by tech-lead --reason "Need rollback plan first"
# With dependency mode, a fix spec is auto-created and original is blocked

# Monitor team activity
chant activity --since 1d

# Find specs waiting for approval
chant list --approval pending

# Find specs a person has been involved with
chant list --mentions tech-lead
```

### Sample Spec with Approval

```markdown
# .chant/specs/2026-01-28-001-abc.md
---
status: pending
approval:
  required: true
  status: approved
  by: tech-lead
  at: 2026-01-28T14:30:00Z
---

# Migrate user table to new schema

## Acceptance Criteria

- [ ] Migration script handles existing data
- [ ] Rollback script tested
- [ ] Zero-downtime migration verified

## Approval Discussion

**tech-lead** - 2026-01-28 14:30 - APPROVED
```

---

## Self-Bootstrap

Building chant using chant.

### Bootstrap Sequence

```bash
# 1. Manual init (chicken-egg)
mkdir -p .chant/tasks .chant/prompts
cat > .chant/config.md << 'EOF'
---
project:
  name: chant
---
# Chant
EOF

# 2. Create first spec manually
cat > .chant/specs/2026-01-22-001-x7m.md << 'EOF'
---
status: pending
---
# Implement chant init command
EOF

# 3. Implement init command (manually or with other agent)

# 4. Now use chant for everything
chant add "Implement chant add command"
chant work 2026-01-22-002-abc

chant add "Implement chant work command"
chant work 2026-01-22-003-def

# 5. Chant is now self-sustaining
```

### Dogfooding Specs

```bash
chant add "Add spec parser"
chant add "Implement Tantivy search"
chant add "Add worktree isolation"
chant add "Implement daemon mode"
chant add "Add Prometheus metrics"
# ... all features as specs
```
