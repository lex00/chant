# TDD Workflow Example

## Overview

This example demonstrates Chant's Test-Driven Development workflow for systematically improving test coverage. The Payments team at Acme SaaS Corp increases refund module coverage from 38% to 85%+ using research-driven test planning and parallel test implementation through a driver spec pattern.

## Structure

The workflow consists of two main specs:

1. **001-coverage-analysis.md** - Research spec that analyzes existing test coverage gaps and prioritizes work based on production incidents
2. **002-test-suite-driver.md** - Driver spec coordinating three parallel test implementations:
   - **002-test-suite-driver.1.md** - Refund flow tests (16 test cases)
   - **002-test-suite-driver.2.md** - Currency conversion tests (4 test cases)
   - **002-test-suite-driver.3.md** - Retry logic tests (4 test cases)

Context files in `.chant/context/tdd-standards/`:
- `coverage-requirements.md` - Team coverage policies by module
- `test-patterns.md` - Naming conventions and assertion requirements

## Usage

Execute the TDD workflow with the driver pattern:
```bash
cd examples/tdd-workflow
chant work 001  # Run coverage analysis first
chant work 002  # Run driver spec (coordinates members 1-3)
```

Or work member specs independently in parallel:
```bash
chant work 002.1  # Refund flow tests
chant work 002.2  # Currency conversion tests
chant work 002.3  # Retry logic tests
```

Create your own TDD workflow:
```bash
mkdir -p .chant/context/tdd-standards
# Add coverage-requirements.md and test-patterns.md
chant add "Analyze test coverage gaps in payments module" --type research
chant add "Add comprehensive tests for payment refund flow"
chant work <spec-id>
```

## Testing

Verify the TDD workflow:
1. Review team test standards in `.chant/context/tdd-standards/`
2. Examine research spec `001-coverage-analysis.md` with `informed_by:` and `target_files:` usage
3. Check driver spec `002-test-suite-driver.md` showing acceptance criteria as test cases
4. Explore member specs organized by test category (Happy Path, Edge Cases, Error Handling)
5. Run generated tests to verify coverage improvement

Key patterns demonstrated:
- Research specs analyze coverage gaps before writing code
- Context files store team standards that agents reference
- Acceptance criteria become test cases with specific assertions
- Driver specs coordinate parallel test implementation

## See Also

- [Enterprise TDD Workflow Guide](../../docs/guides/enterprise/tdd-workflow/README.md) — Complete TDD workflow walkthrough
