# Research Workflows Guide

Using chant for research: investigation, synthesis, data analysis, and reproducible findings.

## Why Chant for Research?

Research has a reproducibility crisis:
- Methods under-specified
- Data not preserved
- Analysis steps lost
- Conclusions go stale

Chant addresses this:
- **Spec IS the method** — Not a paper written after the fact
- **Execution is recorded** — Agent logs every step
- **Verification is built-in** — Re-run and compare
- **Drift detection** — Know when inputs change

## Quick Start

```bash
# Initialize chant
chant init

# Create a research spec
chant add "Analyze Q1 survey data"
```

Edit the spec:

```yaml
---
type: research
prompt: research-analysis
origin:
  - data/q1-survey.csv
target_files:
  - analysis/q1-results.md
  - analysis/figures/q1-correlation.png
---
# Analyze Q1 survey data

## Methodology
- Descriptive statistics for all numeric columns
- Correlation analysis between satisfaction and tenure
- Theme coding for open-ended responses

## Research Questions
- [ ] What is the average satisfaction score?
- [ ] Does tenure correlate with satisfaction?
- [ ] What themes emerge from comments?

## Acceptance Criteria
- [ ] Analysis script runs without error
- [ ] All questions answered with data
- [ ] Visualizations generated
- [ ] Statistical significance noted
```

Run:

```bash
chant work 001
```

## Research Patterns

### Pattern: Synthesis

Synthesize multiple sources into findings:

```yaml
---
type: research
prompt: research-synthesis
informed_by:
  - papers/smith2025.pdf
  - papers/jones2024.pdf
  - arxiv:2401.12345
target_files:
  - findings/lit-review.md
---
# Synthesize transformer efficiency research

## Research Questions
- [ ] What are the main efficiency approaches?
- [ ] Which show >50% improvement on benchmarks?
- [ ] What are the trade-offs?

## Acceptance Criteria
- [ ] 10+ papers reviewed
- [ ] Comparison table created
- [ ] Gaps in research identified
```

The `informed_by:` field lists materials to synthesize. Changes to these files trigger drift detection.

### Pattern: Data Analysis

Analyze data files to generate findings:

```yaml
---
type: research
prompt: research-analysis
origin:
  - data/experiment-results.csv
target_files:
  - analysis/experiment-findings.md
---
# Analyze experiment results

## Methodology
- Two-sample t-test for treatment vs control
- Effect size calculation (Cohen's d)
- Confidence intervals

## Acceptance Criteria
- [ ] p-value calculated
- [ ] Effect size reported
- [ ] 95% CI for mean difference
- [ ] Visualization of distributions
```

The `origin:` field declares input data. When data changes after analysis, drift is detected.

### Pattern: Codebase Investigation

Research specs work for code analysis too:

```yaml
---
type: research
prompt: research-analysis
informed_by:
  - src/**/*.rs
target_files:
  - analysis/tech-debt.md
---
# Investigate technical debt

## Research Questions
- [ ] Where are the TODO/FIXME comments?
- [ ] Which modules have highest complexity?
- [ ] What patterns are inconsistently applied?

## Acceptance Criteria
- [ ] All modules scanned
- [ ] Issues prioritized by impact
- [ ] Recommendations provided
```

### Pattern: Library Comparison

Before choosing a dependency:

```yaml
---
type: research
prompt: research-synthesis
informed_by:
  - https://docs.rs/serde
  - https://docs.rs/rkyv
target_files:
  - findings/serialization-comparison.md
---
# Compare serialization libraries

## Research Questions
- [ ] Performance characteristics?
- [ ] API ergonomics?
- [ ] Ecosystem support?

## Acceptance Criteria
- [ ] Both libraries evaluated
- [ ] Benchmarks compared
- [ ] Recommendation with rationale
```

## Recurring Research

For recurring analysis, use the `schedule:` field to track cadence:

```yaml
---
type: research
prompt: research-analysis
schedule: weekly
origin:
  - logs/production-*.json
target_files:
  - reports/weekly-errors.md
---
# Weekly error analysis

## Methodology
- Aggregate errors by type
- Identify new patterns
- Compare to previous week

## Acceptance Criteria
- [ ] All error types categorized
- [ ] Trends identified
- [ ] Actionable recommendations
```

The `schedule:` field documents intended recurrence (e.g., `daily`, `weekly`, `monthly`) but does not trigger automated execution.

## Drift Detection

### Data Drift

When `origin:` data changes:

```bash
$ chant verify 001
Spec 001 (research): DRIFT

Origin files changed since completion:
  - data/q1-survey.csv (modified 2026-01-25)

New data rows: 47 added since analysis

Recommendation: Re-run spec to update analysis
```

### Source Drift

When `informed_by:` materials change:

```bash
$ chant verify 002
Spec 002 (research): DRIFT

Informed-by files changed since completion:
  - papers/smith2025.pdf (modified 2026-01-25)

Recommendation: Re-run spec to incorporate updates
```

### Re-running Analysis

When drift is detected, re-run the spec to update findings:

```bash
$ chant reset 001
$ chant work 001
```

Compare the new results to the original to see what changed.

## Verification Strategies

### Objective Verification

Use acceptance criteria that can be checked programmatically:

```yaml
## Acceptance Criteria
- [ ] Analysis script runs without error
- [ ] p-value < 0.05 for primary hypothesis
- [ ] All required visualizations exist
- [ ] Output matches expected schema
```

The agent can write validation scripts:

```python
# Agent writes this during execution
import json

with open('analysis/results.json') as f:
    results = json.load(f)

assert results['p_value'] < 0.05, f"p-value {results['p_value']} not significant"
assert 'effect_size' in results, "Effect size not calculated"
print("Validation passed")
```

### Subjective Verification

For qualitative research, criteria are inherently subjective:

```yaml
## Acceptance Criteria
- [ ] All interviews coded
- [ ] Themes supported by 3+ quotes
- [ ] Negative cases discussed
```

These require human review, but drift detection still works.

## Provenance

Every finding traces back to:

```yaml
---
status: completed
completed_at: 2026-01-22T15:00:00Z
commit: abc123
origin:
  - data/survey.csv
informed_by:
  - papers/methodology.pdf
---
```

The spec itself is the audit trail.

## Research Pipelines

### Experiment Pipeline

```yaml
# 001.md - Data collection
---
type: research
target_files: [data/experiment.csv]
---

# 002.md - Analysis (depends on collection)
---
type: research
depends_on: [001]
origin: [data/experiment.csv]
target_files: [analysis/results.md]
---

# 003.md - Write-up (depends on analysis)
---
type: research
depends_on: [002]
informed_by: [analysis/results.md]
target_files: [paper/methodology.md]
---
```

### Reproducibility Check

```yaml
---
type: research
origin:
  - data/original-study.csv
informed_by:
  - papers/original-paper.pdf
---
# Replicate Smith 2025 findings

Attempt to reproduce results from original paper.

## Acceptance Criteria
- [ ] Same methodology applied
- [ ] Results compared to original
- [ ] Discrepancies documented
```

## Use Cases Summary

| Use Case | `informed_by:` | `origin:` | `schedule:` |
|----------|----------------|-----------|-------------|
| Literature review | papers, docs | — | — |
| Log analysis | — | log files | `daily` |
| Codebase health | `src/**/*.rs` | — | `weekly` |
| Performance report | prior reports | metrics CSV | `weekly` |
| Bug investigation | related code | error logs | — |
| Library comparison | library docs | — | — |
| Survey analysis | methodology docs | survey data | — |

## Limitations

Chant helps with:
- Computational research
- Data analysis
- Literature synthesis
- Code investigation

Chant doesn't help with:
- Wet lab work (but can analyze results)
- Creative insight (but can synthesize)
- Peer review (but can prepare submissions)
- Data collection (but can analyze collected data)

## See Also

- [Research Workflow Example](../../examples/research-workflow/) — Example demonstrating synthesis and analysis
- [spec-types.md](../concepts/spec-types.md) — Overview of spec types including research
- [prompts.md](../concepts/prompts.md) — research-synthesis and research-analysis prompts
