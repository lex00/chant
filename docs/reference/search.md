# Search Command

> **Status: Implemented** - The `chant search` command performs simple text search across spec titles and body content.

## Overview

Search specs by title and body content with optional filters for date, status, type, and labels. Supports both interactive wizard mode (when run without arguments) and direct command-line search.

## Interactive Wizard Mode

Run without arguments to launch an interactive search wizard:

```bash
chant search
```

The wizard prompts for:
- **Search query** (required)
- **Search scope**: Title + Body, Title only, or Body only
- **Date range**: Any time, Last 7 days, Last 2 weeks, Last month, or Custom
- **Archive scope**: Include archived specs or not
- **Status filter**: Any, pending, ready, in_progress, completed, failed
- **Type filter**: Any, code, task, documentation, research

## Direct Search Mode

Search from the command line with optional filters:

```bash
chant search "authentication"           # Search title + body
chant search "auth" --title-only        # Search title only
chant search "TODO" --body-only         # Search body only
chant search "Auth" --case-sensitive    # Case-sensitive match
```

## Filtering

### By Status

```bash
chant search "api" --status pending      # Filter by status
chant search "auth" --status completed
chant search "fix" --status in_progress
```

Supported statuses: `pending`, `ready`, `in_progress`, `completed`, `failed`

### By Type

```bash
chant search "auth" --type code          # Filter by type
chant search "doc" --type documentation
```

Supported types: `code`, `task`, `documentation`, `research`

### By Labels

```bash
chant search "api" --label feature       # Filter by label
chant search "bug" --label urgent --label critical
```

Labels use OR logic - specs with any matching label are included.

### By Date Range

Use relative dates or absolute dates:

```bash
# Relative dates
chant search "bug" --since 7d            # Last 7 days
chant search "feature" --since 2w        # Last 2 weeks
chant search "api" --since 1m            # Last month
chant search "auth" --since 1w           # Last week

# Absolute dates
chant search "auth" --since 2026-01-20   # Since specific date
chant search "fix" --until 2026-01-15    # Until specific date

# Date ranges
chant search "api" --since 1w --until 3d # Between dates
```

Date is based on the spec ID date component (YYYY-MM-DD prefix).

### Archive Scope

By default, search includes both active and archived specs:

```bash
chant search "auth"                     # Both active and archived
chant search "auth" --active-only       # Only .chant/specs/
chant search "auth" --archived-only     # Only .chant/archive/
```

## Combined Filters

Combine multiple filters with AND logic:

```bash
# Pending code specs from last 2 weeks
chant search "auth" --status pending --type code --since 2w

# Failed API tasks with urgent label
chant search "api" --status failed --label urgent

# Recently completed documentation
chant search "doc" --status completed --type documentation --since 1w
```

## Text Matching Options

### Case-Sensitive Search

```bash
chant search "Auth" --case-sensitive     # Matches exact case
```

By default, search is case-insensitive.

### Title-Only Search

```bash
chant search "authentication" --title-only
```

Searches only the spec title (first `# ` heading in the spec body).

### Body-Only Search

```bash
chant search "TODO" --body-only
```

Searches only the spec body (everything after frontmatter).

## Output Format

Results show:
- Status icon (● for active, ◌ for archived)
- Spec ID (in cyan)
- Spec title
- Archive indicator `[archived]` for archived specs

```
● 2026-01-24-001-abc Add user authentication
● 2026-01-24-005-xyz Fix auth token refresh
◌ 2026-01-20-003-def [archived] Old auth implementation

Found 3 specs matching "auth"
```

## Examples

### Find pending authentication tasks

```bash
chant search "auth" --status pending
```

### Find recently completed code specs

```bash
chant search "" --status completed --type code --since 1w
```

### Find failed API tasks

```bash
chant search "api" --status failed
```

### Find critical bugs added this week

```bash
chant search "bug" --label critical --since 7d
```

### Search only active specs

```bash
chant search "refactor" --active-only
```

### Case-sensitive search for specific term

```bash
chant search "TODO" --body-only --case-sensitive
```
