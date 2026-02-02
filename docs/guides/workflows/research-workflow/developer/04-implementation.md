# Week 3: Implementation

Based on the investigation and documentation, Alex coordinates a proof-of-concept extraction of the reporting service. This phase uses driver specs to coordinate multiple implementation steps.

## The Implementation Pattern

Service extraction requires coordinated changes:

```
┌─────────────────────────────────────────────────────────────────┐
│                     Driver Spec                                  │
│              Reporting Service Extraction                        │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
   ┌─────────┐          ┌─────────┐          ┌─────────┐
   │  API    │          │Database │          │  Tests  │
   │ Extract │          │ Migrate │          │  Suite  │
   │  (.1)   │          │  (.2)   │          │  (.3)   │
   └────┬────┘          └────┬────┘          └────┬────┘
        │                     │                     │
        └─────────────────────┴─────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  Integration    │
                    │   Testing       │
                    │    (.4)         │
                    └─────────────────┘
```

## Creating the Driver Spec

Alex creates a driver spec for the extraction:

```bash
chant add "Extract reporting service POC" --type driver
```

**File: `.chant/specs/2026-02-01-001-ext.md`**

```yaml
---
type: driver
status: pending
prompt: driver
depends_on:
  - 2026-01-27-001-cpl  # Coupling analysis
  - 2026-01-28-001-doc  # Architecture docs
informed_by:
  - analysis/coupling/extraction-candidates.md
  - analysis/coupling/risk-assessment.md
members:
  - 2026-02-01-001-ext.1  # API extraction
  - 2026-02-01-001-ext.2  # Database migration
  - 2026-02-01-001-ext.3  # Test suite
  - 2026-02-01-001-ext.4  # Integration testing
target_files:
  - services/reporting/README.md
---
```

```markdown
# Extract reporting service POC

## Problem

The reporting module is the best extraction candidate (score: 8.5/10):
- Zero incoming dependencies
- Clear API boundary
- Distinct resource profile
- Single team ownership

This POC extracts it as a standalone service to validate the approach.

## Extraction Steps

### Step 1: API Extraction (.1)
- Create new service repository structure
- Extract reporting endpoints
- Implement API gateway routing
- Output: Standalone reporting service

### Step 2: Database Migration (.2)
- Create read replica for reporting
- Set up denormalized views
- Configure connection pooling
- Output: Isolated data access

### Step 3: Test Suite (.3)
- Migrate existing reporting tests
- Add integration tests
- Create contract tests for API
- Output: Comprehensive test coverage

### Step 4: Integration Testing (.4)
- Test with monolith in parallel
- Verify data consistency
- Load test the new service
- Output: Validation report

## Execution Order

```
.1 (API) ──────────┐
                   │
.2 (Database) ─────┼───> .4 (Integration)
                   │
.3 (Tests) ────────┘
```

.1, .2, .3 can run in parallel; .4 depends on all three.

## Acceptance Criteria

- [ ] All four member specs created
- [ ] Standalone service deployable
- [ ] All existing tests passing
- [ ] Integration tests passing
- [ ] Performance meets baseline
```

## Member Specs

### Member .1: API Extraction

**File: `.chant/specs/2026-02-01-001-ext.1.md`**

```yaml
---
type: code
status: pending
parent: 2026-02-01-001-ext
informed_by:
  - src/reporting/**/*.py
  - docs/architecture/modules.md
target_files:
  - services/reporting/src/main.py
  - services/reporting/src/routes.py
  - services/reporting/src/models.py
  - services/reporting/Dockerfile
  - services/reporting/pyproject.toml
---
```

```markdown
# API extraction

## Problem

Extract reporting endpoints from monolith to standalone service.

## Tasks

- [ ] Create service structure
- [ ] Extract route handlers from `src/reporting/routes.py`
- [ ] Extract models (read-only copies)
- [ ] Configure FastAPI application
- [ ] Create Dockerfile for deployment
- [ ] Set up pyproject.toml with dependencies

## Acceptance Criteria

- [ ] Service starts and responds to health check
- [ ] All 5 reporting endpoints functional
- [ ] Same API contract as monolith
- [ ] Docker image builds successfully
```

### Member .2: Database Migration

**File: `.chant/specs/2026-02-01-001-ext.2.md`**

```yaml
---
type: code
status: pending
parent: 2026-02-01-001-ext
informed_by:
  - src/reporting/models.py
  - analysis/coupling/dependency-matrix.md
target_files:
  - services/reporting/src/database.py
  - services/reporting/migrations/001_initial.py
  - infrastructure/reporting-replica.tf
---
```

```markdown
# Database migration

## Problem

Reporting needs isolated data access without affecting monolith.

## Tasks

- [ ] Create read replica configuration
- [ ] Set up denormalized views for reporting queries
- [ ] Configure connection pooling (separate from monolith)
- [ ] Create Terraform for replica infrastructure

## Acceptance Criteria

- [ ] Read replica provisioned
- [ ] Reporting queries use replica (not primary)
- [ ] Query performance equal or better than baseline
- [ ] Zero impact on monolith database
```

### Member .3: Test Suite

**File: `.chant/specs/2026-02-01-001-ext.3.md`**

```yaml
---
type: code
status: pending
parent: 2026-02-01-001-ext
informed_by:
  - tests/reporting/**/*.py
target_files:
  - services/reporting/tests/test_routes.py
  - services/reporting/tests/test_models.py
  - services/reporting/tests/conftest.py
  - services/reporting/tests/contracts/reporting_api.yaml
---
```

```markdown
# Test suite

## Problem

Extracted service needs comprehensive tests.

## Tasks

- [ ] Migrate unit tests from `tests/reporting/`
- [ ] Create integration tests for new service
- [ ] Define contract tests (Pact-style)
- [ ] Set up test fixtures and conftest

## Acceptance Criteria

- [ ] All existing tests pass in new service
- [ ] Coverage >= 80%
- [ ] Contract tests verify API compatibility
- [ ] CI/CD pipeline runs tests
```

### Member .4: Integration Testing

**File: `.chant/specs/2026-02-01-001-ext.4.md`**

```yaml
---
type: task
status: pending
parent: 2026-02-01-001-ext
depends_on:
  - 2026-02-01-001-ext.1
  - 2026-02-01-001-ext.2
  - 2026-02-01-001-ext.3
target_files:
  - analysis/extraction/validation-report.md
---
```

```markdown
# Integration testing

## Problem

Validate extracted service works correctly with rest of system.

## Tasks

- [ ] Deploy service alongside monolith
- [ ] Route 10% of reporting traffic to new service
- [ ] Compare response accuracy
- [ ] Measure latency and throughput
- [ ] Document findings

## Acceptance Criteria

- [ ] 100% response accuracy (vs monolith)
- [ ] P99 latency within 10% of baseline
- [ ] No errors under normal load
- [ ] Validation report complete
```

## Running the Extraction

Alex executes the driver:

```bash
chant work 001-ext
```

```
Working: 2026-02-01-001-ext (Extract reporting service POC)

Phase 1: Parallel
  [✓] 2026-02-01-001-ext.1 (API extraction) - 8m 22s
  [✓] 2026-02-01-001-ext.2 (Database migration) - 12m 45s
  [✓] 2026-02-01-001-ext.3 (Test suite) - 6m 18s

Phase 2: Sequential (depends on all above)
  [✓] 2026-02-01-001-ext.4 (Integration testing) - 18m 30s

Driver complete. All 4 members succeeded.
```

## Validation Report

**File: `analysis/extraction/validation-report.md`** (generated)

```markdown
# Reporting Service Extraction - Validation Report

## Summary

| Metric | Baseline (Monolith) | Extracted Service | Status |
|--------|--------------------|--------------------|--------|
| P99 Latency | 2,400ms | 1,890ms | ✓ 21% improvement |
| Throughput | 45K req/day | 52K req/day capacity | ✓ Headroom |
| Error Rate | 0.08% | 0.06% | ✓ Slight improvement |
| Response Accuracy | — | 100% | ✓ All responses match |

## Test Results

| Test Suite | Pass | Fail | Skip |
|------------|------|------|------|
| Unit tests | 147 | 0 | 3 |
| Integration tests | 28 | 0 | 0 |
| Contract tests | 12 | 0 | 0 |
| Load tests | 5 | 0 | 0 |

Total: 192 passed, 0 failed, 3 skipped

## Performance Comparison

### P99 Latency by Endpoint

| Endpoint | Monolith | Extracted | Change |
|----------|----------|-----------|--------|
| /reports/daily | 1,850ms | 1,420ms | -23% |
| /reports/weekly | 2,400ms | 1,890ms | -21% |
| /reports/custom | 3,200ms | 2,650ms | -17% |
| /reports/export | 4,100ms | 3,200ms | -22% |
| /reports/schedule | 280ms | 245ms | -12% |

### Resource Usage

| Resource | Monolith (reporting) | Extracted | Change |
|----------|---------------------|-----------|--------|
| CPU | 78% | 65% | -17% |
| Memory | 52% | 48% | -8% |
| DB Queries | 28.6/req | 24.2/req | -15% |

## Parallel Running

Traffic split: 90% monolith, 10% extracted service

| Hour | Monolith Requests | Extracted Requests | Discrepancies |
|------|-------------------|-------------------|---------------|
| 00:00 | 1,890 | 210 | 0 |
| 01:00 | 1,620 | 180 | 0 |
| ... | ... | ... | 0 |
| 23:00 | 2,340 | 260 | 0 |

Total: 0 discrepancies in 24-hour parallel run

## Conclusion

Extraction successful. The POC validates:
1. Reporting can run as isolated service
2. Performance improves with dedicated resources
3. Zero data consistency issues
4. Test coverage is sufficient

### Recommendation

Proceed with production migration using gradual traffic shift:
- Week 1: 10% → 25%
- Week 2: 25% → 50%
- Week 3: 50% → 75%
- Week 4: 75% → 100%
```

## Code Structure Created

The extraction created a new service structure:

```
services/
└── reporting/
    ├── src/
    │   ├── main.py           # FastAPI application
    │   ├── routes.py         # Extracted endpoints
    │   ├── models.py         # Read-only model copies
    │   └── database.py       # Replica connection
    ├── tests/
    │   ├── test_routes.py    # Unit tests
    │   ├── test_models.py    # Model tests
    │   ├── conftest.py       # Fixtures
    │   └── contracts/
    │       └── reporting_api.yaml
    ├── migrations/
    │   └── 001_initial.py    # Schema setup
    ├── Dockerfile
    ├── pyproject.toml
    └── README.md
```

## What's Next

With the POC complete, Alex sets up ongoing maintenance:

**[Maintenance](05-maintenance.md)** — Drift detection on documentation and metrics as code evolves
