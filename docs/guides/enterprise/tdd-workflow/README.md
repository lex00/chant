# Enterprise TDD Workflow

A complete walkthrough showing how chant improves Test-Driven Development (TDD) across multiple teams in an enterprise environment.

## The Scenario

**Acme SaaS Corp** (the same B2B platform from the [KPI/OKR guide](../kpi-okr/README.md)) faces inconsistent testing practices across their 3 product teams. This guide follows their journey to standardize TDD using chant's spec-driven approach.

### Teams

| Team | Focus | Engineers | Test Coverage | Test Flakiness |
|------|-------|-----------|---------------|----------------|
| Auth | Authentication & SSO | 5 | 85% | 8% |
| Payments | Billing & subscriptions | 4 | 45% | 18% |
| Analytics | Reporting & dashboards | 6 | 92% | 5% |

### TDD Challenges

```
Current State                    Target State
┌─────────────────────┐          ┌─────────────────────┐
│ Inconsistent coverage│          │ 80%+ all teams     │
│ Tests after code     │    →     │ Tests first (TDD)  │
│ 12% flaky tests     │          │ <5% flaky tests    │
│ 23% test drift      │          │ <5% drift          │
└─────────────────────┘          └─────────────────────┘
```

## How Chant Helps

Chant's spec-driven model naturally aligns with TDD:

- **Acceptance criteria = test cases** — Each checkbox becomes a test
- **Specs before code** — Tests are planned before implementation
- **Research specs** — Analyze coverage gaps before writing tests
- **Drift detection** — Keep tests synchronized with behavior

## Guide Pages

1. **[The TDD Challenge](01-scenario.md)** — Acme's testing problems across 3 teams
2. **[Chant Meets TDD](02-integration.md)** — How spec-driven development aligns with TDD
3. **[Specs as Test Plans](03-test-planning.md)** — Using acceptance criteria to define tests
4. **[Writing Tests with Chant](04-execution.md)** — Agent-assisted test implementation
5. **[Ensuring Quality](05-consistency.md)** — Enforcing test standards across teams
6. **[Keeping Tests Current](06-drift-detection.md)** — Detecting and fixing test drift

## Key Concepts Demonstrated

- **Spec-first testing** — Write acceptance criteria before tests before code
- **Research specs** for test coverage analysis
- **Parallel test execution** across feature areas
- **Config validation** for test quality standards
- **Drift detection** for test maintenance

## Prerequisites

Familiarity with [core concepts](../../../concepts/specs.md) and the [KPI/OKR workflow](../kpi-okr/README.md).

## See Also

- [TDD Workflow Example](../../../../examples/tdd-workflow/) — Working example demonstrating this workflow
