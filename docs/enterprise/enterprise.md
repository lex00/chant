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

**Status: Implemented ✅**

Enterprise teams have conventions: branch naming, path structure, ticket systems. Chant extracts metadata automatically during spec completion.

### How It Works

Spec created on branch `sprint/2026-Q1-W4/PROJ-123-add-auth`:

```yaml
# Before completion (user writes)
---
status: pending
---

# Add authentication
```

After completion, derived fields are auto-populated:

```yaml
# After completion (auto-populated)
---
status: completed
completed_at: 2026-01-22T15:30:00Z
commit: a1b2c3d4

# Derived fields (auto-populated)
sprint: 2026-Q1-W4      [derived]
jira_key: PROJ-123      [derived]
team: platform          [derived]

derived_fields: [sprint, jira_key, team]
---
```

The `[derived]` indicator shows which fields were auto-populated. Derived field names are tracked in the `derived_fields` list for auditing.

### Configuration

Configure derivation rules in `.chant/config.md`:

```yaml
---
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
      validate:
        type: enum
        values: [api, auth, web, mobile]
---
```

### Derivation Sources

Chant supports **4 derivation sources**:

| Source | Description | Example | How to Use |
|--------|-------------|---------|-----------|
| `branch` | Current git branch name | `sprint/2026-Q1-W4/PROJ-123` | Extract from branch naming conventions |
| `path` | Spec file path | `.chant/specs/teams/platform/task.md` | Extract from directory structure |
| `env` | Environment variable | `TEAM_NAME=platform` | Extract from shell environment |
| `git_user` | Git user.name or user.email | `alice@company.com` | Extract from git config |

### Pattern Syntax

Patterns are standard regex with capture groups. The **first capture group** becomes the field value:

```yaml
pattern: "prefix/([^/]+)/suffix"
#                 ^^^^^^^^^^^
#            First capture group → field value
```

**Examples:**

| Pattern | Source | Value | Notes |
|---------|--------|-------|-------|
| `sprint/(\d{4}-Q\d-W\d)` | `sprint/2026-Q1-W4/task` | `2026-Q1-W4` | Sprint format |
| `([A-Z]+-\d+)` | `PROJ-123-auth` | `PROJ-123` | Jira ticket |
| `teams/(\w+)/` | `.chant/specs/teams/platform/task.md` | `platform` | Team directory |
| `(\w+)\.md` | `dashboard.md` | `dashboard` | Filename |

**Pattern Matching Rules:**
- If pattern doesn't match source → field is **omitted** (graceful failure)
- Invalid regex pattern → field is **omitted** (graceful failure)
- Multi-line sources (env vars, git config) are matched as single lines
- All matches are case-sensitive

### Validation Rules

Apply validation after successful extraction:

```yaml
enterprise:
  derived:
    team:
      from: path
      pattern: "teams/(\\w+)/"
      validate:
        type: enum
        values: [platform, frontend, backend, infra]
```

**Current Validation Types:**

- `enum`: Value must be in allowed list (case-sensitive)
  - Failure: Field **still included** but warning logged to stderr
  - Use to catch naming convention violations

**Behavior:**
- Valid value → field included, no warning
- Invalid value → field included, warning logged
- Validation never blocks derivation (graceful degradation)

### Manual Derivation

Re-run derivation on existing specs:

```bash
chant derive 2026-01-22-001-x7m   # Derive fields for one spec
chant derive --all                  # Derive for all specs
chant derive --all --dry-run        # Preview without modifying
```

Use cases:
- Add derivation rules and backfill existing specs
- Update derived values if branch names changed
- Fix invalid derived values

### Common Patterns

**Jira Tickets:**
```yaml
jira_key:
  from: branch
  pattern: "([A-Z]+-\\d+)"    # PROJ-123, AUTH-456, etc.
```

**Sprint Cycles:**
```yaml
sprint:
  from: branch
  pattern: "sprint/(\\d{4}-Q\\d-W\\d)"  # 2026-Q1-W1, 2026-Q1-W2, etc.
```

**Team Organization:**
```yaml
team:
  from: path
  pattern: "teams/(\\w+)/"     # Extract from directory structure
  validate:
    type: enum
    values: [platform, frontend, backend, infra]
```

**Multiple Derivation Sources:**
```yaml
team:
  from: env
  pattern: TEAM_NAME           # Fallback to environment variable
```

### Troubleshooting

**"Field not derived - pattern didn't match"**
- Verify the pattern is correct regex syntax
- Test pattern on actual values using online regex tools
- Ensure capture group `()` is present in pattern
- Check if source contains the expected value

**"Field has unexpected value"**
- Check if a more specific pattern should be used earlier
- Add validation rules to catch invalid formats
- Review the pattern with team to align on conventions

**"Validation warning for valid value"**
- Verify enum values are spelled exactly (case-sensitive)
- Check if all valid options are in the enum list
- Update enum list if new values are introduced

### UI Display

When viewing specs with `chant show`:

```
ID:            2026-01-22-001-x7m
Title:         Add authentication
Type:          code
Status:        completed

Team [derived]: platform
Jira_Key [derived]: PROJ-123
Custom_Field: manual_value
```

The `[derived]` indicator distinguishes automatically-populated fields from user-entered values.

## Required Fields

**Status: Implemented ✅**

Enforce required fields for compliance and audit:

```yaml
---
enterprise:
  required:
    - team
    - component
    - jira_key
---
```

When a spec is missing any required field, `chant lint` reports an error:

```bash
$ chant lint
Error: Missing required field 'jira_key'
File: 2026-01-22-001-x7m.md

ℹ Enterprise policy requires: team, component, jira_key
```

**Validation:**
- Required fields can be either **derived** or **explicitly set** in frontmatter
- Checked by `chant lint` command
- Blocks spec operations if missing
- Works with any frontmatter field name (standard or custom)

**Combined with Derivation:**

Most commonly, derivation rules populate required fields automatically:

```yaml
---
enterprise:
  derived:
    team:
      from: path
      pattern: "teams/(\\w+)/"
    jira_key:
      from: branch
      pattern: "([A-Z]+-\\d+)"

  required:        # These fields are enforced
    - team
    - jira_key
---
```

When a spec completes, derived fields are auto-populated, and `chant lint` verifies all required fields are present.

## Audit Trail

Track who did what and when:

```yaml
# Auto-populated on completion
---
status: completed
completed_by: alice@company.com   # From git user
completed_at: 2026-01-22T15:30:00Z
model: claude-haiku-4-5-20251001  # Agent model used
---
```

These fields are automatically populated by `chant finalize` or auto-finalize when a spec completes:
- `completed_by`: Git user name or email (from git config)
- `completed_at`: ISO 8601 timestamp of completion
- `model`: Model ID of agent that completed the spec

Combined with git history, this provides full audit trail of spec execution.

## Enterprise vs Personal

| Feature | Personal | Enterprise |
|---------|----------|------------|
| Silent mode | ✓ | - |
| Derived fields | ✓ | ✓ |
| Required fields | ✓ | ✓ |
| Audit trail | ✓ | ✓ |

All features are opt-in via the `enterprise:` config block in `.chant/config.md`.
