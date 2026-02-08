# TDD Workflow: Test-First Development with Specs

Command examples and output are illustrative — your exact output will differ.

## Scenario

You are part of the Payments team at Acme SaaS Corp. The team has four engineers, 45% test coverage, and an 18% flaky test rate. Production incidents in Q4 revealed untested edge cases in the refund flow — partial refunds, currency conversion, and fraud holds all caused customer-facing errors. Engineering leadership has set a Q1 target: 80%+ coverage across all teams, under 5% flaky tests, and test drift below 5%.

Marcus, the engineering manager, decides to use chant's spec-driven model to bring structure to the effort. The approach: write acceptance criteria first, treat them as the test plan, then let agents execute.

## Analyzing the Gap

Before writing any tests, the team needs to understand what is missing. Marcus creates a research spec to analyze the payment service's coverage gaps:

```bash
$ chant add "Analyze payment service test coverage gaps"
Created spec: 2026-01-18-001-cov
```

He edits the spec to define the research questions — which modules have less than 50% coverage, which error paths are untested, which edge cases show up in production logs but have no tests, and how coverage breaks down by risk level:

```bash
$ chant edit 001-cov
```

The spec targets a context file where the agent will write its findings. This is how research specs work: they produce documentation, not code. Marcus kicks off the analysis:

```bash
$ chant work 001-cov
Working 001-cov: Analyze payment service test coverage gaps
...
Completed in 3m 20s
```

The agent parses the existing coverage report, cross-references it with production error logs from the last 30 days, and produces a prioritized gap analysis. The key finding: `refund.py` sits at 38% coverage with four production incidents in the last two months. Transaction processing is better at 72%, but error handling paths across the service are at just 18%.

The analysis recommends 19 new tests across four areas — partial refund flow, currency conversion, fraud hold scenarios, and retry logic — estimating about 5.5 hours of work.

## Structuring the Test Plan

With the coverage analysis in hand, Marcus creates a spec for the highest-priority gap — the refund flow:

```bash
$ chant add "Add comprehensive tests for payment refund flow"
Created spec: 2026-01-20-001-rfn
```

He opens the spec and writes acceptance criteria organized by test category. Each criterion is a specific, testable assertion — not "refund flow works correctly" but "Refund exceeds original transaction amount (reject)" and "Payment processor timeout triggers retry with backoff." By the time he is done, the spec has 16 test cases across four categories (happy path, authorization, edge cases, error handling) plus four acceptance criteria covering implementation completeness, fixture setup, coverage targets, and flakiness verification.

This is the core insight of spec-driven TDD: the acceptance criteria are the test plan. Each checkbox becomes a test function. The spec makes it impossible to ship without tests, because the tests are the first thing the criteria demand.

Marcus then creates specs for the other three gap areas identified in the research — currency conversion (4 tests), fraud handling (3 tests), and retry logic (4 tests). He ties them together with a driver spec:

```bash
$ chant add "Payment service test coverage expansion"
Created spec: 2026-01-20-005-drv
```

After editing it into a driver with four members:

```bash
$ chant dag

005-drv (driver): Payment service test coverage expansion
  001-rfn  Add comprehensive tests for payment refund flow
  002-cur  Add currency conversion tests
  003-frd  Add fraud handling tests
  004-rty  Add retry logic tests
```

All four member specs are independent — no dependencies between them — so they can all run in parallel.

## Enforcing Standards

Before executing, Marcus sets up guardrails. The three teams at Acme each had different testing practices: the Auth team used `test_*` naming with factories, the Payments team used `it_should_*` with raw data and one or two assertions per test, and the Analytics team used yet another pattern. Different conventions made tests harder to review and maintain across teams.

Marcus adds test standards to context files that agents will reference. The naming convention is `test_<action>_<condition>_<expected_result>`. Fixtures must use factories, not raw data. External services get mocked at the client boundary. Every test needs a docstring and at least two assertions.

He also runs lint to catch incomplete specs before execution:

```bash
$ chant lint

Linting 8 specs...

WARN  2026-01-20-006-xyz: Missing required label (team identifier)
ERROR 2026-01-20-007-abc: No acceptance criteria found

1 error, 1 warning
```

Specs with errors cannot be executed. The engineer who wrote `007-abc` adds acceptance criteria before it can proceed.

## Executing the Test Specs

With standards in place, Marcus runs all four test specs in parallel:

```bash
$ chant work --parallel 001-rfn 002-cur 003-frd 004-rty

[001-rfn] Starting: Add comprehensive tests for payment refund flow
[002-cur] Starting: Add currency conversion tests
[003-frd] Starting: Add fraud handling tests
[004-rty] Starting: Add retry logic tests

[003-frd] Completed (3 tests added)              1m 10s
[004-rty] Completed (4 tests added)              1m 25s
[002-cur] Completed (4 tests added)              1m 40s
[001-rfn] Completed (16 tests added)             3m 50s

All 4 specs completed. 27 tests added.
```

Each agent works in its own git worktree — no conflicts between them. Each reads the spec's acceptance criteria, reads the context files for test standards, examines the existing code interfaces, then writes tests matching every criterion.

Marcus monitors progress by tailing the log of the largest spec:

```bash
$ chant log 001-rfn

[10:02] Reading spec 2026-01-20-001-rfn...
[10:02] Reading context: tdd-standards/test-patterns.md
[10:03] Analyzing refund.py interface...
[10:04] Creating test file: tests/payments/test_refund_flow.py
[10:05] Writing test: test_full_refund_on_completed_transaction
[10:05] Writing test: test_partial_refund_correct_remaining_balance
...
[10:12] Running tests... 16 passed, 0 failed
[10:12] Checking coverage... refund.py: 87% (target: 85%)
[10:13] Running flakiness check (10 iterations)...
[10:15] All iterations passed. No flaky tests detected.
[10:15] Marking acceptance criteria complete.
```

The agent produced test files organized by category — `TestRefundHappyPath`, `TestRefundAuthorization`, `TestRefundEdgeCases`, `TestRefundErrorHandling` — each following the naming conventions from the context files. Assertions check return values, state changes, and side effects. Fixtures use factories. External service calls are mocked at the client boundary.

After all four specs complete, Marcus merges the changes:

```bash
$ chant merge --all-completed --rebase --auto

Merging 4 completed specs...

  001-rfn (refund tests):    Merged
  002-cur (currency tests):  Merged
  003-frd (fraud tests):     Merged
  004-rty (retry tests):     Merged

All specs merged to main.
```

The driver spec auto-completes once all its members finish:

```bash
$ chant show 005-drv

ID:     2026-01-20-005-drv
Type:   driver
Status: completed
Title:  Payment service test coverage expansion
```

## Measuring the Results

The numbers tell the story:

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Payment service coverage | 45% | 86% | 85% |
| Refund module coverage | 38% | 87% | 85% |
| Flaky test rate | 18% | 4% | <5% |
| Test count | 42 | 69 | -- |

All three targets met in one coordinated push. The research spec identified the gaps, the test planning specs defined exactly what to test, the agents executed in parallel, and the standards ensured consistency.

## Detecting Drift

Three weeks later, the refund service changes. A new `reason` parameter is added to `process_refund()`, the authorization thresholds are updated ($100 to $150 for auto-approval, $1000 to $1500 for manager approval), and a new `REFUND_ALREADY_PROCESSED` error code is added for idempotency checks.

The existing tests still pass — they never tested the new parameter or the new error code, and they now assert the old thresholds. This is test drift: the tests describe behavior that no longer matches reality.

Marcus runs drift detection:

```bash
$ chant drift

Drift Detection Report

Checked 12 completed specs...

001-rfn: 3 drift issues detected
  1. NEW PARAMETER: 'reason' added to process_refund()
     No tests verify reason parameter
  2. BEHAVIOR CHANGE: Authorization thresholds updated
     Tests still assert old thresholds ($100/$1000)
  3. NEW ERROR CODE: REFUND_ALREADY_PROCESSED
     No test coverage for idempotency check

002-cur: No drift detected
003-frd: No drift detected
004-rty: No drift detected

Summary: 3 drift issues in 1 spec
```

Drift detection compares the current code against what the specs describe. It catches gaps that passing tests miss — the tests pass because they never exercise the changed behavior, not because the behavior is correct.

## Fixing Drift

Marcus creates a follow-up spec to address the drift, linking it to the original for traceability:

```bash
$ chant add "Fix test drift in refund module"
Created spec: 2026-02-10-001-fix

$ chant edit 001-fix
```

The spec lists three concrete criteria: add a test for the `reason` parameter, update the authorization threshold assertions, and add a test for the `REFUND_ALREADY_PROCESSED` error code. It also adds `depends_on` linking back to the original refund spec and carries the `drift-fix` label.

```bash
$ chant work 001-fix
Working 001-fix: Fix test drift in refund module
...
Completed in 1m 30s
```

After merging, Marcus confirms the drift is resolved:

```bash
$ chant drift

Drift Detection Report

Checked 12 completed specs...
No drift detected.

All tests aligned with current behavior.
```

The cycle closes. The original specs still describe what was built. The follow-up spec documents why the tests changed and links back to the drift that prompted it. Any engineer can trace from a test function back to its spec, and from a spec to the business requirement that drove it.

## Summary

| Concept | What it does |
|---------|-------------|
| **Research specs** | Analyze coverage gaps before writing tests |
| **Acceptance criteria as tests** | Each checkbox becomes a test function |
| **Driver specs** | Coordinate multiple test specs under one initiative |
| **Context files** | Store team test standards that agents reference |
| **Lint** | Catches incomplete or malformed specs before execution |
| **Parallel execution** | Independent test specs run simultaneously in worktrees |
| **Drift detection** | Catches when code diverges from what tests describe |
| **Follow-up specs** | `depends_on` links drift fixes to what prompted them |

## Reference Implementation

**[Reference artifacts](workflows/tdd-workflow/artifacts/)** accompany this guide with concrete examples:

- **Spec files** — Pre-built specs showing what each phase produces:
  - `coverage-research-spec-001-cov.md` — Research spec for coverage gap analysis
  - `test-planning-spec-001-rfn.md` — Test planning spec with 16 test cases
  - `test-suite-driver-spec.md` — Driver coordinating four test specs
  - `tdd-config-template.md` — Configuration for enforcing test standards
- **Generated code** — Agent output from test execution:
  - `generated-test-file.py` — Complete test file with four test classes

See the artifacts directory for full file contents.

## Further Reading

- [Lifecycle Walkthrough](lifecycle-walkthrough.md) — The full spec lifecycle from creation to drift
- [Lifecycle](../concepts/lifecycle.md) — State machine and transitions
- [CLI Reference](../reference/cli.md) — Full command documentation
