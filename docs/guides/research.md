# Research Workflows Guide

Using chant for research: literature synthesis, data analysis, and reproducible findings.

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
- **Drift detection** — Know when results change

## Quick Start

```bash
# Initialize chant
chant init

# Create a research spec
chant add "Analyze Q1 survey data" --type research
```

Edit the spec:

```yaml
---
type: research
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

## Research Spec Types

### Literature Synthesis

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

The `informed_by` field lists sources to synthesize. External sources (arxiv, doi) are resolved by the prompt.

### Data Analysis

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

The `origin` field declares data files. When they change after analysis, drift is detected.

## Drift Detection

### Data Drift

When origin data changes:

```bash
$ chant verify 001
Spec 001 (research): DRIFT

Origin files changed since completion:
  - data/q1-survey.csv (modified 2026-01-25)

New data rows: 47 added since analysis

Recommendation: Re-run spec to update analysis
```

### Replaying Analysis

```bash
$ chant replay 001
Replaying spec 001: Analyze Q1 survey data

Original completion: 2026-01-15
Data changed: 2026-01-25 (47 new rows)

Agent re-analyzing...

Comparison:
  - Average satisfaction: 4.2 → 4.1 (slight decrease)
  - Tenure correlation: 0.45 → 0.52 (stronger)
  - New theme identified: "remote work"

Replay complete.
```

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
  - data/survey.csv@sha256:def456  # Could track hash
informed_by:
  - papers/methodology.pdf
---
```

The spec itself is the audit trail.

## Patterns

### Living Literature Review

```yaml
---
type: research
informed_by:
  - semantic-scholar:topic/transformer-efficiency
---
# Transformer efficiency literature

When new papers match the topic, re-run to update synthesis.
```

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

## Limitations

Chant helps with:
- Computational research
- Data analysis
- Literature synthesis
- Documentation

Chant doesn't help with:
- Wet lab work (but can analyze results)
- Creative insight (but can synthesize)
- Peer review (but can prepare submissions)
- Data collection (but can analyze collected data)

## See Also

- [spec-types.md](../concepts/spec-types.md) — Overview of research spec type
- [prompts.md](../concepts/prompts.md) — research-synthesis and research-analysis prompts
- [autonomy.md](../concepts/autonomy.md) — Drift detection and replay
