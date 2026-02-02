---
name: research-analysis
purpose: Analyze data/metrics to produce findings
---

# Research Analysis

You are analyzing research data for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Analysis Process

### 1. Understand the Data

**Tracked files** (data to analyze):
- These are the data files you must analyze
- Read all data sources completely before analyzing
- Understand the structure, format, and domain
- Identify what metrics or patterns matter

**Target files** (output):
- Analysis report(s) you will create
- Should contain findings, patterns, and insights

### 2. Read and Examine Data

1. **Read all data sources** - Understand the dataset completely
   - Understand structure and schema
   - Note data types, ranges, and distributions
   - Identify any data quality issues
   - Understand the domain context

2. **Identify analysis questions**
   - What patterns should you look for?
   - What metrics are relevant?
   - What comparisons are meaningful?
   - What trends or anomalies matter?

3. **Perform analysis**
   - Calculate relevant statistics
   - Identify patterns and correlations
   - Note outliers and anomalies
   - Compare across dimensions
   - Look for trends over time (if applicable)

### 3. Apply Analysis Principles

- **Rigorous** — Use appropriate methods for the data type
- **Evidence-based** — Ground findings in actual data
- **Balanced** — Report both expected and unexpected results
- **Transparent** — Show your methodology and assumptions
- **Actionable** — Connect findings to implications

### 4. Output Format

Your analysis should include:

```markdown
# Analysis: [Topic]

## Overview

Brief summary of the analysis scope, data sources, and methodology.

## Dataset Description

- **Sources**: List of data files analyzed
- **Size**: Number of records, time period, scope
- **Structure**: Key fields and data types
- **Quality**: Any data issues or limitations noted

## Methodology

How you performed the analysis:
- Analysis techniques used
- Metrics calculated
- Assumptions made
- Limitations of the approach

## Findings

### Finding 1: [Name]

Description of what you found.

**Evidence:**
- Metric/statistic: [value]
- Supporting data point: [detail]
- Pattern observed: [description]

**Interpretation:** What this finding means.

### Finding 2: [Name]

[Same structure]

## Key Metrics

Summary table or list of important metrics:

| Metric | Value | Interpretation |
|--------|-------|----------------|
| Metric 1 | X | What it means |
| Metric 2 | Y | What it means |

## Patterns and Trends

Patterns observed across the data:
- Pattern 1: [description with evidence]
- Pattern 2: [description with evidence]

## Anomalies and Outliers

Unusual observations that warrant attention:
- Anomaly 1: [description and potential significance]
- Anomaly 2: [description and potential significance]

## Implications

What the findings mean for the project or research question:
- Implication 1: [description]
- Implication 2: [description]

## Limitations

Constraints on the analysis or data:
- Limitation 1: [description]
- Limitation 2: [description]

## Recommendations

Suggested next steps based on findings:
- Recommendation 1: [description]
- Recommendation 2: [description]
```

### 5. Verification

Before completing:

1. **Data coverage check**: Did you analyze all relevant data sources?
2. **Methodology check**: Are your analysis methods appropriate?
3. **Evidence check**: Are findings grounded in actual data?
4. **Interpretation check**: Are conclusions supported by evidence?
5. **Acceptance criteria check**: Does output meet all requirements?

## Constraints

- Read all tracked data sources before analyzing
- Use appropriate methods for the data type
- Ground every finding in specific evidence
- Report methodology and assumptions
- Note limitations and data quality issues
- Focus on actionable insights

## Instructions

1. **Read** all tracked data files completely
2. **Understand** the data structure and domain
3. **Perform** analysis using appropriate methods
4. **Document** findings with supporting evidence
5. **Interpret** what the findings mean
6. **Check off** each acceptance criterion in `{{spec.path}}` by changing `- [ ]` to `- [x]`
7. **Commit** with message: `chant({{spec.id}}): <description>`
8. **Verify git status is clean** - ensure no uncommitted changes remain
