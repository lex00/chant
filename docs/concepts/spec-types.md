# Spec Types

Chant supports three spec types, each with different behaviors for execution, verification, and drift detection.

## Overview

| Type | What It Creates | Drift Trigger | Drifts When |
|------|-----------------|---------------|-------------|
| `code` | Code, config, infra | Criteria | Acceptance criteria fail |
| `documentation` | Docs about code | `tracks:` files | Tracked source changes |
| `research` | Analysis, findings | `origin:` + `informed_by:` | Input files change |

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

| Concept | Code | Documentation | Research |
|---------|------|---------------|----------|
| **Purpose** | Implement features | Document code | Analyze/synthesize |
| **Context field** | `context:` | `context:` | `context:` |
| **Work input** | Acceptance criteria | `tracks:` | `informed_by:` / `origin:` |
| **Drift trigger** | Criteria fail | `tracks:` changes | `informed_by:` or `origin:` changes |
| **Supports schedule** | No | No | Yes |
| **Default prompt** | `standard` | `documentation` | `research-synthesis` |

---

## Implementation Status

> **Status: Planned** — The `documentation` and `research` spec types are designed but not yet implemented. The following design questions are TBD.

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

### TBD: Required Prompts

These prompts need to be created:

| Prompt | Type | Purpose |
|--------|------|---------|
| `documentation` | documentation | Generate/update docs from tracked code |
| `research-synthesis` | research | Synthesize materials into findings |
| `research-analysis` | research | Analyze data, generate reports |

**Status:** Not yet created
