# Spec Types

Chant supports three spec types, each with different behaviors for execution, verification, and drift detection.

## Overview

| Type | What It Creates | Drift Trigger | Drifts When |
|------|-----------------|---------------|-------------|
| `code` | Code, config, infra | Criteria | Acceptance criteria fail |
| `documentation` | Docs about code | `tracks:` files | Tracked source changes |
| `research` | Analysis, findings | `data:` files | Input data changes |

## Field Names by Type

Each type uses specific field names to avoid ambiguity:

| Field | Code | Documentation | Research |
|-------|------|---------------|----------|
| **Context** (what to read for background) | `context:` | `context:` | `context:` |
| **Work input** (what to process) | — | `tracks:` | `sources:` |
| **Drift trigger** (what triggers re-verification) | Criteria | `tracks:` | `data:` |
| **Output** | `target_files:` | `target_files:` | `target_files:` |

## Code Specs

The default type. Creates or modifies code, configuration, infrastructure.

```yaml
---
type: code
context:                          # Reference docs for background
  - docs/api-design.md
target_files:
  - src/auth/middleware.go
  - src/auth/jwt.go
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
  - src/auth/*.go
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

The `tracks:` field creates a relationship between docs and code:

```
Code changes → triggers → Doc spec drift → re-verify → update docs
```

When `src/auth/*.go` changes after doc completion:

```bash
$ chant verify 001
Spec 001 (documentation): DRIFT

Tracked files changed since completion:
  - src/auth/middleware.go (modified 2026-01-25)
  - src/auth/token.go (added 2026-01-25)

Recommendation: Re-run spec to update docs
```

## Research Specs

Creates analysis, synthesis, or findings from source materials.

### Literature Synthesis

```yaml
---
type: research
sources:                          # Materials to synthesize
  - papers/smith2025.pdf
  - papers/jones2024.pdf
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

### Data Analysis

```yaml
---
type: research
data:                             # Input data files (trigger drift)
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

**Execution**: Agent reads sources/data, performs analysis, writes findings.
**Verification**: Data files haven't changed.
**Drift**: When data changes after analysis is complete.

### Research Fields

| Field | Purpose |
|-------|---------|
| `sources:` | Materials to synthesize (papers, docs, prior findings) |
| `data:` | Input data files that trigger drift when changed |
| `context:` | Background reference (same as code specs) |

A research spec can have both `sources:` AND `data:`:

```yaml
---
type: research
sources:                          # Prior work to build on
  - papers/methodology.pdf
data:                             # Data to analyze
  - data/experiment-results.csv
target_files:
  - findings/analysis.md
---
```

### Research Drift

| Drift Type | Trigger | Detection |
|------------|---------|-----------|
| **Data drift** | Input data changed | `data:` file changes |
| **Literature drift** | New papers published | External (future) |
| **Reproducibility drift** | Can't replicate results | `chant verify` fails |

## Prompt Selection by Type

Prompts are auto-selected based on type:

```yaml
# config.md
prompts:
  by_type:
    code: standard
    documentation: documentation
    research: research-synthesis   # or research-analysis
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
| **Work input** | Acceptance criteria | `tracks:` | `sources:` / `data:` |
| **Drift trigger** | Criteria fail | `tracks:` changes | `data:` changes |
| **Default prompt** | `standard` | `documentation` | `research-*` |
