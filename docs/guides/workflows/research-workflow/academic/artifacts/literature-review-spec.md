---
type: research
status: completed
labels:
  - dissertation
  - arctic-warming
  - literature
prompt: research-synthesis
informed_by:
  - papers/**/*.pdf
  - .chant/context/arctic-research/paper-index.md
target_files:
  - analysis/literature-review.md
  - analysis/research-gaps.md
completed_at: 2026-01-15T14:30:00Z
model: claude-sonnet-4-20250514
---

# Synthesize Arctic warming literature

## Problem

25 papers on Arctic warming need systematic synthesis. I need to identify:
- Consensus findings on acceleration patterns
- Methodological approaches for temperature analysis
- Gaps in current research that my dissertation can address

## Research Questions

- [x] What is the consensus on Arctic amplification magnitude?
- [x] Which feedback mechanisms are well-established vs. debated?
- [x] What data sources and methodologies are standard?
- [x] What temporal patterns (seasonal, decadal) are documented?
- [x] What gaps exist that my research can address?

## Methodology

1. Read all papers in `papers/` directory
2. Extract key findings on acceleration patterns
3. Identify methodological standards from IPCC/NOAA papers
4. Map areas of consensus vs. debate
5. Document research gaps relevant to my thesis question

## Acceptance Criteria

- [x] All 25 papers reviewed and cited
- [x] Themes organized by: acceleration, feedback, methodology
- [x] Consensus vs. debate clearly distinguished
- [x] 3+ research gaps identified with supporting citations
- [x] literature-review.md written with proper citations
- [x] research-gaps.md identifies dissertation contribution

## Findings Summary

### Consensus Points
- Arctic warming 2-4x faster than global average
- Amplification factor increasing over time
- Sea ice loss is primary driver

### Research Gaps Identified
1. Post-2015 acceleration quantification
2. Station-level variability analysis
3. Seasonal acceleration patterns

### Output Files
- `analysis/literature-review.md` — Full synthesis with 25 citations
- `analysis/research-gaps.md` — Three gaps with dissertation positioning
