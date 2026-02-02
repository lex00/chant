---
type: research
prompt: research-analysis
informed_by:
  - src/**/*.rs
target_files:
  - analysis/tech-debt-report.md
---
# Investigate technical debt in authentication module

## Research Questions
- [ ] Where are the TODO/FIXME comments located?
- [ ] Which functions have highest cyclomatic complexity?
- [ ] What error handling patterns are inconsistently applied?
- [ ] Are there any security concerns (hardcoded values, unsafe operations)?

## Acceptance Criteria
- [ ] All source files scanned and analyzed
- [ ] Issues categorized by severity (critical, high, medium, low)
- [ ] Complexity metrics calculated for each function
- [ ] Recommendations provided with specific file locations
- [ ] Report generated in target location
