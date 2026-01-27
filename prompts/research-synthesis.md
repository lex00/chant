---
name: research-synthesis
purpose: Synthesize multiple sources into coherent findings and recommendations
---

# Research Synthesis

You are performing research synthesis for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Synthesis Process

### 1. Understand Your Input Sources

**Informed by files** (materials to synthesize):
- These are the sources you will read, compare, and synthesize
- May include research papers, documentation, prior analyses, code samples
- Read all sources completely before beginning synthesis

**Target files** (output):
- Specify the format(s) you must produce
- Typically Markdown for synthesis reports

### 2. Read and Understand All Sources

1. **Read all informed_by files** - Understand each source completely
   - Take notes on key points, arguments, and findings
   - Note the context and perspective of each source
   - Identify the source's methodology and scope

2. **Summarize each source** - Create a brief summary
   - Key claims or findings
   - Evidence and methodology
   - Limitations and scope

3. **Track provenance** - Know where each idea comes from
   - Use citations and references
   - Be able to attribute claims to sources

### 3. Identify Themes and Patterns

After understanding all sources:

1. **Find agreements** - Where do sources agree?
   - Common findings across multiple sources
   - Shared recommendations or conclusions
   - Consensus on best practices

2. **Find conflicts** - Where do sources disagree?
   - Contradictory findings or claims
   - Different recommendations
   - Explain the reason for disagreement if possible

3. **Find gaps** - What's missing?
   - Questions not addressed by any source
   - Areas needing more research
   - Limitations in available information

4. **Identify patterns** - What emerges across sources?
   - Common themes or categories
   - Trends or trajectories
   - Underlying principles

### 4. Apply Synthesis Principles

- **Comprehensive** — Consider all sources, not just the ones that agree
- **Comparative** — Note agreements, conflicts, and nuances
- **Analytical** — Draw conclusions supported by evidence
- **Actionable** — Provide clear recommendations
- **Balanced** — Represent different viewpoints fairly

### 5. Format Guidelines by Synthesis Type

#### Literature Review

For synthesizing research papers or technical documents:

```markdown
# Literature Review: Topic Name

## Executive Summary

Key findings in 3-5 bullet points.

## Source Overview

| Source | Focus | Key Finding | Methodology |
|--------|-------|-------------|-------------|
| Smith 2025 | X | Finding Y | Method Z |
| Jones 2024 | A | Finding B | Method C |

## Thematic Analysis

### Theme 1: Topic Name

**Summary**: What the sources collectively say about this theme.

**Supporting Evidence**:
- Source 1 says... (citation)
- Source 2 agrees/disagrees... (citation)

**Gaps**: What remains unknown or contested.

### Theme 2: Topic Name

[Same structure]

## Points of Agreement

- Point 1 (supported by: Source 1, Source 2, Source 3)
- Point 2 (supported by: ...)

## Points of Conflict

### Conflict 1: Description

- **Position A**: (Sources 1, 3) — Explanation
- **Position B**: (Source 2) — Explanation
- **Analysis**: Why they might differ, which is more credible

## Research Gaps

- Gap 1: What's unknown and why it matters
- Gap 2: ...

## Recommendations

Based on the synthesis:

1. **Recommendation 1** — Rationale with citations
2. **Recommendation 2** — Rationale with citations

## References

Full citations for all sources.
```

#### Library/Technology Comparison

For comparing libraries, frameworks, or technologies:

```markdown
# Comparison: Options for [Purpose]

## Executive Summary

Recommended choice and why (2-3 sentences).

## Options Considered

1. **Option A** — Brief description
2. **Option B** — Brief description
3. **Option C** — Brief description

## Feature Comparison

| Feature | Option A | Option B | Option C |
|---------|----------|----------|----------|
| Feature 1 | ✓ Full | ◐ Partial | ✗ None |
| Feature 2 | ✓ | ✓ | ✓ |
| Feature 3 | ✗ | ✓ | ✓ |

## Detailed Analysis

### Option A: Name

**Pros:**
- Pro 1 with evidence
- Pro 2 with evidence

**Cons:**
- Con 1 with evidence
- Con 2 with evidence

**Best for:** Use case description

### Option B: Name

[Same structure]

### Option C: Name

[Same structure]

## Trade-offs

| Consideration | Best Option | Why |
|--------------|-------------|-----|
| Performance | Option A | Reason |
| Ease of use | Option B | Reason |
| Maintenance | Option C | Reason |

## Recommendation

**Recommended: Option X**

**Rationale:**
1. Reason 1 with supporting evidence
2. Reason 2 with supporting evidence

**Caveats:**
- When Option Y might be better instead
- Risks or limitations to consider
```

#### Technical Investigation

For investigating a technical question or problem:

```markdown
# Investigation: Question or Problem

## Summary

Answer to the main question (2-3 sentences).

## Background

Context needed to understand the investigation.

## Findings

### Finding 1: Title

**Evidence:**
- Source/observation 1
- Source/observation 2

**Implications:**
What this means for the question.

### Finding 2: Title

[Same structure]

## Analysis

Synthesis of findings into coherent understanding.

## Unknowns

- What we couldn't determine and why
- What would require further investigation

## Recommendations

### Immediate Actions

1. Action 1 — Rationale
2. Action 2 — Rationale

### Further Research Needed

1. Question that needs investigation
2. Question that needs investigation

## Appendix

Supporting data, detailed evidence, or raw notes.
```

### 6. Answer Research Questions

If your spec includes specific research questions:

1. **Quote the question** — State it explicitly
2. **Provide the answer** — Clear and direct
3. **Cite sources** — Reference which sources support the answer
4. **Note uncertainty** — If the answer is partial or contested

Example:
```markdown
### Q: What is the best approach for X?

**Answer:** Approach A is recommended based on...

**Supporting evidence:**
- Source 1 found that... (citation)
- Source 2 confirms... (citation)

**Caveats:** Source 3 notes that in situation Y, Approach B may be better.
```

### 7. Verification

Before producing final output:

1. **Source coverage**: Did you consider all informed_by files?
2. **Attribution check**: Can you cite a source for each claim?
3. **Balance check**: Did you fairly represent different viewpoints?
4. **Conflict check**: Did you note and explain disagreements?
5. **Actionability check**: Are recommendations clear and justified?
6. **Acceptance criteria check**: Does output meet all stated requirements?

## Constraints

- Read all sources completely; don't skip any
- Cite sources for all claims
- Note conflicts and disagreements; don't just pick one view
- Be clear about what's well-supported vs. uncertain
- Provide actionable recommendations
- Don't introduce claims not supported by sources

## Acceptance Criteria

- [ ] All informed_by files read completely
- [ ] Each source summarized
- [ ] Key themes and patterns identified
- [ ] Agreements and conflicts noted
- [ ] Research gaps identified
- [ ] Clear recommendations provided
- [ ] All claims attributed to sources
- [ ] Output file(s) created in correct format
- [ ] All acceptance criteria from spec met
- [ ] Commit with message: `chant({{spec.id}}): <synthesis summary>`
