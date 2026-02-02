# The TDD Challenge

## Acme SaaS Corp — Testing Reality

Acme SaaS Corp runs the same B2B project management platform from the [KPI/OKR guide](../kpi-okr/README.md). After successfully reducing churn from 8% to 5%, engineering leadership turns to a persistent problem: inconsistent testing practices.

| Metric | Current | Target |
|--------|---------|--------|
| Average test coverage | 74% | 80%+ |
| Flaky test rate | 12% | <5% |
| Test drift | 23% | <5% |
| Tests written before code | 35% | 90%+ |

## Three Teams, Three Approaches

### Auth Team (5 engineers)

The Auth team follows TDD religiously. Every feature starts with failing tests.

| Metric | Value | Assessment |
|--------|-------|------------|
| Coverage | 85% | Strong |
| Flaky tests | 8% | Needs work |
| Test drift | 12% | Moderate |
| TDD adoption | 95% | Excellent |

**Pain point:** Flaky tests in OAuth integration tests. External service mocks don't match production behavior.

### Payments Team (4 engineers)

The Payments team writes tests after implementation. High bug rate in production.

| Metric | Value | Assessment |
|--------|-------|------------|
| Coverage | 45% | Critical |
| Flaky tests | 18% | Critical |
| Test drift | 35% | Critical |
| TDD adoption | 15% | Poor |

**Pain point:** Tests are an afterthought. Engineers say "no time for tests" under delivery pressure.

### Analytics Team (6 engineers)

The Analytics team has the highest coverage but struggles with test maintenance.

| Metric | Value | Assessment |
|--------|-------|------------|
| Coverage | 92% | Excellent |
| Flaky tests | 5% | Good |
| Test drift | 28% | Needs work |
| TDD adoption | 40% | Moderate |

**Pain point:** Tests don't match current behavior. The codebase evolved, tests didn't.

## The Root Causes

Engineering leadership identifies five systemic issues:

### 1. Inconsistent Test Planning

No standard process for deciding what to test. Each engineer makes ad-hoc decisions.

```
Engineer A: "I'll test the happy path"
Engineer B: "I'll test all edge cases"
Engineer C: "I'll test what seems important"
```

### 2. Tests Written Too Late

When tests come after code, they test implementation rather than behavior. They become brittle and don't catch real bugs.

```
Traditional Flow (common at Acme):
  Code → Tests → Review → Ship

TDD Flow (goal):
  Spec → Tests → Code → Review → Ship
```

### 3. No Test Quality Gates

No validation that tests meet standards. Missing assertions pass CI. Low-value tests inflate coverage numbers.

### 4. Test Drift

Tests describe behavior that no longer exists. 23% of tests are out of sync with current code behavior.

### 5. Slow Test Authoring

Writing tests is seen as slow. Engineers estimate 30-40% of implementation time goes to test writing.

## Q1 OKR: TDD Transformation

**Objective:** Establish consistent TDD practices across all teams

**Key Results:**
- All teams at 80%+ coverage by end of Q1
- Flaky test rate below 5%
- Test drift below 5%
- 90%+ of new features start with test specs

## Team Structure

The transformation follows chant's orchestrator pattern:

- **Marcus** (Engineering Manager) creates standards, reviews test specs
- **Team leads** review and approve test planning specs
- **Chant agents** analyze coverage gaps and write tests
- **CI/CD** runs test quality reports daily

## Project Setup

```bash
# Initialize chant with test-focused configuration
chant init --agent claude

# Create context directory for test standards
mkdir -p .chant/context/tdd-standards
```

The `.chant/context/tdd-standards/` directory holds team test guidelines that agents reference when writing tests.

## What's Next

With the problem defined, Marcus explores how chant's spec model aligns with TDD:

1. **[Chant Meets TDD](02-integration.md)** — How specs naturally support test-first development
2. **[Test Planning](03-test-planning.md)** — Using acceptance criteria as test cases
3. **[Execution](04-execution.md)** — Agents writing tests before implementation
4. **[Consistency](05-consistency.md)** — Enforcing standards across teams
