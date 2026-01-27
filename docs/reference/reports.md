# Export

> **Status: Implemented** ✅
>
> The `chant export` command is fully implemented as of v0.3.0.
> You can export specs to JSON, CSV, and Markdown formats with filtering options.

## Philosophy

Chant exports spec data. External tools make reports.

```
Chant exports raw data → Agents/tools process → Human-readable reports
```

This keeps chant focused: spec execution, not report generation.

## Basic Export

```bash
chant export                          # All specs, JSON
chant export --format json            # Explicit JSON
chant export --format csv             # Spreadsheet-friendly
chant export --format markdown        # Simple list
```

## Date Filtering

```bash
# Completed in date range
chant export --from 2026-01-01 --to 2026-01-31

# Last N days
chant export --last 30d

# Since specific date
chant export --since 2026-01-01

# Completed this week
chant export --last 7d --status completed
```

## Search Filtering

Use standard search syntax:

```bash
chant export --search "status:completed"
chant export --search "label:security"
chant export --search "project:auth"
chant export --search "label:security completed_at:>2026-01-01"
```

## Output Formats

### JSON (Default)

```bash
$ chant export --last 7d --format json
```

```json
{
  "exported_at": "2026-01-22T15:00:00Z",
  "query": "completed_at:>2026-01-15",
  "count": 12,
  "specs": [
    {
      "id": "2026-01-22-001-x7m",
      "title": "Add authentication",
      "status": "completed",
      "created_at": "2026-01-20T10:00:00Z",
      "completed_at": "2026-01-20T11:30:00Z",
      "labels": ["feature", "auth"],
      "commit": "abc123",
      "cost": {
        "tokens": 15432,
        "usd": 1.23
      }
    }
  ]
}
```

### CSV

```bash
$ chant export --last 7d --format csv
```

```csv
id,title,status,created_at,completed_at,labels,commit,tokens,cost_usd
2026-01-22-001-x7m,Add authentication,completed,2026-01-20T10:00:00Z,2026-01-20T11:30:00Z,"feature,auth",abc123,15432,1.23
```

### Markdown

```bash
$ chant export --last 7d --format markdown
```

```markdown
# Spec Export
Query: completed_at:>2026-01-15
Count: 12

## 2026-01-22-001-x7m: Add authentication
- Status: completed
- Completed: 2026-01-20
- Labels: feature, auth
- Commit: abc123

## 2026-01-22-002-q2n: Fix payment bug
- Status: completed
- Completed: 2026-01-21
- Labels: bug, payments
- Commit: def456
```

## What Gets Exported

| Field | Description |
|-------|-------------|
| `id` | Spec ID |
| `title` | First heading from spec body |
| `status` | Current status |
| `created_at` | Creation timestamp |
| `completed_at` | Completion timestamp (if completed) |
| `labels` | Labels array |
| `project` | Project prefix (if set) |
| `commit` | Git commit hash (if completed) |
| `branch` | Git branch (if created) |
| `cost.tokens` | Token count (if tracked) |
| `cost.usd` | Cost in USD (if tracked) |
| `duration_s` | Execution duration in seconds |
| `agent` | Agent/model used |
| `prompt` | Prompt used |

## Selective Fields

```bash
# Only specific fields
chant export --fields id,title,status,completed_at

# Exclude cost data
chant export --exclude cost
```

## Piping to Tools

```bash
# Count by status
chant export --format json | jq '.specs | group_by(.status) | map({status: .[0].status, count: length})'

# Total cost
chant export --format json | jq '[.specs[].cost.usd] | add'

# Have agent summarize
chant export --last 7d | claude "Summarize this week's completed specs for a standup"

# Generate release notes
chant export --search "label:feature" --from 2026-01-01 | claude "Write release notes from these specs"
```

## Audit Export

For compliance, include full spec bodies:

```bash
chant export --full                   # Include spec body content
chant export --full --include-output  # Include agent output too
```

```json
{
  "specs": [
    {
      "id": "2026-01-22-001-x7m",
      "title": "Add authentication",
      "body": "# Add authentication\n\n## Acceptance Criteria\n...",
      "output": "[10:15:32] Reading src/auth/handler.go\n..."
    }
  ]
}
```

## Git Integration

Export includes git metadata:

```bash
chant export --git-details
```

```json
{
  "specs": [
    {
      "id": "2026-01-22-001-x7m",
      "git": {
        "commit": "abc123",
        "branch": "chant/2026-01-22-001-x7m",
        "author": "alice@example.com",
        "files_changed": ["src/auth/handler.go", "src/auth/jwt.go"],
        "insertions": 145,
        "deletions": 12
      }
    }
  ]
}
```

## Configuration

```yaml
# config.md
export:
  default_format: json
  include_cost: true
  include_git: false
```

## Examples

### Weekly Summary for Standup

```bash
chant export --last 7d --status completed --format json | \
  claude "Create a bullet-point summary of what was accomplished this week"
```

### Security Audit Data

```bash
chant export --search "label:security" --full --git-details > audit-data.json
```

### Cost Tracking

```bash
chant export --last 30d --format csv --fields id,title,tokens,cost_usd > monthly-costs.csv
```

### Release Notes Input

```bash
chant export --from 2026-01-01 --to 2026-01-31 --search "label:feature OR label:bugfix" | \
  claude "Write customer-facing release notes from these specs"
```

## Why Not Built-in Reports?

Chant focuses on spec execution. Report generation is:

1. **Highly variable** - Every team wants different formats
2. **Better done by agents** - LLMs excel at summarization
3. **Already solved** - jq, csvkit, pandas, etc.

Export gives you the raw data. Use the right tool for presentation.
