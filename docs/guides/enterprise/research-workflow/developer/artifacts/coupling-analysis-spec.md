---
type: research
status: completed
labels:
  - migration
  - microservices
  - analysis
prompt: research-analysis
informed_by:
  - src/**/*.py
  - .chant/context/migration-research/production-metrics.md
target_files:
  - analysis/coupling/dependency-matrix.md
  - analysis/coupling/extraction-candidates.md
  - analysis/coupling/risk-assessment.md
completed_at: 2026-01-27T16:30:00Z
model: claude-sonnet-4-20250514
---

# Analyze codebase coupling for microservices evaluation

## Problem

TechCorp's 80K LOC Python monolith needs evaluation for microservices migration.
Before recommending extraction candidates, I need to understand:
- Which modules are tightly coupled?
- Which modules are loosely coupled and easy to extract?
- What shared state exists between modules?
- Where are the database coupling hotspots?

## Research Questions

- [x] What are the import dependencies between modules?
- [x] Which modules share database tables?
- [x] What is the cyclomatic complexity distribution?
- [x] Which modules have the most external callers?
- [x] What shared utilities create implicit coupling?

## Methodology

### Import Analysis
1. Parse all Python files for import statements
2. Build module-to-module dependency graph
3. Calculate coupling metrics (fan-in, fan-out)
4. Identify circular dependencies

### Database Coupling
1. Find all ORM model definitions
2. Map models to modules
3. Identify cross-module foreign keys
4. Analyze query patterns in each module

### Complexity Analysis
1. Calculate cyclomatic complexity per module
2. Identify high-complexity functions
3. Map complexity to test coverage
4. Flag under-tested complex code

### Extraction Scoring
1. Score each module on extractability (1-10)
2. Consider: coupling, complexity, test coverage, team ownership
3. Recommend extraction order

## Acceptance Criteria

- [x] Import dependency matrix generated
- [x] Database coupling analysis complete
- [x] Complexity metrics calculated
- [x] Extraction candidates ranked with rationale
- [x] Risk assessment for top 3 candidates
- [x] All findings written to target files

## Key Results

### Extraction Ranking

| Rank | Module | Score | Key Factor |
|------|--------|-------|------------|
| 1 | reporting | 8.5 | Zero incoming dependencies |
| 2 | notifications | 7.2 | Clear async boundary |
| 3 | billing | 5.8 | Business importance |
| 4 | auth | 4.2 | High coupling |
| 5 | users | 2.1 | Hub module |

### Critical Findings

1. **utils module** has 92 incoming dependencies (implicit coupling)
2. Three circular dependency chains found
3. reporting is a "leaf" node - ideal first extraction
4. users is a "hub" - extract last

### Recommended Extraction Order

1. Reporting (standalone)
2. Notifications (after preference refactor)
3. Billing (after event bus setup)
4. Auth (after users interface stabilizes)
5. Users (becomes core service)
