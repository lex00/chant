# Spec Types

Chant supports six spec types, each with different behaviors for execution, verification, and drift detection.

## Overview

| Type | Purpose | Drift Trigger |
|------|---------|---------------|
| `code` | Implement features, fix bugs | Acceptance criteria fail |
| `task` | Manual work, prompts, config | Acceptance criteria fail |
| `driver` | Coordinate multiple specs | Members incomplete |
| `group` | Alias for `driver` | Members incomplete |
| `documentation` | Generate docs from source | `tracks:` files change |
| `research` | Analysis, synthesis, findings | `origin:` or `informed_by:` files change |

## Field Reference

| Field | Type(s) | Triggers Drift? | Purpose |
|-------|---------|-----------------|---------|
| `context:` | all | No | Background reading for the agent |
| `tracks:` | documentation | **Yes** | Source code being documented |
| `informed_by:` | research | **Yes** | Materials to synthesize |
| `origin:` | research | **Yes** | Input data for analysis |
| `target_files:` | all | No | Output files to create/modify |
| `schedule:` | research | No | Recurring execution (e.g., `daily`, `weekly`) |

## Code Specs

The default type. Creates or modifies code, configuration, infrastructure.

```yaml
---
type: code
context:                          # Reference docs for background
  - docs/api-design.md
target_files:
  - src/auth/middleware.rs
  - src/auth/jwt.rs
---
# Add JWT authentication

## Acceptance Criteria
- [ ] JWT tokens validated
- [ ] 401 on invalid token
- [ ] Token refresh works
```

**Execution**: Agent reads context, implements code changes.
**Verification**: Acceptance criteria checked.
**Drift**: When criteria no longer pass.

## Task Specs

For manual work, creating prompts, configuration, or anything that doesn't produce code.

```yaml
---
type: task
target_files:
  - .chant/prompts/documentation.md
---
# Create documentation prompt

Create a prompt that guides agents to generate documentation from tracked source files.

## Acceptance Criteria
- [ ] Prompt file created
- [ ] Prompt explains process
- [ ] Prompt is actionable
```

**Execution**: Agent performs the task, creates artifacts.
**Verification**: Acceptance criteria checked.
**Drift**: When criteria no longer pass.

### Task vs Code

| Aspect | `code` | `task` |
|--------|--------|--------|
| Output | Source code, tests | Prompts, config, docs |
| Tests | Usually runs tests | Usually no tests |
| Build | May require build | No build needed |

## Driver Specs

Coordinate multiple related specs. A spec becomes a driver when it has member specs (files with `.N` suffix).

```yaml
---
type: driver
status: pending
---
# Implement authentication system

This driver coordinates the auth implementation.

## Members
- `.1` - Add JWT middleware
- `.2` - Add login endpoint
- `.3` - Add tests
```

**File structure:**
```
2026-01-22-001-x7m.md      ← Driver
2026-01-22-001-x7m.1.md    ← Member 1
2026-01-22-001-x7m.2.md    ← Member 2
2026-01-22-001-x7m.3.md    ← Member 3
```

**Execution**: Driver waits for all members to complete, then auto-completes.
**Verification**: All members must be completed.
**Drift**: When any member becomes incomplete.

### Driver Behavior

- Driver cannot complete until all members complete
- Members execute in order (`.1` before `.2`)
- Starting a member marks driver as `in_progress`
- Driver auto-completes when last member completes

## Group Specs

`group` is an alias for `driver`. Use whichever term feels more natural.

```yaml
---
type: group
---
# Feature: User profiles

## Members
- `.1` - Add profile model
- `.2` - Add profile API
- `.3` - Add profile UI
```

### The `context:` Field

For code specs, `context:` provides reference material:

```yaml
---
type: code
context:
  - docs/api-design.md#error-handling
  - docs/security-policy.md
---
# Add error responses
```

The agent sees the referenced doc content while implementing. This is background information, not work input.

## Documentation Specs

Creates or updates documentation based on source code.

```yaml
---
type: documentation
tracks:                           # Source code to document and monitor
  - src/auth/*.rs
target_files:
  - docs/authentication.md
---
# Document authentication module

## Scope
- All auth endpoints
- JWT flow
- Error codes

## Acceptance Criteria
- [ ] All public functions documented
- [ ] Usage examples included
- [ ] Architecture diagram current
```

**Execution**: Agent reads tracked files, writes documentation.
**Verification**: Tracked files haven't changed since completion.
**Drift**: When tracked source code changes after doc is complete.

### The `tracks:` Field

The `tracks:` field creates a living link between docs and code:

```
Code changes → triggers → Doc spec drift → re-verify → update docs
```

When `src/auth/*.rs` changes after doc completion:

```bash
$ chant verify 001
Spec 001 (documentation): DRIFT

Tracked files changed since completion:
  - src/auth/middleware.rs (modified 2026-01-25)
  - src/auth/token.rs (added 2026-01-25)

Recommendation: Re-run spec to update docs
```

### Documentation Use Cases

| Use Case | `tracks:` | `target_files:` |
|----------|-----------|-----------------|
| API reference | `src/api/**/*.rs` | `docs/api.md` |
| Architecture docs | `src/`, `Cargo.toml` | `docs/architecture.md` |
| Module docs | `src/auth/*.rs` | `docs/auth.md` |
| README | `src/lib.rs` | `README.md` |

## Research Specs

Creates analysis, synthesis, or findings. Supports two patterns: synthesis (reading materials) and analysis (processing data). Both can be combined.

### Pattern: Synthesis

Read and synthesize materials into findings:

```yaml
---
type: research
prompt: research-synthesis
informed_by:                      # Materials to read and synthesize
  - papers/smith2025.pdf
  - papers/jones2024.pdf
  - docs/prior-analysis.md
target_files:
  - findings/lit-review.md
---
# Synthesize ML efficiency papers

## Research Questions
- [ ] What are the main efficiency approaches?
- [ ] Which show >50% improvement?
- [ ] What are the trade-offs?

## Acceptance Criteria
- [ ] All papers summarized
- [ ] Comparison table created
- [ ] Research gaps identified
```

### Pattern: Analysis

Process data files into reports:

```yaml
---
type: research
prompt: research-analysis
origin:                           # Input data to analyze
  - data/survey-2026.csv
target_files:
  - analysis/survey-results.md
  - analysis/figures/correlation.png
---
# Analyze survey responses

## Methodology
- Descriptive statistics
- Correlation analysis
- Theme coding for open responses

## Acceptance Criteria
- [ ] Analysis script runs without error
- [ ] All columns analyzed
- [ ] Statistical significance noted
- [ ] Visualizations generated
```

### Combined Pattern

Research specs can use both `informed_by:` AND `origin:`:

```yaml
---
type: research
informed_by:                      # Prior work to build on
  - papers/methodology.pdf
  - docs/previous-analysis.md
origin:                           # Data to analyze
  - data/experiment-results.csv
target_files:
  - findings/analysis.md
---
# Analyze experiment using established methodology
```

### Recurring Research

Use `schedule:` for automated recurring analysis:

```yaml
---
type: research
prompt: research-analysis
schedule: weekly                  # daily | weekly | monthly | cron expression
origin:
  - logs/production-*.json
target_files:
  - reports/weekly-errors.md
---
# Weekly error analysis

## Methodology
- Aggregate errors by type
- Identify new error patterns
- Compare to previous week

## Acceptance Criteria
- [ ] All error types categorized
- [ ] Trends identified
- [ ] Actionable recommendations
```

**Execution**: Agent reads `informed_by:` and `origin:` files, performs analysis/synthesis, writes findings.
**Verification**: Input files haven't changed since completion.
**Drift**: When `origin:` OR `informed_by:` files change after completion.

### Research Drift

| Drift Type | Trigger | Detection |
|------------|---------|-----------|
| **Data drift** | `origin:` files changed | File modification detected |
| **Source drift** | `informed_by:` files changed | File modification detected |
| **Reproducibility drift** | Can't replicate results | `chant verify` fails |

## Prompt Selection by Type

Prompts are auto-selected based on type:

```yaml
# config.yaml
prompts:
  by_type:
    code: standard
    documentation: documentation
    research: research-synthesis
```

Override per-spec:

```yaml
---
type: research
prompt: research-analysis         # Use analysis prompt, not synthesis
---
```

## Summary

| Concept | Code | Task | Driver/Group | Documentation | Research |
|---------|------|------|--------------|---------------|----------|
| **Purpose** | Implement features | Manual work | Coordinate specs | Document code | Analyze/synthesize |
| **Work input** | Criteria | Criteria | Members | `tracks:` | `informed_by:` / `origin:` |
| **Drift trigger** | Criteria fail | Criteria fail | Members incomplete | `tracks:` changes | Input files change |
| **Schedule** | No | No | No | No | Yes |
| **Default prompt** | `standard` | `standard` | — | `documentation` | `research-*` |

---

## Implementation Status

The `documentation` and `research` spec types are implemented with:
- Frontmatter fields: `tracks:`, `informed_by:`, `origin:`, `schedule:`
- Lint validation warnings for missing fields
- Auto-selection of prompts based on spec type
- Prompts: `documentation`, `research-analysis`, `research-synthesis`

The following design questions remain TBD for future versions.

### TBD: Schedule Execution Model

How does `schedule:` trigger recurring execution?

| Option | Description | Trade-offs |
|--------|-------------|------------|
| A: Daemon | Daemon polls schedules, triggers work | Requires daemon running |
| B: External cron | User configures cron to call `chant work --scheduled` | User manages cron |
| C: Built-in scheduler | `chant schedule` manages system cron/launchd | Platform-specific |

**Decision:** TBD

### TBD: Drift Detection Storage

Where do we store baseline file state for drift comparison?

| Option | Description | Trade-offs |
|--------|-------------|------------|
| A: Frontmatter | Store hashes in spec (`tracks_hashes:`) | Clutters spec, but self-contained |
| B: Separate files | `.chant/drift/<spec-id>.json` | Clean specs, extra files |
| C: Git-based | Compare to files at `commit:` time | No extra storage, requires git history |

**Decision:** TBD

### TBD: URL Handling in `informed_by:`

Can `informed_by:` reference URLs and external identifiers?

```yaml
informed_by:
  - papers/local.pdf          # Local file
  - https://docs.rs/serde     # URL
  - arxiv:2401.12345          # External identifier
```

| Option | Description | Trade-offs |
|--------|-------------|------------|
| A: Local only | Only local file paths | Simple, defer URLs to future |
| B: URLs fetched | Agent fetches URLs at execution | Need caching, auth handling |
| C: Full external | Support `arxiv:`, `doi:`, etc. | Complex resolution logic |

**Decision:** TBD (recommend A for v0.2.0)

### Prompts

These prompts are available in `.chant/prompts/`:

| Prompt | Type | Purpose |
|--------|------|---------|
| `documentation` | documentation | Generate/update docs from tracked code |
| `research-synthesis` | research | Synthesize materials into findings |
| `research-analysis` | research | Analyze data, generate reports |

**Auto-selection:**
- `type: documentation` → `documentation` prompt
- `type: research` with `origin:` → `research-analysis` prompt
- `type: research` without `origin:` → `research-synthesis` prompt
