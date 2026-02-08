# Week 2: Architecture Documentation

With investigation complete, Alex creates architecture documentation that stays synchronized with the codebase. This phase uses documentation specs with the `tracks:` field.

## The Documentation Pattern

Documentation specs create a linkage between docs and source code:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Documentation Spec                            │
└─────────────────────────────────────────────────────────────────┘

     informed_by:                tracks:                  target:
   ┌──────────────┐         ┌──────────────┐         ┌──────────────┐
   │  Analysis    │         │   Source     │         │   Output     │
   │  Results     │    +    │    Code      │    =    │    Docs      │
   │              │         │              │         │              │
   └──────────────┘         └──────────────┘         └──────────────┘

   Previous research        Code being               Generated
   findings                  documented               documentation
```

The `tracks:` field creates a bidirectional link:
- Docs are generated FROM the tracked code
- When tracked code changes, docs become stale (drift)

## Why Documentation Specs?

Traditional documentation has problems:

| Problem | Traditional Docs | Documentation Spec |
|---------|-----------------|-------------------|
| Stale immediately | Updated manually, if at all | Drift detection alerts |
| Unclear source | "Where did this come from?" | `tracks:` links to code |
| Hard to verify | No way to check accuracy | Re-run spec to regenerate |
| Scattered ownership | Anyone can edit | Spec owns the doc |

## Creating the Documentation Spec

Alex creates a documentation spec for architecture:

```bash
chant add "Document system architecture"
```

**File: `.chant/specs/2026-01-28-001-doc.md`**

```yaml
---
type: documentation
status: pending
prompt: documentation
depends_on:
  - 2026-01-27-001-cpl  # Coupling analysis
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

```markdown
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

- [ ] Overview with architecture diagram
- [ ] All 5 modules documented
- [ ] Data flow diagram created
- [ ] 3+ architecture decisions documented
- [ ] All docs reference specific source locations
```

## Running the Documentation Spec

Alex executes the spec:

```bash
chant work 001-doc
```

The agent:
1. Reads the coupling analysis (via `informed_by:`)
2. Reads tracked source files
3. Generates documentation that maps to the code

## Agent Output

**File: `docs/architecture/overview.md`** (generated)

```markdown
# System Architecture Overview

## Purpose

TechCorp Backend is a B2B SaaS platform providing project management,
billing, and reporting services. It handles 3.8M requests/day across
5,000 customer accounts.

## Architecture Diagram

```
                    ┌─────────────────────────────────────┐
                    │           Load Balancer             │
                    └──────────────────┬──────────────────┘
                                       │
                    ┌──────────────────┴──────────────────┐
                    │                                      │
            ┌───────┴───────┐                  ┌──────────┴──────────┐
            │   API Layer   │                  │     Web Layer       │
            │   (REST)      │                  │    (GraphQL)        │
            └───────┬───────┘                  └──────────┬──────────┘
                    │                                      │
                    └──────────────────┬───────────────────┘
                                       │
    ┌──────────┬──────────┬────────────┼────────────┬──────────────────┐
    │          │          │            │            │                  │
┌───┴───┐ ┌────┴────┐ ┌───┴────┐ ┌─────┴─────┐ ┌────┴─────┐  ┌─────────┴──────┐
│ Auth  │ │  Users  │ │Billing │ │ Reporting │ │ Notifs   │  │     Utils      │
│       │ │         │ │        │ │           │ │          │  │   (Shared)     │
└───┬───┘ └────┬────┘ └───┬────┘ └─────┬─────┘ └────┬─────┘  └────────────────┘
    │          │          │            │            │
    └──────────┴──────────┴────────────┴────────────┘
                           │
              ┌────────────┴────────────┐
              │                         │
         ┌────┴────┐              ┌─────┴─────┐
         │ Primary │              │  Redis    │
         │  (PG)   │              │  Cache    │
         └─────────┘              └───────────┘
```

## Technology Stack

| Layer | Technology |
|-------|------------|
| Language | Python 3.11 |
| Framework | FastAPI |
| Database | PostgreSQL 15 |
| Cache | Redis 7 |
| Queue | RabbitMQ |
| Container | Docker |
| Orchestration | Kubernetes |

## Deployment Topology

| Environment | Instances | Database | Purpose |
|-------------|-----------|----------|---------|
| Production | 12 pods | Primary + 2 replicas | Live traffic |
| Staging | 3 pods | Single instance | Pre-release testing |
| Development | 1 pod | Docker compose | Local development |
```

**File: `docs/architecture/modules.md`** (generated)

```markdown
# Module Documentation

## Auth Module

**Location:** `src/auth/`
**Owner:** Platform Team
**LOC:** 8,200

### Purpose

Handles authentication, authorization, and session management.

### Public Interface

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth/login` | POST | User login |
| `/auth/logout` | POST | Session termination |
| `/auth/refresh` | POST | Token refresh |
| `/auth/permissions` | GET | Get user permissions |

### Key Files

- `src/auth/routes.py:1-150` — API endpoints
- `src/auth/services.py:1-280` — Business logic
- `src/auth/models.py:1-95` — ORM models
- `src/auth/permissions.py:1-180` — Permission system

### Dependencies

- Imports from: users (3), utils (12)
- Imported by: users (5), billing (3), reporting (2), notifications (1)

---

## Users Module

**Location:** `src/users/`
**Owner:** Platform Team
**LOC:** 12,400

### Purpose

Manages user accounts, profiles, preferences, and team relationships.

### Public Interface

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/users` | GET | List users |
| `/users/{id}` | GET | Get user details |
| `/users/{id}` | PATCH | Update user |
| `/users/{id}/preferences` | GET/PUT | User preferences |
| `/users/{id}/teams` | GET | User's teams |

### Key Files

- `src/users/routes.py:1-320` — API endpoints
- `src/users/services.py:1-450` — Business logic
- `src/users/models.py:1-180` — ORM models (User, Team, Preference)

### Dependencies

- Imports from: auth (5), billing (2), notifications (1), utils (18)
- Imported by: auth (3), billing (8), reporting (6), notifications (3)

---

[... billing, reporting, notifications modules ...]
```

**File: `docs/architecture/decisions.md`** (generated)

```markdown
# Architecture Decision Records

## ADR-001: Monolithic Architecture

**Date:** 2017-03-15
**Status:** Active (but under evaluation)

### Context

When TechCorp was founded, team size was 3 engineers. A monolithic
architecture provided:
- Simpler deployment
- Easier debugging
- Faster initial development

### Decision

Build as a single deployable unit with module-based organization.

### Consequences

- Positive: Fast time-to-market in years 1-3
- Negative: Scaling and deployment coupling issues at current size

### Source Evidence

Found in `src/config/settings.py:1-15` (original architecture notes)

---

## ADR-002: Shared Utils Pattern

**Date:** 2018-06-20
**Status:** Active (identified as tech debt)

### Context

Common functionality needed across modules: logging, caching, database
connections.

### Decision

Create `src/utils/` package imported by all modules.

### Consequences

- Positive: Code reuse, consistent patterns
- Negative: 92 incoming dependencies create implicit coupling

### Source Evidence

Found in `src/utils/__init__.py:1-50` (utility exports)

### Recommendation

Consider splitting utils into:
- `utils/core` — Database, config (always needed)
- `utils/cache` — Caching (needed by some)
- `utils/logging` — Logging (standalone)

---

## ADR-003: PostgreSQL for All Data

**Date:** 2017-03-20
**Status:** Active

### Context

Needed a reliable, scalable database for all application data.

### Decision

Use PostgreSQL for all persistent data, no polyglot persistence.

### Consequences

- Positive: Single database to manage, strong consistency
- Negative: Reporting queries compete with OLTP workloads

### Source Evidence

Found in `src/config/database.py:1-30` (single connection pool)
```

## Tracking and Drift

The `tracks:` field creates a linkage. When tracked files change, documentation becomes stale:

```bash
$ chant drift 001-doc
```

```
Drift Report
============

Spec: 2026-01-28-001-doc (Document system architecture)
Status: UP TO DATE

Tracked files (152) unchanged since completion (2026-01-28)

Next check scheduled: 2026-02-04
```

Later, when a developer modifies `src/auth/routes.py`:

```bash
$ chant drift 001-doc
```

```
Drift Report
============

Spec: 2026-01-28-001-doc (Document system architecture)
Status: DRIFT DETECTED

Tracked files changed since completion (2026-01-28):
  - src/auth/routes.py
      Modified: 2026-02-15
      Lines changed: +45, -12
      New endpoint: /auth/mfa/setup

Recommendation: Re-run spec to update documentation
  $ chant reset 001-doc && chant work 001-doc
```

## Documentation Benefits

| Benefit | How It Helps |
|---------|-------------|
| **Always current** | Drift detection catches stale docs |
| **Traceable** | Every statement links to source code |
| **Reproducible** | Re-run spec to regenerate from scratch |
| **Verified** | Agent validates docs match code |

## What's Next

With architecture documented, Alex coordinates a POC extraction:

**[Implementation](04-implementation.md)** — Using driver specs to coordinate the reporting service extraction
