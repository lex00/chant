# TDD Workflow Configuration

This configuration enforces TDD standards for the Payments team.

## spec_defaults

### code

Required fields for all code specs:
- `target_files`: required
- `test_coverage_target`: 80

### required_labels

Team identifier (one required):
- payments
- auth
- analytics

Test level (one required):
- unit
- integration
- e2e

## validation

### coverage

Global coverage thresholds:
- minimum: 80%
- warn_below: 85%

Module-specific overrides:
- **payments/***: 85% minimum (critical financial operations)
- **auth/***: 80% minimum (security-sensitive code)

### test_quality

Test quality requirements:
- min_assertions_per_test: 2
- require_docstrings: true
- naming_pattern: `test_<action>_<condition>_<expected_result>`

### flakiness

Flaky test thresholds:
- max_flaky_rate: 5%
- flakiness_runs: 10
- fail_on_flaky: true

## templates

### test-spec

Default template for test specs:

```yaml
---
type: code
status: pending
labels:
  - {{team}}
  - {{test_level}}
target_files:
  - tests/{{module}}/test_{{feature}}.py
---

# Add tests for {{feature}}

## Problem

Brief description of what needs testing and why.

## Test Categories

### Happy Path
- [ ] Test case 1
- [ ] Test case 2

### Edge Cases
- [ ] Edge case 1
- [ ] Edge case 2

### Error Handling
- [ ] Error case 1
- [ ] Error case 2

## Acceptance Criteria

- [ ] All test cases passing
- [ ] Coverage target met (80%+)
- [ ] No flaky tests (verified with 10 runs)
- [ ] Tests follow naming convention
```

## context_files

Standard context files for TDD:

- `.chant/context/tdd-standards/coverage-requirements.md`
- `.chant/context/tdd-standards/test-patterns.md`
