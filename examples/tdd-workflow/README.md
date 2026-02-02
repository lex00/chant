# TDD Workflow Example

This example demonstrates Chant's Test-Driven Development workflow, showing how specs naturally support test-first development.

## Scenario

The Payments team at Acme SaaS Corp needs to improve test coverage for their refund processing flow. Current coverage is 38%, and production incidents revealed untested edge cases.

**Goal:** Increase refund module coverage from 38% to 85%+ using systematic TDD practices.

## TDD Workflow Phases

### 1. Research: Coverage Analysis

**Spec:** `001-coverage-analysis.md`

A research spec analyzes existing test coverage gaps, identifies untested code paths, and prioritizes work based on production incidents.

Key features:
- Uses `informed_by:` to reference team test standards
- Produces `target_files:` with coverage analysis results
- Cross-references production logs to find real gaps

### 2. Planning: Test Suite Design

**Spec:** `002-test-suite-driver.md`

A driver spec coordinates multiple test implementation specs, organizing work by test category.

Key features:
- Driver with 3 member specs for parallel execution
- Each member spec focuses on specific test categories
- Acceptance criteria written as test cases

### 3. Implementation: Write Tests

**Member Specs:**
- `002-test-suite-driver.1.md` - Refund flow tests
- `002-test-suite-driver.2.md` - Currency conversion tests
- `002-test-suite-driver.3.md` - Retry logic tests

Each spec includes:
- Test categories (Happy Path, Authorization, Edge Cases, Error Handling)
- Specific test assertions as acceptance criteria
- Target test files

## Project Structure

```
examples/tdd-workflow/
├── README.md                               # This file
├── .chant/
│   ├── config.md                           # TDD standards enforcement
│   ├── context/
│   │   └── tdd-standards/
│   │       ├── coverage-requirements.md    # Team coverage policies
│   │       └── test-patterns.md            # Naming conventions
│   ├── specs/
│   │   ├── 001-coverage-analysis.md        # Research spec
│   │   ├── 002-test-suite-driver.md        # Driver spec
│   │   ├── 002-test-suite-driver.1.md      # Member: refund tests
│   │   ├── 002-test-suite-driver.2.md      # Member: currency tests
│   │   └── 002-test-suite-driver.3.md      # Member: retry tests
│   └── prompts/
│       └── standard.md                     # Default prompt
├── src/
│   └── payments/
│       └── refund.py                       # Sample code to test
└── tests/
    └── payments/
        └── test_refund_flow.py             # Generated tests
```

## What This Demonstrates

### Context Standards

`.chant/context/tdd-standards/` contains team test standards:
- **coverage-requirements.md** - Coverage targets by module
- **test-patterns.md** - Naming conventions and assertion requirements

Agents reference these standards when writing tests, ensuring consistency.

### Research Specs

The coverage analysis spec (`001-coverage-analysis.md`) uses:
- `type: research` - Indicates investigation, not code changes
- `informed_by:` - References context files with team standards
- `target_files:` - Specifies where to save analysis results

### Test Planning

The test suite driver (`002-test-suite-driver.md`) shows:
- Acceptance criteria as test cases
- Test organization by category (Happy Path, Edge Cases, etc.)
- Specific assertions in each criterion

### Driver + Members

The driver spec coordinates 3 member specs:
- Member 1: Refund flow tests (16 test cases)
- Member 2: Currency conversion tests (4 test cases)
- Member 3: Retry logic tests (4 test cases)

All member specs can run in parallel, speeding up test implementation.

## Running This Example

This is a standalone demonstration. To use this pattern in your project:

1. **Create test standards:**
   ```bash
   mkdir -p .chant/context/tdd-standards
   # Add coverage-requirements.md and test-patterns.md
   ```

2. **Analyze coverage:**
   ```bash
   chant add "Analyze test coverage gaps in payments module" --type research
   ```

3. **Plan tests:**
   ```bash
   chant add "Add comprehensive tests for payment refund flow"
   ```

4. **Execute:**
   ```bash
   chant work <spec-id>
   ```

## Key Takeaways

1. **Research specs** analyze coverage gaps before writing code
2. **Context files** store team standards that agents reference
3. **Acceptance criteria** become test cases when written as specific assertions
4. **Driver specs** coordinate parallel test implementation across categories
5. **Test-first** approach ensures tests describe behavior, not implementation

## Based On

This example is extracted from the guide: `docs/guides/enterprise/tdd-workflow/`

For detailed explanations of each phase, see:
- [01-scenario.md](../../docs/guides/enterprise/tdd-workflow/01-scenario.md) - The business context
- [03-test-planning.md](../../docs/guides/enterprise/tdd-workflow/03-test-planning.md) - Using specs as test plans
- [04-execution.md](../../docs/guides/enterprise/tdd-workflow/04-execution.md) - Agent-driven test implementation
