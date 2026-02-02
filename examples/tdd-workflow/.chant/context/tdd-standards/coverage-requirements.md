# Coverage Requirements

## Team Coverage Policies

Acme SaaS Corp test coverage requirements by module risk level.

## Global Minimum

**80% coverage** for all production code.

## Module-Specific Requirements

### Critical Modules (85% minimum)

**Payments:**
- Refund processing
- Transaction handling
- Subscription billing

Rationale: Financial operations require exhaustive testing. Production incidents directly impact revenue.

### High-Risk Modules (80% minimum)

**Auth:**
- Login/logout
- Password reset
- OAuth integration

Rationale: Security-sensitive code paths require comprehensive coverage.

### Standard Modules (75% minimum)

**Analytics:**
- Reporting
- Dashboard queries
- Export functionality

Rationale: Lower business risk, but still require solid coverage.

## Coverage Measurement

Coverage measured by:
1. Line coverage (primary metric)
2. Branch coverage (target: 70%+)
3. Function coverage (target: 90%+)

## Exclusions

Exclude from coverage requirements:
- Test files
- Migration scripts
- Development tooling
- Generated code
- Third-party integrations (mock at boundary)

## Enforcement

- CI fails if coverage drops below minimum
- Coverage reports required in all PRs
- Weekly coverage trend reports to engineering leadership

## Priority Levels

When addressing coverage gaps:

**P0 (Immediate):**
- Coverage <50% in critical modules
- Untested paths causing production incidents
- Security-sensitive code without tests

**P1 (This sprint):**
- Coverage 50-80% in critical modules
- Missing edge case tests
- Error handling gaps

**P2 (Next sprint):**
- Coverage 75-80% in standard modules
- Missing documentation tests
- Performance test gaps

**P3 (Backlog):**
- Coverage 80-85% (above minimum)
- Nice-to-have test additions
- Test refactoring for clarity
