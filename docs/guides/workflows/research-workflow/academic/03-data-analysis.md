# Week 2: Data Analysis

With the literature synthesized, Sarah moves to data analysis. This phase processes 30 years of temperature data to generate statistical findings.

## The Analysis Pattern

Data analysis uses the **analysis pattern** — processing source data to extract findings:

```
┌─────────────────────────────────────────────────────────────┐
│                     Origin Data Files                        │
├─────────────┬─────────────┬─────────────┬─────────────────────┤
│ station-01  │ station-02  │ station-03  │      ...            │
│   .csv      │   .csv      │   .csv      │   (12 stations)     │
└──────┬──────┴──────┬──────┴──────┬──────┴──────────┬──────────┘
       │             │             │                 │
       └─────────────┴─────────────┴─────────────────┘
                              │
                              ▼
                 ┌─────────────────────────┐
                 │     Research Spec       │
                 │    origin:              │
                 │     - data/temp/*.csv   │
                 └────────────┬────────────┘
                              │
                              ▼
                 ┌─────────────────────────┐
                 │   Statistical Analysis  │
                 │   (Findings & Figures)  │
                 └─────────────────────────┘
```

The `origin:` field tells the agent what data to analyze. Changes to origin files trigger drift detection.

## Data Overview

Sarah's temperature data spans 12 Arctic monitoring stations:

```
data/temperature/
├── barrow-alaska.csv          # 1994-2024, 10,957 daily readings
├── resolute-canada.csv        # 1994-2024, 10,957 daily readings
├── longyearbyen-svalbard.csv  # 1994-2024, 10,957 daily readings
├── tiksi-russia.csv           # 1994-2024, 10,957 daily readings
├── alert-canada.csv           # 1994-2024, 10,957 daily readings
├── thule-greenland.csv        # 1994-2024, 10,957 daily readings
├── nord-greenland.csv         # 1994-2024, 10,957 daily readings
├── dikson-russia.csv          # 1994-2024, 10,957 daily readings
├── eureka-canada.csv          # 1994-2024, 10,957 daily readings
├── jan-mayen-norway.csv       # 1994-2024, 10,957 daily readings
├── bear-island-norway.csv     # 1994-2024, 10,957 daily readings
└── ny-alesund-svalbard.csv    # 1994-2024, 10,957 daily readings
```

Each CSV contains:
```csv
date,temp_c,temp_anomaly,quality_flag
1994-01-01,-28.3,-2.1,A
1994-01-02,-29.1,-2.9,A
...
```

## Creating the Analysis Spec

Sarah creates a research spec for statistical analysis:

```bash
chant add "Analyze Arctic temperature acceleration" --type research
```

She edits the spec with detailed methodology:

**File: `.chant/specs/2026-01-16-001-ana.md`**

```yaml
---
type: research
status: pending
prompt: research-analysis
depends_on:
  - 2026-01-15-001-lit
origin:
  - data/temperature/*.csv
informed_by:
  - analysis/literature-review.md
  - analysis/research-gaps.md
target_files:
  - analysis/temperature-findings.md
  - analysis/figures/trend-acceleration.png
  - analysis/figures/seasonal-patterns.png
  - analysis/figures/station-comparison.png
---
```

```markdown
# Analyze Arctic temperature acceleration

## Problem

30 years of temperature data from 12 Arctic stations needs statistical analysis
to answer the thesis question: Have Arctic temperature increases accelerated
since 1995?

This analysis builds on the literature review findings that identified gaps in:
- Post-2015 acceleration quantification
- Station-level variability analysis
- Seasonal acceleration patterns

## Research Questions

- [ ] What is the overall warming trend (1994-2024)?
- [ ] Is there evidence of acceleration (nonlinear trend)?
- [ ] Do stations show consistent or divergent patterns?
- [ ] How do seasonal patterns differ?
- [ ] When did acceleration begin (if present)?

## Methodology

### Data Preparation
1. Load all CSV files from `data/temperature/`
2. Handle quality flags (exclude 'X' flagged observations)
3. Convert to annual and seasonal means
4. Calculate anomalies from 1994-2000 baseline

### Trend Analysis
1. Linear regression for overall trend
2. Quadratic regression to test for acceleration
3. Piecewise regression to find breakpoints
4. Mann-Kendall test for monotonic trends

### Station Comparison
1. Calculate trends for each station
2. Test for homogeneity across stations
3. Identify outlier stations
4. Map spatial patterns

### Seasonal Decomposition
1. Separate annual means into DJF, MAM, JJA, SON
2. Calculate trends by season
3. Test for acceleration by season
4. Identify which seasons drive overall acceleration

## Acceptance Criteria

- [ ] All 12 stations analyzed with consistent methodology
- [ ] Quality-flagged data excluded
- [ ] Linear and quadratic trend estimates with confidence intervals
- [ ] Breakpoint analysis with statistical tests
- [ ] Station-level trends with heterogeneity test
- [ ] Seasonal trends with acceleration testing
- [ ] Three figures generated (trend, seasonal, station comparison)
- [ ] temperature-findings.md with all results and statistics
```

## Running the Analysis

Sarah executes the spec:

```bash
chant work 001-ana
```

The agent:
1. Reads all CSV files via `origin:`
2. Reads the literature review via `informed_by:` for context
3. Runs the statistical methodology
4. Generates findings and figures

## Agent Output

After execution, Sarah finds new analysis files:

**File: `analysis/temperature-findings.md`** (generated)

```markdown
# Arctic Temperature Acceleration Analysis

## Executive Summary

Analysis of 12 Arctic monitoring stations (1994-2024) confirms acceleration:
- **Overall warming:** +0.67°C per decade (p < 0.001)
- **Acceleration detected:** Quadratic term significant (p = 0.003)
- **Breakpoint:** Acceleration begins ~2005 (95% CI: 2003-2008)
- **Seasonal pattern:** Winter (DJF) acceleration 2x stronger than summer

## 1. Data Summary

| Metric | Value |
|--------|-------|
| Stations analyzed | 12 |
| Total observations | 131,484 |
| Quality-excluded | 1,247 (0.95%) |
| Period | 1994-01-01 to 2024-12-31 |

## 2. Overall Trend

### Linear Model

```
Temperature = -0.89 + 0.067 × Year
```

| Parameter | Estimate | SE | 95% CI | p-value |
|-----------|----------|-----|--------|---------|
| Intercept | -0.89°C | 0.12 | [-1.12, -0.66] | < 0.001 |
| Trend | +0.067°C/yr | 0.008 | [0.051, 0.083] | < 0.001 |

Interpretation: +0.67°C warming per decade.

### Quadratic Model (Acceleration Test)

```
Temperature = -0.72 + 0.042 × Year + 0.0018 × Year²
```

| Parameter | Estimate | SE | p-value |
|-----------|----------|-----|---------|
| Quadratic term | +0.0018 | 0.0006 | 0.003 |

Interpretation: Statistically significant acceleration. Rate of warming is increasing.

### Breakpoint Analysis

Piecewise regression identified a breakpoint at **2005** (95% CI: 2003-2008):

| Period | Trend (°C/decade) | SE |
|--------|-------------------|-----|
| 1994-2005 | +0.42 | 0.11 |
| 2005-2024 | +0.89 | 0.09 |

Warming rate doubled after 2005.

## 3. Station Comparison

| Station | Trend (°C/decade) | Rank |
|---------|-------------------|------|
| Tiksi, Russia | +0.92 | 1 |
| Longyearbyen | +0.84 | 2 |
| Barrow, Alaska | +0.78 | 3 |
| Resolute, Canada | +0.72 | 4 |
| Ny-Alesund | +0.68 | 5 |
| Thule, Greenland | +0.65 | 6 |
| Nord, Greenland | +0.61 | 7 |
| Dikson, Russia | +0.59 | 8 |
| Alert, Canada | +0.58 | 9 |
| Eureka, Canada | +0.55 | 10 |
| Jan Mayen, Norway | +0.51 | 11 |
| Bear Island, Norway | +0.44 | 12 |

Heterogeneity test: Q = 45.2, p < 0.001 (significant station variation)

## 4. Seasonal Patterns

| Season | Trend (°C/decade) | Acceleration (°C/decade²) | p-value |
|--------|-------------------|---------------------------|---------|
| Winter (DJF) | +1.12 | +0.048 | 0.001 |
| Spring (MAM) | +0.71 | +0.022 | 0.018 |
| Summer (JJA) | +0.34 | +0.008 | 0.241 |
| Autumn (SON) | +0.52 | +0.015 | 0.067 |

Winter acceleration is 2x the overall average. Summer shows no significant acceleration.

## 5. Figures

- `figures/trend-acceleration.png` — Time series with linear and quadratic fits
- `figures/seasonal-patterns.png` — Seasonal decomposition with trend lines
- `figures/station-comparison.png` — Map of station-level trends

## 6. Conclusions

1. Arctic warming is accelerating (quadratic trend significant)
2. Acceleration began around 2005 (±3 years)
3. Post-2005 warming rate (0.89°C/decade) is 2x pre-2005 rate
4. Winter drives acceleration; summer shows no acceleration
5. Stations vary significantly, with Siberian stations warming fastest

## 7. Statistical Notes

- All tests use α = 0.05
- Confidence intervals are 95%
- Autocorrelation addressed via Newey-West standard errors
- Missing data handled via pairwise deletion
```

## Verifying the Analysis

Sarah checks the completed spec:

```bash
chant show 001-ana
```

```
Spec: 2026-01-16-001-ana
Type: research
Status: completed

Depends on: 2026-01-15-001-lit (completed)
Origin: data/temperature/*.csv (12 files)
Informed by: analysis/literature-review.md, analysis/research-gaps.md

Research Questions:
  [x] What is the overall warming trend (1994-2024)?
  [x] Is there evidence of acceleration (nonlinear trend)?
  [x] Do stations show consistent or divergent patterns?
  [x] How do seasonal patterns differ?
  [x] When did acceleration begin (if present)?

Acceptance Criteria:
  [x] All 12 stations analyzed with consistent methodology
  [x] Quality-flagged data excluded
  [x] Linear and quadratic trend estimates with confidence intervals
  [x] Breakpoint analysis with statistical tests
  [x] Station-level trends with heterogeneity test
  [x] Seasonal trends with acceleration testing
  [x] Three figures generated (trend, seasonal, station comparison)
  [x] temperature-findings.md with all results and statistics
```

## Origin Tracking

The spec records exactly which data files were analyzed:

```yaml
origin:
  - data/temperature/barrow-alaska.csv (sha256: a1b2c3...)
  - data/temperature/resolute-canada.csv (sha256: d4e5f6...)
  # ... 10 more files
```

If any of these files change, drift detection will flag the analysis as stale.

## Connecting Literature and Data

The `informed_by:` field ensured the analysis addressed the gaps identified in the literature review:

| Gap Identified | How Analysis Addressed |
|----------------|------------------------|
| Post-2015 acceleration | Extended analysis to 2024 |
| Station-level variability | Analyzed 12 stations individually |
| Seasonal patterns | Decomposed by DJF, MAM, JJA, SON |

The `depends_on:` field ensured the literature review completed before analysis began.

## What's Next

With individual analyses complete, Sarah coordinates a multi-step pipeline:

**[Pipeline Coordination](04-pipeline.md)** — Using driver specs to coordinate data cleaning, analysis, and visualization
