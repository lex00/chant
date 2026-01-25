# Enterprise Features

## Overview

Chant supports enterprise environments through:

1. **Silent mode** - Personal use on shared repos
2. **Derived frontmatter** - Auto-populate fields from conventions
3. **Schema enforcement** - Required fields, validation

## Silent Mode

For developers on projects that don't officially use Chant:

```bash
chant init --silent
```

- `.chant/` added to `.git/info/exclude` (local only)
- No files committed to shared repo
- Personal AI workflow, invisible to team
- See [init.md](../reference/init.md) for details

## Derived Frontmatter

Enterprise teams have conventions: branch naming, path structure, ticket systems. Chant extracts metadata automatically.

### Configuration

```yaml
# config.md frontmatter
enterprise:
  derived:
    # Extract sprint from branch name
    sprint:
      from: branch
      pattern: "sprint/(\\d{4}-Q\\d-W\\d)"

    # Extract Jira key from branch name
    jira_key:
      from: branch
      pattern: "([A-Z]+-\\d+)"

    # Extract team from spec file path
    team:
      from: path
      pattern: "teams/(\\w+)/"

    # Extract component from path
    component:
      from: path
      pattern: "src/(\\w+)/"
```

### How It Works

Spec created on branch `sprint/2026-Q1-W4/PROJ-123-add-auth`:

```yaml
# Before completion (user writes)
---
status: pending
---

# Add authentication

...
```

```yaml
# After completion (auto-populated)
---
status: completed
completed_at: 2026-01-22T15:30:00Z
commit: a1b2c3d4

# Derived fields (auto-populated)
sprint: 2026-Q1-W4
jira_key: PROJ-123
team: platform
---
```

### Derivation Sources

| Source | Description | Example |
|--------|-------------|---------|
| `branch` | Current git branch name | `sprint/2026-Q1-W4/PROJ-123` |
| `path` | Spec file path | `teams/platform/tasks/...` |
| `env` | Environment variable | `$TEAM_NAME` |
| `git_user` | Git user.name | `alice@company.com` |

### Pattern Syntax

Standard regex with capture group:

```yaml
pattern: "prefix/([^/]+)/suffix"
#                 ^^^
#            Captured value
```

First capture group becomes the field value.

### Multiple Patterns

Try patterns in order, use first match:

```yaml
jira_key:
  from: branch
  patterns:
    - "([A-Z]+-\\d+)"           # PROJ-123
    - "issue-(\\d+)"            # issue-123 → 123
    - ".*"                       # Fallback: empty
  default: "UNKNOWN"
```

### Validation

Derived fields can be validated:

```yaml
enterprise:
  derived:
    team:
      from: path
      pattern: "teams/(\\w+)/"
      validate:
        enum: [platform, frontend, backend, infra]
```

Invalid derived value → warning, not error.

## Required Fields

Enforce fields for compliance:

```yaml
enterprise:
  required:
    - team
    - component
    - jira_key
```

```bash
$ chant lint
Error: Missing required field 'jira_key'
File: 2026-01-22-001-x7m.md

Enterprise policy requires: team, component, jira_key
```

## Audit Trail

Track who did what:

```yaml
# Auto-populated on completion
---
status: completed
completed_by: alice@company.com   # From git user
completed_at: 2026-01-22T15:30:00Z
agent_session: claude-abc123      # Agent identifier
---
```

## Integration Points

### Jira Sync (Future)

```yaml
enterprise:
  integrations:
    jira:
      enabled: true
      sync_status: true    # Update Jira when spec completes
```

### Slack Notifications (Future)

```yaml
enterprise:
  integrations:
    slack:
      webhook: $SLACK_WEBHOOK
      on_complete: true
```

## Enterprise vs Personal

| Feature | Personal | Enterprise |
|---------|----------|------------|
| Silent mode | ✓ | - |
| Derived fields | - | ✓ |
| Required fields | - | ✓ |
| Audit trail | - | ✓ |
| Integrations | - | Future |

Enterprise features are opt-in via `enterprise:` config block.
