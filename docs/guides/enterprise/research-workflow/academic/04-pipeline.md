# Week 3: Pipeline Coordination

Sarah's analysis requires a multi-step pipeline. Each step must complete before the next begins. This phase uses driver specs to coordinate the workflow.

## The Pipeline Pattern

Complex research often requires coordinated phases:

```
┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ Data Clean  │──>│ Statistical │──>│ Sensitivity │──>│ Figure      │
│  (.1)       │   │ Analysis    │   │  Analysis   │   │ Generation  │
│             │   │  (.2)       │   │   (.3)      │   │   (.4)      │
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
       │                 │                 │                 │
       └─────────────────┴─────────────────┴─────────────────┘
                                   │
                                   ▼
                      ┌─────────────────────────┐
                      │      Driver Spec        │
                      │   members: [.1-.4]      │
                      │   Coordinates all steps │
                      └─────────────────────────┘
```

A **driver spec** coordinates member specs, ensuring proper sequencing and parallel execution where possible.

## Why a Pipeline?

Sarah's analysis has dependencies:

| Step | Input | Output | Depends On |
|------|-------|--------|------------|
| Data Cleaning | Raw CSVs | Cleaned CSVs | None |
| Statistical Analysis | Cleaned CSVs | Results | Cleaning |
| Sensitivity Analysis | Cleaned CSVs, Results | Robustness tests | Cleaning, Analysis |
| Figure Generation | Results | Publication figures | Analysis |

Some steps can run in parallel (Sensitivity + Figures), but all depend on earlier steps.

## Creating the Driver Spec

Sarah creates a driver spec to coordinate the pipeline:

```bash
chant add "Arctic analysis pipeline" --type driver
```

**File: `.chant/specs/2026-01-20-001-drv.md`**

```yaml
---
type: driver
status: pending
prompt: driver
members:
  - 2026-01-20-001-drv.1  # Data cleaning
  - 2026-01-20-001-drv.2  # Statistical analysis
  - 2026-01-20-001-drv.3  # Sensitivity analysis
  - 2026-01-20-001-drv.4  # Figure generation
target_files:
  - analysis/pipeline-report.md
---
```

```markdown
# Arctic analysis pipeline

## Problem

The full analysis requires four coordinated steps. Manual execution risks:
- Running steps out of order
- Missing dependencies
- Inconsistent methodology across steps

## Pipeline Steps

### Step 1: Data Cleaning (.1)
- Validate all CSV files
- Apply quality flag filters
- Standardize date formats
- Output: cleaned data files

### Step 2: Statistical Analysis (.2)
- Run trend analysis on cleaned data
- Calculate confidence intervals
- Output: statistical results

### Step 3: Sensitivity Analysis (.3)
- Test alternative methodologies
- Vary quality thresholds
- Output: robustness report

### Step 4: Figure Generation (.4)
- Generate publication-ready figures
- Apply consistent styling
- Output: PNG files for dissertation

## Execution Order

```
.1 (cleaning)
    │
    ▼
.2 (analysis)
    │
    ├─────────┐
    ▼         ▼
.3 (sens)   .4 (figs)
```

.3 and .4 can run in parallel after .2 completes.

## Acceptance Criteria

- [ ] All four member specs created
- [ ] Dependencies correctly specified
- [ ] All members complete successfully
- [ ] Pipeline report summarizing all steps
```

## Creating Member Specs

Sarah creates each member spec. The driver spec doesn't execute them directly — it coordinates their creation and execution.

### Member .1: Data Cleaning

**File: `.chant/specs/2026-01-20-001-drv.1.md`**

```yaml
---
type: code
status: pending
parent: 2026-01-20-001-drv
origin:
  - data/temperature/*.csv
target_files:
  - data/cleaned/*.csv
  - analysis/cleaning-report.md
---
```

```markdown
# Data cleaning

## Problem

Raw temperature CSV files need standardization before analysis.

## Tasks

- [ ] Validate date formats (ISO 8601)
- [ ] Apply quality flag filtering (exclude 'X' flags)
- [ ] Handle missing values (document, don't interpolate)
- [ ] Standardize column names
- [ ] Output cleaned files to data/cleaned/

## Acceptance Criteria

- [ ] All 12 stations processed
- [ ] Quality flags applied correctly
- [ ] Cleaning report documents excluded observations
- [ ] Cleaned files ready for analysis
```

### Member .2: Statistical Analysis

**File: `.chant/specs/2026-01-20-001-drv.2.md`**

```yaml
---
type: research
status: pending
parent: 2026-01-20-001-drv
depends_on:
  - 2026-01-20-001-drv.1
origin:
  - data/cleaned/*.csv
target_files:
  - analysis/statistical-results.json
  - analysis/statistical-report.md
---
```

```markdown
# Statistical analysis

## Problem

Run core statistical analysis on cleaned data.

## Methodology

- Linear trend estimation
- Quadratic (acceleration) test
- Breakpoint detection
- Seasonal decomposition

## Acceptance Criteria

- [ ] All statistical tests documented
- [ ] Results in JSON for downstream use
- [ ] Report with interpretation
```

### Member .3: Sensitivity Analysis

**File: `.chant/specs/2026-01-20-001-drv.3.md`**

```yaml
---
type: research
status: pending
parent: 2026-01-20-001-drv
depends_on:
  - 2026-01-20-001-drv.1
  - 2026-01-20-001-drv.2
origin:
  - data/cleaned/*.csv
informed_by:
  - analysis/statistical-results.json
target_files:
  - analysis/sensitivity-report.md
---
```

```markdown
# Sensitivity analysis

## Problem

Test robustness of findings to methodological choices.

## Tests

1. **Quality threshold**: Vary from strict (A only) to permissive (A, B, C)
2. **Baseline period**: 1994-2000 vs 1994-2005
3. **Trend method**: OLS vs Theil-Sen (robust)
4. **Missing data**: Exclude vs interpolate

## Acceptance Criteria

- [ ] All four sensitivity tests run
- [ ] Results compared to primary analysis
- [ ] Robustness assessment documented
```

### Member .4: Figure Generation

**File: `.chant/specs/2026-01-20-001-drv.4.md`**

```yaml
---
type: code
status: pending
parent: 2026-01-20-001-drv
depends_on:
  - 2026-01-20-001-drv.2
informed_by:
  - analysis/statistical-results.json
target_files:
  - analysis/figures/main-trend.png
  - analysis/figures/seasonal.png
  - analysis/figures/station-map.png
  - analysis/figures/sensitivity.png
---
```

```markdown
# Figure generation

## Problem

Generate publication-ready figures for dissertation.

## Figures

1. **main-trend.png**: Time series with trend and uncertainty
2. **seasonal.png**: Four-panel seasonal decomposition
3. **station-map.png**: Geographic map with station trends
4. **sensitivity.png**: Sensitivity analysis comparison

## Style Requirements

- Font: Times New Roman, 12pt
- Colors: Publication-ready palette
- Resolution: 300 DPI
- Format: PNG with transparency

## Acceptance Criteria

- [ ] All four figures generated
- [ ] Consistent styling applied
- [ ] Ready for dissertation inclusion
```

## Running the Pipeline

Sarah executes the driver:

```bash
chant work 001-drv
```

Chant automatically:
1. Creates worktrees for each member spec
2. Executes .1 (cleaning) first
3. Waits for .1 to complete
4. Executes .2 (analysis)
5. Waits for .2 to complete
6. Executes .3 and .4 in parallel

```
$ chant work 001-drv
Working: 2026-01-20-001-drv (Arctic analysis pipeline)

Phase 1: Sequential
  [✓] 2026-01-20-001-drv.1 (Data cleaning) - 2m 15s

Phase 2: Sequential
  [✓] 2026-01-20-001-drv.2 (Statistical analysis) - 4m 32s

Phase 3: Parallel
  [✓] 2026-01-20-001-drv.3 (Sensitivity analysis) - 3m 45s
  [✓] 2026-01-20-001-drv.4 (Figure generation) - 1m 58s

Driver complete. All 4 members succeeded.
```

## Pipeline Report

The driver spec generates a summary:

**File: `analysis/pipeline-report.md`** (generated)

```markdown
# Arctic Analysis Pipeline Report

## Execution Summary

| Step | Spec ID | Duration | Status |
|------|---------|----------|--------|
| Data Cleaning | drv.1 | 2m 15s | Success |
| Statistical Analysis | drv.2 | 4m 32s | Success |
| Sensitivity Analysis | drv.3 | 3m 45s | Success |
| Figure Generation | drv.4 | 1m 58s | Success |

Total pipeline duration: 12m 30s

## Data Flow

```
Raw CSVs (12 files)
    │
    ▼ [drv.1]
Cleaned CSVs (12 files, 1,247 observations excluded)
    │
    ▼ [drv.2]
Statistical Results (JSON + report)
    │
    ├──────────────────┬───────────────────┐
    ▼ [drv.3]          ▼ [drv.4]           │
Sensitivity Report   4 Figures           │
    │                   │                  │
    └───────────────────┴──────────────────┘
                       │
                       ▼
               Pipeline Complete
```

## Key Findings

From drv.2 (Statistical Analysis):
- Warming trend: +0.67°C/decade
- Acceleration: Significant (p = 0.003)
- Breakpoint: 2005

From drv.3 (Sensitivity Analysis):
- Results robust to quality threshold variations
- Baseline period has minor effect (<5% change)
- Theil-Sen gives similar results to OLS

## Generated Outputs

- `data/cleaned/*.csv` (12 files)
- `analysis/cleaning-report.md`
- `analysis/statistical-results.json`
- `analysis/statistical-report.md`
- `analysis/sensitivity-report.md`
- `analysis/figures/*.png` (4 files)
```

## Verifying the Pipeline

```bash
chant show 001-drv
```

```
Spec: 2026-01-20-001-drv
Type: driver
Status: completed

Members:
  [x] 2026-01-20-001-drv.1 (Data cleaning) - completed
  [x] 2026-01-20-001-drv.2 (Statistical analysis) - completed
  [x] 2026-01-20-001-drv.3 (Sensitivity analysis) - completed
  [x] 2026-01-20-001-drv.4 (Figure generation) - completed

Acceptance Criteria:
  [x] All four member specs created
  [x] Dependencies correctly specified
  [x] All members complete successfully
  [x] Pipeline report summarizing all steps
```

## Pipeline Benefits

| Benefit | How It Helps Sarah |
|---------|-------------------|
| **Reproducibility** | Pipeline can be re-run identically |
| **Provenance** | Each output traces to specific inputs |
| **Parallelism** | Independent steps run concurrently |
| **Documentation** | Pipeline report captures the whole workflow |

## What's Next

With the pipeline complete, Sarah needs to handle ongoing changes:

**[Drift Detection](05-drift-detection.md)** — Detecting when new data or papers arrive and re-verifying analysis
