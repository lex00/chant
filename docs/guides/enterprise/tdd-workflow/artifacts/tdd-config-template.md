# TDD Configuration Template

This file shows a sample `.chant/config.md` configuration for enforcing
TDD standards across teams.

## spec_defaults

### code

Required fields for all code specs:
- target_files: required
- test_coverage_target: 80

### required_labels

Team identifier (one required):
- auth
- payments
- analytics

Test level (one required):
- unit
- integration
- e2e

## validation

### coverage

Global coverage thresholds:
- minimum: 80
- warn_below: 85

Module-specific overrides:
- path: "payments/*"
  minimum: 85
  rationale: "Critical financial operations"

- path: "auth/*"
  minimum: 80
  rationale: "Security-sensitive code"

- path: "analytics/*"
  minimum: 75
  rationale: "Reporting, lower risk"

### test_quality

Test quality requirements:
- min_assertions_per_test: 2
- max_test_file_length: 500
- require_docstrings: true
- naming_pattern: "test_[a-z_]+_[a-z_]+"

### flakiness

Flaky test thresholds:
- max_flaky_rate: 5
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

- `.chant/context/tdd-standards/naming-conventions.md`
- `.chant/context/tdd-standards/fixture-patterns.md`
- `.chant/context/tdd-standards/assertion-guidelines.md`
- `.chant/context/tdd-standards/team-overrides.md`
