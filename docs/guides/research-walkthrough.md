# Research Walkthrough: Evaluating a Microservices Migration

This guide walks through a research workflow using a realistic scenario. Investigation, analysis, documentation, implementation, and maintenance each appear naturally as the evaluation progresses. Command examples and output are illustrative -- your exact output will differ.

These same patterns apply equally to academic research. A PhD student synthesizing papers and analyzing datasets would use the same spec types (`research` with `informed_by:`, `research` with `origin:`, `documentation` with `tracks:`, `driver` with members) and the same drift detection workflow. The difference is domain, not method.

## Scenario

Alex Torres is a Staff Engineer at TechCorp. Leadership wants to know whether to migrate their 80K-line Python monolith to microservices -- and if so, which pieces to extract first. Alex has six weeks.

The monolith has symptoms that suggest extraction might help: a 45-minute test suite, deployment coupling across three teams, and a reporting module that consumes 78% of CPU while auth barely registers. Alex has seen migrations fail when teams skip investigation, so this time the work starts with structured research.

Alex initializes chant in the monolith repository and creates directories for the investigation outputs:

```bash
$ chant init --agent claude
```

The plan is four weeks: codebase analysis, architecture documentation, a proof-of-concept extraction, then ongoing maintenance.

## Week 1: Investigation

Alex creates a research spec to analyze codebase coupling. The `informed_by:` field tells the agent what source material to read -- in this case, Python files and production metrics rather than academic papers:

```bash
$ chant add "Analyze codebase coupling for microservices evaluation"
Created spec: 2026-01-27-001-cpl
```

Alex edits the spec to add structure: research questions about import dependencies, database coupling, and cyclomatic complexity. The frontmatter declares what to read and what to produce:

```yaml
---
type: research
status: pending
informed_by:
  - src/**/*.py
  - .chant/context/migration-research/production-metrics.md
target_files:
  - analysis/coupling/dependency-matrix.md
  - analysis/coupling/extraction-candidates.md
  - analysis/coupling/risk-assessment.md
---
```

Then Alex runs it:

```bash
$ chant work 001-cpl
```

The agent reads all 342 Python files, analyzes import patterns, and generates three output files. The dependency matrix reveals the structure: `users` is a hub with 20 incoming dependencies, `reporting` is a leaf with zero, and `utils` has 92 incoming dependencies creating implicit coupling everywhere. Three circular dependency chains complicate extraction ordering.

The extraction candidates file ranks modules by extractability. Reporting scores 8.5/10 -- zero incoming dependencies, clear API boundary, distinct resource profile, single team ownership. Users scores 2.1 -- it's the hub, extract last.

Alex reviews the completed spec with `chant show 001-cpl` -- all five research questions answered, all acceptance criteria checked off, 342 source files read. When leadership asks "How did you conclude reporting should be first?", Alex points to the spec and its analysis files. The provenance is explicit: methodology documented in the spec itself, inputs declared in `informed_by:`.

## Week 2: Architecture Documentation

With investigation complete, Alex creates documentation that stays synchronized with the codebase. A documentation spec uses `tracks:` to link docs to source code:

```bash
$ chant add "Document system architecture"
Created spec: 2026-01-28-001-doc
```

The spec declares a dependency on the coupling analysis (so it runs second), reads the analysis results via `informed_by:`, and tracks the actual source files:

```yaml
---
type: documentation
status: pending
depends_on:
  - 2026-01-27-001-cpl
informed_by:
  - analysis/coupling/dependency-matrix.md
  - analysis/coupling/extraction-candidates.md
tracks:
  - src/auth/**/*.py
  - src/users/**/*.py
  - src/billing/**/*.py
  - src/reporting/**/*.py
  - src/notifications/**/*.py
target_files:
  - docs/architecture/overview.md
  - docs/architecture/modules.md
  - docs/architecture/data-flow.md
  - docs/architecture/decisions.md
---
```

```bash
$ chant work 001-doc
```

The agent reads the coupling analysis, reads all tracked source files, and generates four documentation files: an ASCII architecture diagram, per-module docs with endpoints and dependency counts, data flow diagrams, and three ADRs with source evidence linking back to specific files.

The `tracks:` field creates a bidirectional link. When tracked code changes, drift detection flags the docs as stale. At this point the files are fresh:

```bash
$ chant drift 001-doc

Spec: 2026-01-28-001-doc (Document system architecture)
Status: UP TO DATE

Tracked files (152) unchanged since completion (2026-01-28)
```

## Week 3: Implementation

Based on the investigation and documentation, Alex coordinates a proof-of-concept extraction of the reporting service using a driver spec. A driver coordinates multiple member specs, ensuring proper sequencing:

```bash
$ chant add "Extract reporting service POC"
Created spec: 2026-02-01-001-ext
```

The driver depends on both prior specs and declares four members:

```yaml
---
type: driver
status: pending
depends_on:
  - 2026-01-27-001-cpl
  - 2026-01-28-001-doc
informed_by:
  - analysis/coupling/extraction-candidates.md
  - analysis/coupling/risk-assessment.md
members:
  - 2026-02-01-001-ext.1  # API extraction
  - 2026-02-01-001-ext.2  # Database migration
  - 2026-02-01-001-ext.3  # Test suite
  - 2026-02-01-001-ext.4  # Integration testing
---
```

The first three members can run in parallel -- API extraction, database migration, and test suite have no dependencies on each other. The fourth member, integration testing, depends on all three. Alex creates each member spec with appropriate `depends_on:` declarations, then runs the driver:

```bash
$ chant work 001-ext

Working: 2026-02-01-001-ext (Extract reporting service POC)

Phase 1: Parallel
  [ok] 2026-02-01-001-ext.1 (API extraction) - 8m 22s
  [ok] 2026-02-01-001-ext.2 (Database migration) - 12m 45s
  [ok] 2026-02-01-001-ext.3 (Test suite) - 6m 18s

Phase 2: Sequential (depends on all above)
  [ok] 2026-02-01-001-ext.4 (Integration testing) - 18m 30s

Driver complete. All 4 members succeeded.
```

The integration testing member produces a validation report comparing the extracted service against the monolith. The results are encouraging: P99 latency drops 21% (from 2,400ms to 1,890ms), error rate improves slightly, and a 24-hour parallel run produces zero discrepancies. The extracted service creates a clean directory structure under `services/reporting/` with its own FastAPI application, Dockerfile, tests, and Terraform infrastructure.

## Week 4+: Maintenance

Code changes constantly. Documentation and analysis must keep up. Alex uses drift detection to monitor three types of change.

**Documentation drift** catches stale docs. Weeks later, a developer adds a new `/reports/realtime` endpoint. Alex checks:

```bash
$ chant drift 001-doc

Spec: 2026-01-28-001-doc (Document system architecture)
Status: DRIFT DETECTED

Tracked files changed since completion (2026-01-28):
  - services/reporting/src/routes.py
      Modified: 2026-03-10
      Lines changed: +35, -0
      New function: get_realtime_report

Recommendation: Re-run spec to update documentation
  $ chant reset 001-doc && chant work 001-doc
```

**Analysis drift** catches outdated findings. When new production metrics arrive:

```bash
$ chant drift 001-cpl

Spec: 2026-01-27-001-cpl (Analyze codebase coupling)
Status: DRIFT DETECTED

Informed-by files changed since completion (2026-01-27):
  - .chant/context/migration-research/production-metrics.md
      Modified: 2026-03-15
```

**Cascade drift** follows dependency chains. When the coupling analysis drifts, everything downstream is affected:

```bash
$ chant drift

Cascade detected:

2026-01-27-001-cpl (coupling analysis)
  DRIFT: production-metrics.md changed
    |
    +-> 2026-01-28-001-doc (documentation) - CASCADE DRIFT
    +-> 2026-02-01-001-ext (extraction) - CASCADE DRIFT

Recommendation: Re-run upstream spec first
  1. chant reset 001-cpl && chant work 001-cpl
  2. chant reset 001-doc && chant work 001-doc
```

For selective re-runs, Alex resets and re-executes just what is needed:

```bash
$ chant reset 001-doc
$ chant work 001-doc
```

For ongoing monitoring, Alex creates a weekly coupling report spec:

```yaml
---
type: research
status: pending
schedule: weekly
informed_by:
  - src/**/*.py
origin:
  - .chant/context/migration-research/production-metrics.md
target_files:
  - reports/weekly/coupling-status.md
---
```

Each week, Alex runs `chant work 001-wkly` to recalculate the dependency matrix, compare to the previous week, and flag any new circular dependencies or shifts in extraction scores.

## The Complete Workflow

Alex's four-week evaluation used all of chant's research patterns:

```
Week 1: Investigation
  research spec (informed_by: src/**) -> Coupling analysis

Week 2: Documentation
  documentation spec (tracks: src/**) -> Architecture docs

Week 3: Implementation
  driver spec (members: .1-.4) -> POC extraction

Week 4+: Maintenance
  drift detection + scheduled specs -> Ongoing updates
```

The key concepts at work:

| Concept | What it does |
|---------|-------------|
| **`informed_by:`** | Declares what sources the agent should read |
| **`origin:`** | Declares what data files are analyzed (triggers drift) |
| **`tracks:`** | Links documentation to source code (triggers drift) |
| **`depends_on:`** | Ensures specs run in the correct order |
| **Driver specs** | Coordinate multi-step work with parallel and sequential phases |
| **Drift detection** | Catches when inputs change and findings may be stale |
| **`chant reset`** | Returns a completed spec to pending for re-execution |

## Reference Implementation

The **[developer artifacts](workflows/research-workflow/developer/artifacts/)** directory contains concrete spec examples:

| File | What it shows |
|------|---------------|
| `coupling-analysis-spec.md` | Completed research spec for codebase coupling analysis |
| `architecture-docs-spec.md` | Completed documentation spec with `tracks:` linkage |
| `extraction-driver-spec.md` | Completed driver spec coordinating four extraction phases |
| `weekly-coupling-spec.md` | Pending scheduled research spec for ongoing monitoring |

The **[academic artifacts](workflows/research-workflow/academic/artifacts/)** directory contains parallel examples for the academic path (literature review, data analysis, pipeline driver, monthly update).

## Further Reading

- [Research Workflows](research.md) -- Concepts and spec types for research
- [Lifecycle](../concepts/lifecycle.md) -- State machine and transitions
- [Dependencies](../concepts/deps.md) -- Dependency resolution and blocking
- [CLI Reference](../reference/cli.md) -- Full command documentation
