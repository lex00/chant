# Research Workflow Example

Demonstrates using Chant for research: synthesis and analysis paths.

## Overview

This example shows two main research patterns:

1. **Academic Path** - Synthesizing multiple papers into findings
2. **Developer Path** - Analyzing code to generate tech debt reports

Both paths use research specs with `informed_by:` and demonstrate how to structure research questions as checkboxes.

## Structure

```
research-workflow/
├── README.md (this file)
├── academic-path/
│   ├── .chant/specs/001-lit-review.md    # Synthesis spec
│   └── papers/                            # Source materials to synthesize
│       ├── smith2025.md
│       └── jones2024.md
└── developer-path/
    ├── .chant/specs/001-tech-debt.md     # Analysis spec
    └── src/sample-code/                   # Code to analyze
        ├── auth.rs
        ├── database.rs
        └── utils.rs
```

## Academic Path: Literature Synthesis

The academic path demonstrates synthesizing research papers.

**Spec:** `academic-path/.chant/specs/001-lit-review.md`

```yaml
---
type: research
prompt: research-synthesis
informed_by:
  - papers/smith2025.md
  - papers/jones2024.md
target_files:
  - findings/lit-review.md
---
```

**Key Features:**
- Uses `informed_by:` to reference source materials
- Research questions as checkboxes
- Target output in `findings/`
- Agent synthesizes across multiple papers

**To Run:**
```bash
cd academic-path
chant init
chant work 001
```

**Expected Output:** `findings/lit-review.md` with:
- Comparison table of approaches
- Analysis of trade-offs
- Identified research gaps

## Developer Path: Code Analysis

The developer path demonstrates analyzing code for technical debt.

**Spec:** `developer-path/.chant/specs/001-tech-debt.md`

```yaml
---
type: research
prompt: research-analysis
informed_by:
  - src/**/*.rs
target_files:
  - analysis/tech-debt-report.md
---
```

**Key Features:**
- Uses glob pattern in `informed_by:` to analyze all Rust files
- Research questions target specific code issues
- Agent scans for TODOs, complexity, security issues
- Outputs prioritized recommendations

**To Run:**
```bash
cd developer-path
chant init
chant work 001
```

**Expected Output:** `analysis/tech-debt-report.md` with:
- List of TODO/FIXME locations
- Complexity metrics
- Security concerns (hardcoded secrets, unsafe patterns)
- Prioritized action items

## What This Demonstrates

### Common Research Patterns
- **informed_by:** References materials to synthesize or analyze
- **Research questions as checkboxes** - Track what needs answering
- **target_files:** Where findings should be written
- **Drift detection** - If source materials change, spec shows drift

### Academic vs Developer Research
| Aspect | Academic Path | Developer Path |
|--------|---------------|----------------|
| Goal | Synthesize papers | Analyze codebase |
| Sources | Paper files | Code files |
| Pattern | `research-synthesis` | `research-analysis` |
| Output | Literature review | Tech debt report |
| Questions | Theory & gaps | Issues & metrics |

### Why Use Chant for Research?

1. **Reproducible** - Spec documents the methodology
2. **Traceable** - Execution logs show every step
3. **Drift Detection** - Know when sources change
4. **Systematic** - Research questions as acceptance criteria

## Next Steps

After running either path:

1. **Verify drift detection:**
   ```bash
   # Modify a source file
   echo "## New Finding" >> papers/smith2025.md
   chant verify 001  # Shows drift
   ```

2. **Replay analysis:**
   ```bash
   chant replay 001  # Re-runs with updated sources
   ```

3. **Create your own research spec:**
   ```bash
   chant add "Analyze API response times" --type research
   ```

## See Also

- [docs/guides/research.md](../../docs/guides/research.md) - Complete research guide
- [docs/concepts/spec-types.md](../../docs/concepts/spec-types.md) - Research spec type details
- [docs/concepts/prompts.md](../../docs/concepts/prompts.md) - research-synthesis and research-analysis prompts
