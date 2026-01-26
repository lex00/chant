# Search Syntax (Planned)

> **Status: Planned** - The `chant search` command is on the roadmap but not yet implemented. Spec filtering is currently available via `chant list --label`.

## Overview

Chant uses Tantivy for full-text search with structured query syntax.

## Basic Search

```bash
chant search "authentication bug"
```

Searches spec body for matching terms.

## Query Syntax

### Field Search

Search specific frontmatter fields:

```bash
chant search "status:pending"
chant search "type:code"
chant search "project:auth"
chant search "label:urgent"
chant search "prompt:tdd"
chant search "target_files:src/lib.rs"
```

### ID Search

```bash
chant search "id:2026-01-22-001"     # Prefix match
chant search "id:2026-01-22-*"       # Wildcard
chant search "id:auth-*"             # Project prefix
```

### Git Hash Search

Find specs by commit:

```bash
chant search "commit:abc123"         # Specs with this commit
chant search "branch:chant/2026-*"   # Specs on branch
```

### Body Search

Fuzzy text search on spec body:

```bash
chant search "body:authentication"
chant search "body:fix bug"          # Multiple terms (AND)
chant search "body:\"fix the bug\""  # Exact phrase
```

### Combined Queries

```bash
# Pending auth tasks mentioning OAuth
chant search "status:pending project:auth OAuth"

# Failed tasks from this week
chant search "status:failed created:2026-01-*"

# Urgent bugs
chant search "label:urgent label:bug"
```

## Operators

### Boolean

```bash
chant search "auth AND OAuth"        # Both terms
chant search "auth OR OAuth"         # Either term
chant search "auth NOT OAuth"        # Exclude term
chant search "(auth OR login) bug"   # Grouping
```

### Wildcards

```bash
chant search "auth*"                 # Prefix
chant search "*tion"                 # Suffix (slow)
chant search "auth?n"                # Single char
```

### Ranges

```bash
chant search "created:[2026-01-01 TO 2026-01-31]"
chant search "cost_usd:[0 TO 1.00]"
chant search "tokens:[* TO 10000]"   # Up to 10k tokens
```

### Fuzzy

```bash
chant search "authentcation~"        # Typo tolerance
chant search "auth~2"                # Edit distance 2
```

## Indexed Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | keyword | Spec ID (exact match) |
| `status` | keyword | pending, in_progress, completed, failed |
| `type` | keyword | code, task, driver, group |
| `project` | keyword | Project prefix |
| `label` | keyword[] | Labels (multi-value) |
| `prompt` | keyword | Prompt name |
| `target_files` | keyword[] | Target file paths (multi-value) |
| `created` | date | Creation date |
| `updated` | date | Last update |
| `completed` | date | Completion date |
| `body` | text | Full spec body (analyzed) |
| `subject` | text | First line of body |
| `commit` | keyword | Git commit hash |
| `branch` | keyword | Git branch |
| `cost_usd` | float | Cost in USD |
| `tokens` | integer | Token count |
| `duration_s` | integer | Execution duration |
| `group` | keyword | Group driver ID (find all members) |
| `depends_on` | keyword[] | Dependency IDs |

## Output Formats

```bash
# Default (table)
chant search "status:pending"

# JSON (for scripting)
chant search "status:pending" --json

# IDs only
chant search "status:pending" --ids-only

# Count only
chant search "status:pending" --count
```

## Sorting

```bash
chant search "status:pending" --sort created:desc
chant search "status:completed" --sort cost_usd:desc
chant search "project:auth" --sort id:asc
```

## Pagination

```bash
chant search "status:pending" --limit 10 --offset 20
```

## Examples

### Find failed tasks from today

```bash
chant search "status:failed created:2026-01-22"
```

### Find expensive specs

```bash
chant search "cost_usd:[5 TO *]" --sort cost_usd:desc
```

### Find specs targeting specific files

```bash
chant search "target_files:src/auth.rs"
chant search "target_files:src/lib.rs"
```

### Find incomplete members of a group

```bash
chant search "group:2026-01-22-001-x7m NOT status:completed"
```

### Find specs by commit

```bash
chant search "commit:abc123def"
```

### Find blocked specs

```bash
# Specs with unmet dependencies
chant search "depends_on:* NOT status:completed"
```

### Find stale in-progress specs

```bash
chant search "status:in_progress updated:[* TO 2026-01-21]"
```

## Full-Text Analysis

Body field is analyzed with:
- Lowercase normalization
- English stemming (running → run)
- Stop word removal (the, a, is)

```bash
# These find the same specs:
chant search "body:running"
chant search "body:runs"
chant search "body:run"
```

## CLI Shortcuts

Common searches have shortcuts:

```bash
chant ready                          # status:pending (deps met)
chant list --status pending          # status:pending
chant list --failed                  # status:failed
chant list --project auth            # project:auth
```

## Daemon Mode

With daemon, search hits hot index (instant). Without daemon, builds index on-demand (slower for first query).

```bash
# Daemon running: ~5ms
# No daemon: ~200ms (first), ~50ms (cached)
```

## Storage

Tantivy index stored in `.chant/.index/`:
```
.chant/
├── specs/         # Spec files
└── .index/        # Tantivy search index
```

The index is memory-mapped and scales to millions of documents.
