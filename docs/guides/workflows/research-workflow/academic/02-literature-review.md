# Week 1: Literature Review

Sarah begins with a systematic literature review. This phase synthesizes 25 peer-reviewed papers to identify themes, gaps, and the current state of Arctic warming research.

## The Synthesis Pattern

Literature review uses the **synthesis pattern** — reading multiple sources to extract themes:

```
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│  Paper 1    │   │  Paper 2    │   │  Paper N    │
│  (PDF)      │   │  (PDF)      │   │  (PDF)      │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │                 │                 │
       └────────────────┬┴─────────────────┘
                        │
                        ▼
              ┌─────────────────────┐
              │   Research Spec     │
              │  informed_by:       │
              │   - papers/*.pdf    │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │  Literature Review  │
              │  (Themes & Gaps)    │
              └─────────────────────┘
```

The `informed_by:` field tells the agent what to read. The agent synthesizes patterns, not raw data.

## Preparing the Papers

Sarah organizes her paper collection:

```bash
# Papers organized by topic
papers/
├── acceleration/
│   ├── chen-2024-arctic-amplification.pdf
│   ├── morrison-2023-feedback-loops.pdf
│   └── wang-2022-ice-albedo.pdf
├── methodology/
│   ├── ipcc-2023-data-standards.pdf
│   └── noaa-2024-station-calibration.pdf
└── regional/
    ├── greenland-2023-ice-sheet.pdf
    └── siberia-2024-permafrost.pdf
```

Sarah creates a markdown index for the agent:

**File: `.chant/context/arctic-research/paper-index.md`**

```markdown
# Arctic Warming Paper Index

## Acceleration Studies

| Paper | Year | Key Finding |
|-------|------|-------------|
| Chen et al. | 2024 | Arctic amplification 2.5x global average |
| Morrison & Lee | 2023 | Ice-albedo feedback accelerating since 2010 |
| Wang et al. | 2022 | Ocean heat transport increasing |

## Methodology

| Paper | Year | Focus |
|-------|------|-------|
| IPCC WG1 | 2023 | Data quality standards for temperature records |
| NOAA Technical | 2024 | Station calibration procedures |

## Regional Studies

| Paper | Year | Region |
|-------|------|--------|
| Jensen et al. | 2023 | Greenland ice sheet dynamics |
| Petrov et al. | 2024 | Siberian permafrost thaw |

Total papers: 25
```

## Creating the Literature Review Spec

Sarah creates a research spec for the synthesis:

```bash
chant add "Synthesize Arctic warming literature" --type research
```

She edits the spec to add detailed structure:

**File: `.chant/specs/2026-01-15-001-lit.md`**

```yaml
---
type: research
status: pending
prompt: research-synthesis
informed_by:
  - papers/**/*.pdf
  - .chant/context/arctic-research/paper-index.md
target_files:
  - analysis/literature-review.md
  - analysis/research-gaps.md
---
```

```markdown
# Synthesize Arctic warming literature

## Problem

25 papers on Arctic warming need systematic synthesis. I need to identify:
- Consensus findings on acceleration patterns
- Methodological approaches for temperature analysis
- Gaps in current research that my dissertation can address

## Research Questions

- [ ] What is the consensus on Arctic amplification magnitude?
- [ ] Which feedback mechanisms are well-established vs. debated?
- [ ] What data sources and methodologies are standard?
- [ ] What temporal patterns (seasonal, decadal) are documented?
- [ ] What gaps exist that my research can address?

## Methodology

1. Read all papers in `papers/` directory
2. Extract key findings on acceleration patterns
3. Identify methodological standards from IPCC/NOAA papers
4. Map areas of consensus vs. debate
5. Document research gaps relevant to my thesis question

## Acceptance Criteria

- [ ] All 25 papers reviewed and cited
- [ ] Themes organized by: acceleration, feedback, methodology
- [ ] Consensus vs. debate clearly distinguished
- [ ] 3+ research gaps identified with supporting citations
- [ ] literature-review.md written with proper citations
- [ ] research-gaps.md identifies dissertation contribution
```

## Running the Synthesis

Sarah executes the spec:

```bash
chant work 001-lit
```

The agent reads all papers (via `informed_by:`), extracts themes, and produces the literature review.

## Agent Output

After execution, Sarah finds two new files:

**File: `analysis/literature-review.md`** (generated)

```markdown
# Arctic Warming Literature Review

## 1. Arctic Amplification

### Consensus Findings

All 25 papers agree on core amplification:
- Arctic warming 2-4x faster than global average (Chen 2024, Morrison 2023)
- Amplification factor increasing over time (Wang 2022)
- Sea ice loss is primary driver (IPCC 2023)

### Magnitude Estimates

| Study | Amplification Factor | Period |
|-------|---------------------|--------|
| Chen et al. 2024 | 2.5x | 1990-2023 |
| Morrison & Lee 2023 | 3.1x | 2000-2023 |
| IPCC AR6 WG1 | 2.0-4.0x | 1979-2021 |

### Areas of Debate

The exact contribution of each feedback mechanism remains contested:
- Ice-albedo feedback: 40-60% of amplification (range across studies)
- Ocean heat transport: 20-35% contribution
- Atmospheric circulation: 10-25% contribution

## 2. Feedback Mechanisms

### Well-Established

1. **Ice-Albedo Feedback** — Surface reflectivity decreases as ice melts
   - Supported by satellite observations (Jensen 2023)
   - Quantified in all modeling studies reviewed

2. **Lapse Rate Feedback** — Arctic troposphere warms faster
   - Observational evidence from radiosonde data
   - Consistent across all climate models

### Under Investigation

1. **Cloud Feedback** — Direction and magnitude uncertain
   - Petrov 2024: Clouds may amplify OR dampen warming
   - Key uncertainty in projections

## 3. Methodological Standards

### Data Sources

Papers use consistent data sources:
- NOAA Global Historical Climatology Network (12 papers)
- ERA5 Reanalysis (8 papers)
- MODIS satellite observations (15 papers)

### Quality Requirements

IPCC 2023 and NOAA 2024 establish standards:
- Minimum 30-year record for trend detection
- Station calibration requirements
- Homogenization procedures for historical data

## 4. Citations

[Full citation list with 25 entries...]
```

**File: `analysis/research-gaps.md`** (generated)

```markdown
# Research Gaps and Dissertation Contribution

## Identified Gaps

### Gap 1: Acceleration Quantification Post-2015

**Current State:** Most acceleration studies end at 2015-2020.
**Gap:** Limited analysis of whether acceleration itself is accelerating.
**My Contribution:** Extend analysis to 2024 with focus on post-Paris Agreement period.

**Supporting Citations:**
- Chen 2024 notes "further investigation needed for most recent decade"
- Morrison 2023 calls for "updated acceleration estimates"

### Gap 2: Station-Level Variability

**Current State:** Most studies use gridded/reanalysis data.
**Gap:** Individual station behavior not well characterized.
**My Contribution:** Analyze 12 individual monitoring stations for local patterns.

**Supporting Citations:**
- NOAA 2024 recommends "station-level analysis to validate gridded products"

### Gap 3: Seasonal Acceleration Patterns

**Current State:** Annual averages dominate the literature.
**Gap:** Seasonal differences in acceleration rates.
**My Contribution:** Decompose acceleration by season (DJF, MAM, JJA, SON).

**Supporting Citations:**
- Wang 2022 notes "winter amplification may differ from annual"

## Dissertation Positioning

These gaps support my thesis question:
> Have Arctic temperature increases accelerated since 1995, and what factors drive the acceleration?

My analysis will address all three gaps by:
1. Including data through 2024
2. Analyzing individual stations (not just gridded data)
3. Examining seasonal patterns in acceleration
```

## Verifying the Synthesis

Sarah reviews the output and checks the spec:

```bash
chant show 001-lit
```

```
Spec: 2026-01-15-001-lit
Type: research
Status: completed

Research Questions:
  [x] What is the consensus on Arctic amplification magnitude?
  [x] Which feedback mechanisms are well-established vs. debated?
  [x] What data sources and methodologies are standard?
  [x] What temporal patterns (seasonal, decadal) are documented?
  [x] What gaps exist that my research can address?

Acceptance Criteria:
  [x] All 25 papers reviewed and cited
  [x] Themes organized by: acceleration, feedback, methodology
  [x] Consensus vs. debate clearly distinguished
  [x] 3+ research gaps identified with supporting citations
  [x] literature-review.md written with proper citations
  [x] research-gaps.md identifies dissertation contribution

Informed by: papers/**/*.pdf (25 files)
Generated: analysis/literature-review.md, analysis/research-gaps.md
```

## The Provenance Trail

The completed spec now documents:
- Exactly which papers were synthesized
- The research questions asked
- The methodology used
- When the synthesis was done

If Sarah's advisor asks "How did you identify this gap?", she can point to the spec and its `informed_by:` sources.

## What's Next

With the literature synthesized, Sarah moves to data analysis:

**[Data Analysis](03-data-analysis.md)** — Analyzing 30 years of temperature data using research specs with `origin:`
