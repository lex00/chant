# The Research Context

## Dr. Sarah Chen

Dr. Sarah Chen is a third-year PhD student in Climate Science at Northern University. Her dissertation focuses on Arctic temperature acceleration patterns over the past three decades.

| Attribute | Value |
|-----------|-------|
| Program | PhD, Climate Science |
| Year | Third year |
| Advisor | Prof. James Morrison |
| Funding | NSF Arctic Research Grant |
| Timeline | Dissertation defense in 18 months |

## Research Goal

**Thesis Question:** Have Arctic temperature increases accelerated since 1995, and what factors drive the acceleration?

Sarah's analysis requires:
- Synthesizing 25+ peer-reviewed papers on Arctic warming
- Analyzing 30 years of temperature data from 12 monitoring stations
- Processing satellite imagery datasets
- Producing reproducible statistical analysis

## The Reproducibility Challenge

Sarah's field has a reproducibility problem. A 2024 meta-analysis found:

| Issue | Prevalence |
|-------|-----------|
| Methods under-specified | 67% of papers |
| Data not preserved | 45% of papers |
| Analysis steps not documented | 72% of papers |
| Results not reproducible | 38% when attempted |

Her advisor emphasizes: "Every finding must trace back to specific data and methodology. Your dissertation defense will include a reproducibility audit."

## Current Research State

Sarah has:
- Downloaded 30 years of temperature data (CSV files, ~2GB)
- Collected 25 papers on Arctic warming patterns
- Rough notes on initial observations
- No systematic approach to tracking analysis

Her pain points:
- Literature notes scattered across Notion, PDFs, and text files
- Analysis scripts in various Jupyter notebooks, unclear dependencies
- Uncertain which findings came from which data version
- No way to know when new data invalidates old conclusions

## Why Chant?

Chant addresses each challenge:

| Challenge | Chant Solution |
|-----------|----------------|
| Scattered notes | `informed_by:` links findings to sources |
| Unclear dependencies | `depends_on:` chains analysis phases |
| Data versioning | `origin:` tracks input data files |
| Result staleness | Drift detection alerts when inputs change |
| Method documentation | Spec IS the methodology |

## Project Setup

Sarah initializes chant in her dissertation repository:

```bash
# Initialize chant
chant init --agent claude

# Create directories for research data
mkdir -p data/temperature
mkdir -p data/satellite
mkdir -p papers
mkdir -p analysis

# Create context directory for literature synthesis
mkdir -p .chant/context/arctic-research
```

Directory structure:

```
dissertation/
├── .chant/
│   ├── specs/           # Research specs live here
│   ├── context/         # Human-curated summaries
│   │   └── arctic-research/
│   └── config.md
├── data/
│   ├── temperature/     # 30 years of station data
│   └── satellite/       # Imagery datasets
├── papers/              # PDF collection
└── analysis/            # Output: findings, figures
```

## Research Timeline

Sarah plans a four-week research phase:

```
Week 1          Week 2              Week 3            Week 4
┌──────────┐   ┌───────────────┐   ┌──────────────┐   ┌──────────────┐
│Literature│   │    Data       │   │   Pipeline   │   │  Write-up &  │
│  Review  │──>│   Analysis    │──>│ Coordination │──>│   Ongoing    │
│ (Papers) │   │ (Statistics)  │   │  (Driver)    │   │   Drift      │
└──────────┘   └───────────────┘   └──────────────┘   └──────────────┘
```

## Spec Workflow Preview

Sarah's research will use these spec types:

| Week | Spec Type | Purpose |
|------|-----------|---------|
| 1 | `research` with `informed_by:` | Synthesize 25 papers into themes |
| 2 | `research` with `origin:` | Analyze temperature data |
| 3 | `driver` with members | Coordinate multi-step pipeline |
| 4+ | Drift detection | Alert when new data arrives |

## Team Structure

Unlike enterprise scenarios, Sarah works largely alone, but chant's orchestrator pattern still applies:

- **Sarah** creates specs, reviews findings, validates methodology
- **Chant agents** synthesize literature, run statistical analysis
- **Git** provides version control and audit trail
- **Drift detection** runs when data files change

## What's Next

With the project initialized, Sarah begins the literature review phase:

**[Literature Review](02-literature-review.md)** — Synthesizing 25 papers using research specs with `informed_by:`
