---
type: documentation
status: completed
labels:
  - architecture
  - documentation
  - migration
prompt: documentation
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
completed_at: 2026-01-28T14:45:00Z
model: claude-sonnet-4-20250514
---

# Document system architecture

## Problem

The codebase lacks up-to-date architecture documentation. New engineers
take 3 months to become productive, partly because there's no map.

This documentation should:
- Provide a high-level overview of the system
- Document each module's responsibilities and interfaces
- Show data flow between modules
- Capture architecture decisions

## Tracked Files

This spec tracks `src/{auth,users,billing,reporting,notifications}/**/*.py`.
When these files change, the documentation may become stale.

## Documentation Sections

### 1. Overview (overview.md)
- System purpose and boundaries
- High-level architecture diagram (ASCII)
- Technology stack
- Deployment topology

### 2. Modules (modules.md)
- Each module's purpose
- Public interfaces
- Internal structure
- Team ownership

### 3. Data Flow (data-flow.md)
- Request flow through the system
- Data dependencies between modules
- Database access patterns
- External service integrations

### 4. Decisions (decisions.md)
- Key architecture decisions (ADRs)
- Rationale for current structure
- Known limitations and tech debt

## Acceptance Criteria

- [x] Overview with architecture diagram
- [x] All 5 modules documented
- [x] Data flow diagram created
- [x] 3+ architecture decisions documented
- [x] All docs reference specific source locations

## Generated Documentation

### overview.md
- ASCII architecture diagram showing 5 modules
- Technology stack: Python 3.11, FastAPI, PostgreSQL, Redis
- Deployment: 12 production pods, Kubernetes

### modules.md
- 5 modules documented with:
  - Purpose and ownership
  - Public endpoints
  - Key files with line references
  - Dependency counts

### data-flow.md
- Request flow from load balancer to database
- Cross-module data dependencies
- Cache access patterns
- External service integrations

### decisions.md
- ADR-001: Monolithic Architecture (2017)
- ADR-002: Shared Utils Pattern (2018)
- ADR-003: PostgreSQL for All Data (2017)
