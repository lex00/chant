# Research Workflow Guide

A complete walkthrough showing how chant orchestrates research workflows from investigation through implementation. Two parallel paths demonstrate the same patterns in different contexts.

## Why This Guide?

Research workflows share common patterns regardless of domain:

1. **Investigate** — Gather and synthesize sources
2. **Analyze** — Process data to generate findings
3. **Document** — Capture insights for others
4. **Implement** — Act on findings
5. **Maintain** — Keep findings current as inputs change

Chant's spec types map directly to these phases. This guide shows how.

## Two Paths, Same Patterns

This guide follows two researchers using identical chant patterns in different domains:

| Path | Protagonist | Research Goal |
|------|-------------|---------------|
| **[Academic](academic/01-scenario.md)** | Dr. Sarah Chen, PhD student | Analyze 30 years of Arctic temperature data |
| **[Developer](developer/01-scenario.md)** | Alex Torres, Staff Engineer | Evaluate microservices migration for 80K LOC monolith |

Both paths demonstrate:
- Research specs with `informed_by:` (synthesis)
- Research specs with `origin:` (analysis)
- Documentation specs with `tracks:`
- Driver specs for multi-phase coordination
- Drift detection and re-verification

## The Universal Research Pattern

```
┌──────────────────────────────────────────────────────────────────┐
│                    Research Workflow Phases                       │
└──────────────────────────────────────────────────────────────────┘

Phase 1: Investigation
┌─────────────────────┐
│   Research Spec     │  informed_by: [papers|code|docs]
│    (synthesis)      │  → Synthesize sources, identify patterns
└──────────┬──────────┘
           │
           ▼
Phase 2: Analysis
┌─────────────────────┐
│   Research Spec     │  origin: [data|logs|metrics]
│    (analysis)       │  → Analyze data, generate findings
└──────────┬──────────┘
           │
           ├────────────────────────────┐
           ▼                            ▼
Phase 3: Documentation              Implementation
┌─────────────────────┐         ┌─────────────────────┐
│  Documentation      │         │    Driver Spec      │
│      Spec           │         │   (coordinate)      │
│                     │         └──────────┬──────────┘
│  tracks: [code]     │              ┌─────┴─────┬─────────┐
│  → Capture          │              ▼           ▼         ▼
│    findings         │           Code        Code      Code
└─────────────────────┘           (.1)        (.2)      (.3)
           │                       │           │         │
           │                       └───────────┴─────────┘
           │                                │
           ▼                                ▼
Phase 4: Maintenance
┌──────────────────────────────────────────────────────────────────┐
│              Drift Detection & Re-verification                    │
│                                                                   │
│   • origin: files change → re-analyze                            │
│   • tracks: code changes → update docs                           │
│   • informed_by: sources change → review                         │
└──────────────────────────────────────────────────────────────────┘
```

## How the Paths Overlap

Both paths use the same spec types in the same order. The table below maps each phase:

| Phase | Academic Example | Developer Example | Spec Types |
|-------|------------------|-------------------|------------|
| **Investigation** | Literature review (25 papers) | Codebase analysis (`src/**`) | `research` with `informed_by:` |
| **Analysis** | Statistical analysis of climate data | Performance metrics analysis | `research` with `origin:` |
| **Documentation** | Write methodology section | Document architecture decisions | `documentation` with `tracks:` |
| **Implementation** | Data processing pipeline | POC microservice extraction | `code`, `driver`, `depends_on:` |
| **Pipeline** | Phase 1→2→3 dependencies | Service-by-service rollout | `driver` with member specs |
| **Maintenance** | New data triggers re-analysis | Code changes trigger doc updates | drift detection |

## Spec Type Reference

| Spec Type | Purpose | Key Fields | Example |
|-----------|---------|------------|---------|
| `research` | Investigation and analysis | `informed_by:`, `origin:` | Synthesize papers, analyze data |
| `documentation` | Capture and communicate | `tracks:` | Architecture docs, methodology |
| `code` | Implement changes | `target_files:` | Scripts, services, utilities |
| `driver` | Coordinate phases | `members:` | Multi-step pipelines |
| `task` | Non-code work | `target_files:` | Reports, presentations |

## Choose Your Path

**[Academic Path](academic/01-scenario.md)** — Follow Dr. Sarah Chen through:
- Literature review of 25 climate science papers
- Statistical analysis of Arctic temperature datasets
- Multi-phase data pipeline with dependencies
- Drift detection when new data is published

**[Developer Path](developer/01-scenario.md)** — Follow Alex Torres through:
- Codebase coupling analysis of 80K LOC monolith
- Performance metrics analysis from production logs
- Architecture documentation that tracks source code
- Drift detection when code or metrics change

## Key Concepts Demonstrated

- **Synthesis vs. Analysis** — `informed_by:` for reading sources, `origin:` for processing data
- **Dependency chains** — `depends_on:` for phase ordering
- **Driver coordination** — Decomposing complex work into parallel member specs
- **Documentation tracking** — `tracks:` keeps docs synchronized with source
- **Drift detection** — Know when inputs change, trigger re-verification

## Prerequisites

Familiarity with:
- [Core concepts](../../../concepts/specs.md)
- [Spec types](../../../concepts/spec-types.md)
- [Research workflows](../../research.md)
- [Dependencies](../../../concepts/deps.md)

## See Also

- [Research workflow examples](../../../../examples/research-workflow/) — Concrete examples demonstrating these patterns
