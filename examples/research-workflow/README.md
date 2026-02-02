# Research Workflow Example

## Overview

This example demonstrates using Chant for research through two independent paths: synthesizing multiple papers into findings (academic path) and analyzing code to generate tech debt reports (developer path). Both paths use research specs with `informed_by:` and demonstrate how to structure research questions as checkboxes.

## Structure

Two independent research paths:

1. **Academic Path** - Literature synthesis
   - **001-lit-review.md** - Research spec that synthesizes multiple papers
   - Source materials in `papers/`:
     - `smith2025.md` - First research paper
     - `jones2024.md` - Second research paper
   - Output: `findings/lit-review.md` with comparison table and analysis

2. **Developer Path** - Code analysis
   - **001-tech-debt.md** - Research spec that analyzes codebase
   - Source code in `src/sample-code/`:
     - `auth.rs` - Authentication code
     - `database.rs` - Database code
     - `utils.rs` - Utility code
   - Output: `analysis/tech-debt-report.md` with prioritized recommendations

## Usage

Run the academic synthesis path:
```bash
cd examples/research-workflow/academic-path
chant init
chant work 001
# Output: findings/lit-review.md with synthesis across papers
```

Run the developer analysis path:
```bash
cd examples/research-workflow/developer-path
chant init
chant work 001
# Output: analysis/tech-debt-report.md with tech debt findings
```

Create your own research spec:
```bash
chant add "Analyze API response times" --type research
```

## Testing

Verify drift detection:
```bash
# Modify a source file
echo "## New Finding" >> papers/smith2025.md
chant verify 001  # Shows drift

# Re-run with updated sources
chant replay 001
```

Test the workflow by:
1. Running either path to see research output generated
2. Modifying source materials to trigger drift detection
3. Using `chant verify` to check if sources changed
4. Using `chant replay` to re-run analysis with updates

Key patterns demonstrated:
- `informed_by:` references materials to synthesize or analyze
- Research questions as checkboxes track what needs answering
- `target_files:` specifies where findings should be written
- Drift detection alerts when source materials change
