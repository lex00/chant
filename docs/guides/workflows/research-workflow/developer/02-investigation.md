# Week 1: Codebase Investigation

Alex begins with systematic codebase analysis. This phase uses research specs to understand coupling, dependencies, and extraction candidates.

## The Investigation Pattern

Codebase investigation uses the same pattern as literature review — synthesizing multiple sources:

```
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ src/auth/   │   │src/billing/ │   │src/users/   │
│   *.py      │   │   *.py      │   │   *.py      │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │                 │                 │
       └────────────────┬┴─────────────────┘
                        │
                        ▼
              ┌─────────────────────┐
              │   Research Spec     │
              │  informed_by:       │
              │   - src/**/*.py     │
              └──────────┬──────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │  Coupling Analysis  │
              │  (Dependencies)     │
              └─────────────────────┘
```

The `informed_by:` field tells the agent what code to analyze. Instead of papers, it reads source files.

## Creating the Investigation Spec

Alex creates a research spec for coupling analysis:

```bash
chant add "Analyze codebase coupling for microservices evaluation" --type research
```

**File: `.chant/specs/2026-01-27-001-cpl.md`**

```yaml
---
type: research
status: pending
prompt: research-analysis
informed_by:
  - src/**/*.py
  - .chant/context/migration-research/production-metrics.md
target_files:
  - analysis/coupling/dependency-matrix.md
  - analysis/coupling/extraction-candidates.md
  - analysis/coupling/risk-assessment.md
---
```

```markdown
# Analyze codebase coupling for microservices evaluation

## Problem

TechCorp's 80K LOC Python monolith needs evaluation for microservices migration.
Before recommending extraction candidates, I need to understand:
- Which modules are tightly coupled?
- Which modules are loosely coupled and easy to extract?
- What shared state exists between modules?
- Where are the database coupling hotspots?

## Research Questions

- [ ] What are the import dependencies between modules?
- [ ] Which modules share database tables?
- [ ] What is the cyclomatic complexity distribution?
- [ ] Which modules have the most external callers?
- [ ] What shared utilities create implicit coupling?

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

- [ ] Import dependency matrix generated
- [ ] Database coupling analysis complete
- [ ] Complexity metrics calculated
- [ ] Extraction candidates ranked with rationale
- [ ] Risk assessment for top 3 candidates
- [ ] All findings written to target files
```

## Running the Investigation

Alex executes the spec:

```bash
chant work 001-cpl
```

The agent reads all Python files (via `informed_by:`), analyzes patterns, and generates findings.

## Agent Output

**File: `analysis/coupling/dependency-matrix.md`** (generated)

```markdown
# Module Dependency Matrix

## Import Dependencies

Rows: importing module, Columns: imported module

|              | auth | users | billing | reporting | notifications | utils |
|--------------|------|-------|---------|-----------|---------------|-------|
| auth         | -    | 3     | 0       | 0         | 0             | 12    |
| users        | 5    | -     | 2       | 0         | 1             | 18    |
| billing      | 3    | 8     | -       | 1         | 4             | 22    |
| reporting    | 2    | 6     | 5       | -         | 0             | 31    |
| notifications| 1    | 3     | 2       | 0         | -             | 9     |

## Coupling Metrics

| Module | Fan-In | Fan-Out | Instability | Notes |
|--------|--------|---------|-------------|-------|
| auth | 11 | 15 | 0.58 | Core dependency, moderate coupling |
| users | 20 | 26 | 0.57 | Most imported, hub module |
| billing | 6 | 40 | 0.87 | High instability, many outgoing deps |
| reporting | 0 | 44 | 1.00 | Pure consumer, no dependents |
| notifications | 3 | 15 | 0.83 | Leaf node, few dependents |
| utils | 92 | 0 | 0.00 | Shared utilities, imported everywhere |

## Circular Dependencies

Found 3 circular dependency chains:

1. `auth → users → auth` (via auth.permissions)
2. `billing → users → billing` (via billing.subscriptions)
3. `users → billing → notifications → users` (via user preferences)

## Interpretation

- **users** is a hub — everything depends on it
- **reporting** is a leaf — depends on others, nothing depends on it
- **utils** is problematic — 92 incoming dependencies creates implicit coupling
- Circular dependencies complicate extraction ordering
```

**File: `analysis/coupling/extraction-candidates.md`** (generated)

```markdown
# Extraction Candidates

## Ranking

| Rank | Module | Score | Rationale |
|------|--------|-------|-----------|
| 1 | reporting | 8.5/10 | Leaf node, no dependents, resource-heavy |
| 2 | notifications | 7.2/10 | Few dependents, clear boundaries |
| 3 | billing | 5.8/10 | Important, but coupled to users |
| 4 | auth | 4.2/10 | Core dependency, high risk |
| 5 | users | 2.1/10 | Hub module, extract last |

## Detailed Analysis

### 1. Reporting (Score: 8.5)

**Pros:**
- Zero incoming dependencies (pure consumer)
- Clear API boundary (5 endpoints)
- Distinct resource profile (heavy CPU/memory)
- Team C owns entirely

**Cons:**
- Heavy database access (28.6 queries/request)
- Uses 12 models from other modules

**Recommendation:** Extract first. Create read replicas for data access.

### 2. Notifications (Score: 7.2)

**Pros:**
- Only 3 incoming dependencies
- Async by nature (good for separate service)
- Clear business boundary

**Cons:**
- Depends on user preferences from users module
- Circular dependency with users

**Recommendation:** Extract second after resolving user preference coupling.

### 3. Billing (Score: 5.8)

**Pros:**
- Important business domain
- Would benefit from isolated scaling

**Cons:**
- 8 dependencies on users module
- Circular dependency chain
- Highest error rate (0.12%)

**Recommendation:** Extract third. Requires event-driven decoupling.

## Extraction Order

```
Phase 1: reporting     (standalone)
Phase 2: notifications (after preference refactor)
Phase 3: billing       (after event bus setup)
Phase 4: auth          (after users interface stabilizes)
Phase 5: users         (becomes core service)
```
```

**File: `analysis/coupling/risk-assessment.md`** (generated)

```markdown
# Risk Assessment

## Top 3 Candidates

### 1. Reporting Extraction

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Data consistency | Medium | High | Read replicas with eventual consistency |
| Query performance | High | Medium | Dedicated database with denormalized views |
| Team coordination | Low | Low | Single team ownership |

**Overall Risk: MEDIUM**

### 2. Notifications Extraction

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Preference coupling | High | Medium | Extract preferences as shared service |
| Message ordering | Medium | High | Use ordered message queue |
| Delivery guarantees | Medium | High | Implement idempotency |

**Overall Risk: MEDIUM-HIGH**

### 3. Billing Extraction

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Transaction consistency | High | Critical | Saga pattern for distributed transactions |
| User data access | High | High | Event-driven sync |
| Revenue impact | Medium | Critical | Extensive parallel running |

**Overall Risk: HIGH**

## Recommendations

1. Start with reporting (lowest risk, highest standalone value)
2. Build event infrastructure before billing extraction
3. Consider extracting user preferences as shared service first
4. Plan 6-month timeline for first two extractions
```

## Verifying the Investigation

Alex reviews the completed spec:

```bash
chant show 001-cpl
```

```
Spec: 2026-01-27-001-cpl
Type: research
Status: completed

Informed by: src/**/*.py (342 files)

Research Questions:
  [x] What are the import dependencies between modules?
  [x] Which modules share database tables?
  [x] What is the cyclomatic complexity distribution?
  [x] Which modules have the most external callers?
  [x] What shared utilities create implicit coupling?

Acceptance Criteria:
  [x] Import dependency matrix generated
  [x] Database coupling analysis complete
  [x] Complexity metrics calculated
  [x] Extraction candidates ranked with rationale
  [x] Risk assessment for top 3 candidates
  [x] All findings written to target files
```

## The Provenance Trail

The spec records exactly what was analyzed:
- 342 Python files read
- Production metrics incorporated
- Clear methodology documented

When leadership asks "How did you conclude reporting should be first?", Alex points to the spec and its analysis.

## What's Next

With investigation complete, Alex documents the architecture:

**[Documentation](03-documentation.md)** — Creating architecture documentation that tracks source code with `tracks:`
