# Search Syntax

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
chant search "project:auth"
chant search "label:urgent"
chant search "prompt:tdd"
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
| `project` | keyword | Project prefix |
| `label` | keyword[] | Labels (multi-value) |
| `prompt` | keyword | Prompt name |
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

### Find specs touching auth files

```bash
chant search "body:src/auth/*"
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

## Semantic Search

Vector-based similarity search finds conceptually related specs.

### Why Semantic Search?

Keyword search finds exact matches:
```bash
chant search "transformer"      # Finds "transformer"
chant search "attention"        # Finds "attention"
```

Semantic search finds conceptual matches:
```bash
chant search --semantic "efficiency optimization"
# Finds specs about:
#   - "performance improvements"
#   - "reducing latency"
#   - "memory optimization"
# Even if they don't contain "efficiency"
```

### Zero Setup

Semantic search is built into the `chant` binary. No external services.

| What | How |
|------|-----|
| Vector store | arroy (compiled in) |
| Embeddings | fastembed-rs (compiled in) |
| Storage | LMDB (compiled in) |
| Model | Auto-downloads on first use |

First use:
```bash
$ chant search --semantic "auth patterns"
Downloading embedding model (BGE-small-en)... 50MB
[results]
```

After that, fully offline. No API keys, no Docker, no Python.

### Use Cases

**Find similar specs:**
```bash
chant search --semantic "user login flow" --limit 5
```

**Research workflows:**
```bash
chant search --semantic "how does authentication work"
# Finds specs and docs about auth, even without exact terms
```

**Explore related work:**
```bash
chant search --semantic --similar-to 001
# Find specs conceptually similar to spec 001
```

### Combining Keyword and Semantic

```bash
# Filter by status, then rank by similarity
chant search "status:pending" --semantic "performance"

# Hybrid scoring (both keyword and semantic)
chant search "auth" --hybrid --semantic "security patterns"
```

### Configuration

```yaml
# config.md
search:
  semantic:
    enabled: true              # Default: true
    model: BGE-small-en        # Embedding model
    threshold: 0.5             # Minimum similarity score
```

### Storage

Semantic index stored in `.chant/.store/vectors/`:
```
.chant/.store/
├── tantivy/       # Keyword index
└── vectors/       # Semantic index (arroy/LMDB)
```

Both are memory-mapped, both scale to millions of documents.
