# Advanced Patterns

Security issues, breaking changes, performance investigations, and complex multi-spec workflows.

## Security Issues

Security vulnerabilities require special handling:

### Private Spec Handling

Don't expose security details in public specs until fixed:

```yaml
---
type: task
status: ready
labels:
  - security
  - private
---

# Research security vulnerability (CVE pending)

## Context

Reported via security@yourproject.org. Details not in public issue tracker.

## Acceptance Criteria

- [ ] Vulnerability confirmed
- [ ] Impact assessed
- [ ] Fix developed
- [ ] CVE requested (if applicable)
```

### Security Research Prompt

Create a security-focused research prompt:

```markdown
# .chant/prompts/security-research.md

You are conducting security research on a reported vulnerability.

Your goal is to:
1. Confirm the vulnerability exists
2. Determine attack vectors and impact
3. Identify all affected code paths
4. Assess severity (CVSS if applicable)
5. Recommend remediation approach

Security guidelines:
- Do not log sensitive data during testing
- Use isolated test environment
- Document proof-of-concept minimally
- Consider disclosure timeline

Output:
- Vulnerability confirmation
- Impact assessment (confidentiality, integrity, availability)
- Affected versions
- Recommended fix approach
- Disclosure timeline recommendation
```

### CVE Workflow

```bash
# 1. Private research spec
chant add "Research security issue SEC-2026-001" --type task --label security

# 2. Private fix spec
chant add "Fix security issue SEC-2026-001" --type code --label security

# 3. Review with security focus
chant add "Security review SEC-2026-001" --type task --label security

# 4. After fix merged and released:
# - Request CVE
# - Publish advisory
# - Make specs public (redact sensitive details)
```

### Coordinated Disclosure

For issues affecting multiple projects:

```yaml
---
type: task
labels:
  - security
  - coordinated-disclosure
---

# Coordinate disclosure for CVE-2026-XXXX

## Timeline

- [ ] 2026-01-15: Notify affected projects
- [ ] 2026-01-22: Verify all projects have patches ready
- [ ] 2026-01-29: Coordinated release
- [ ] 2026-01-30: Public disclosure

## Affected Projects

- ourproject (this repo)
- related-lib (upstream)
- downstream-tool (dependent)
```

## Breaking Changes

Breaking changes require extra care:

### Breaking Change Labels

```bash
chant add "Remove deprecated API" --type code --label breaking-change
```

### Deprecation Workflow

Before removing, deprecate first:

```yaml
---
type: code
labels:
  - deprecation
  - v0.7.0
---

# Deprecate old_method() in favor of new_method()

## Context

`old_method()` has performance issues. Introducing `new_method()` with
better implementation. Will remove `old_method()` in v0.9.0.

## Acceptance Criteria

- [ ] Add `#[deprecated]` attribute with migration message
- [ ] Add deprecation notice to CHANGELOG
- [ ] Add migration guide to documentation
- [ ] All internal callers migrated to new_method()
```

Then in a future release:

```yaml
---
type: code
labels:
  - breaking-change
  - v0.9.0
depends_on:
  - <deprecation-spec-id>  # Must be released first
---

# Remove deprecated old_method()

## Context

Deprecated in v0.7.0 with 2-release notice period.

## Acceptance Criteria

- [ ] Remove old_method() implementation
- [ ] Update documentation
- [ ] Add to breaking changes in CHANGELOG
```

### SemVer-Aware Labels

Organize by version impact:

```bash
# Breaking change (major version bump)
chant list --label breaking-change

# New features (minor version bump)
chant list --label feature

# Bug fixes (patch version bump)
chant list --label fix
```

## Performance Issues

Performance investigations need benchmarking:

### Profiling Research Spec

```yaml
---
type: task
status: ready
prompt: research
labels:
  - performance
  - issue-1240
informed_by:
  - benchmarks/storage_bench.rs
target_files:
  - .chant/research/performance-issue-1240.md
---

# Research performance regression in storage module

## Context

User reports 10x slowdown after upgrading to v0.7.0. Likely related to
new locking implementation.

## Research Questions

- [ ] What is the baseline performance (v0.6.0)?
- [ ] What is current performance (v0.7.0)?
- [ ] Where is time being spent (profile)?
- [ ] What is the overhead of locking?
- [ ] Can performance be improved without removing locking?

## Methodology

1. Run benchmark suite on v0.6.0
2. Run benchmark suite on v0.7.0
3. Profile hot paths with flamegraph
4. Identify optimization opportunities

## Acceptance Criteria

- [ ] Benchmark comparison documented
- [ ] Profile data analyzed
- [ ] Root cause of slowdown identified
- [ ] Optimization recommendations provided
```

### Benchmark-Informed Implementation

```yaml
---
type: code
labels:
  - performance
  - issue-1240
informed_by:
  - .chant/research/performance-issue-1240.md
  - benchmarks/storage_bench.rs
---

# Optimize lock contention in storage module

## Context

Research identified lock contention as the bottleneck. Recommendation:
use read-write lock instead of exclusive lock.

## Acceptance Criteria

- [ ] Replace Mutex with RwLock for read operations
- [ ] Benchmark shows ≤20% overhead vs v0.6.0 (was 10x)
- [ ] All existing tests pass
- [ ] No correctness regression
```

## Multi-Spec Dependencies

Complex changes span multiple specs:

### Using `depends_on`

```yaml
# Spec 1: Foundation
---
type: code
labels:
  - refactor
  - issue-1234-phase1
---

# Refactor Lock module for RwLock support

# Spec 2: Depends on foundation
---
type: code
depends_on:
  - 2026-01-29-010-abc  # Spec 1
labels:
  - fix
  - issue-1234-phase2
---

# Fix issue #1234 using RwLock
```

Specs with unmet dependencies show as `blocked`:

```bash
$ chant list
2026-01-29-010-abc  ready     Refactor Lock module
2026-01-29-011-def  blocked   Fix issue #1234 (depends on 010-abc)
```

### Using Driver Specs

For coordinated multi-part work:

```yaml
---
type: driver
labels:
  - epic
  - storage-overhaul
---

# Storage module overhaul

## Member Specs

This driver coordinates the storage overhaul:

1. `research-storage-architecture` - Research current issues
2. `refactor-lock-module` - Improve locking primitives
3. `fix-concurrent-writes` - Fix data loss
4. `optimize-read-path` - Improve read performance
5. `update-documentation` - Document new behavior
```

### Using Labels for Organization

Group related specs:

```bash
# Create specs with shared label
chant add "Research storage issues" --type task --label storage-overhaul
chant add "Refactor lock module" --type code --label storage-overhaul
chant add "Fix concurrent writes" --type code --label storage-overhaul

# Work all ready specs with label
chant work --parallel --label storage-overhaul

# Check status
chant list --label storage-overhaul
```

## Parallel Execution

Work multiple issues concurrently:

### Label-Based Parallel Execution

```bash
# Label specs for the release
chant add "Fix issue #1234" --type code --label v0.8.0
chant add "Fix issue #1235" --type code --label v0.8.0
chant add "Fix issue #1236" --type code --label v0.8.0

# Execute all in parallel
chant work --parallel --label v0.8.0
```

### Managing Parallel Work

```bash
# Limit concurrent agents
chant work --parallel --label v0.8.0 --max-parallel 3

# Check progress
chant list --label v0.8.0 --status in_progress
```

### Handling Parallel Conflicts

If parallel specs conflict (both modify same file):

```bash
# Auto-resolve with rebase
chant merge --all --rebase --auto

# Or handle manually
chant merge 2026-01-29-010-abc  # Merge first
chant merge 2026-01-29-011-def --rebase  # Rebase second onto first
```

## Spec Relationships

### `informed_by` vs `depends_on`

| Relationship | Meaning | Effect |
|-------------|---------|--------|
| `informed_by` | "Read this for context" | No blocking |
| `depends_on` | "Must complete first" | Blocks execution |

Use `informed_by` for research context:
```yaml
informed_by:
  - .chant/specs/triage-spec.md
  - .chant/research/rca-document.md
```

Use `depends_on` for sequencing:
```yaml
depends_on:
  - 2026-01-29-010-abc  # Must complete first
```

### Spec Chains

For linear workflows:

```bash
# Work specs in specific order
chant work --chain spec1 spec2 spec3
```

### Spec Groups

For coordinated parallel work:

```yaml
---
type: group
labels:
  - epic
---

# Q1 Bug Bash

## Member Specs

Execute these specs as a coordinated group:

- 2026-01-29-010-abc (independent)
- 2026-01-29-011-def (independent)
- 2026-01-29-012-ghi (depends on 010-abc)
```

## Handling Stuck Agents

When an agent isn't making progress:

### Monitor Progress

```bash
# Check agent logs
chant log 2026-01-29-010-abc

# Look for:
# - Repeated errors
# - Circular changes
# - Scope creep
```

### Stop and Split

If agent is struggling:

```bash
# Stop the work
# (Ctrl+C or chant cancel <spec-id>)

# Review what was attempted
chant log 2026-01-29-010-abc

# Split into phases
chant add "Research: Understand the issue" --type task
chant add "Implement: Make the fix" --type code
```

### Research-Then-Implement Pattern

When uncertain:

1. **Research spec** (task type): Investigate, document findings
2. **Implementation spec** (code type): Implement based on research

The research spec de-risks the implementation.

## Cross-Repository Issues

For issues spanning multiple repos:

```yaml
---
type: task
labels:
  - cross-repo
  - upstream-dependency
---

# Coordinate fix across repositories

## Context

Bug exists in upstream library. Need to:
1. Fix in upstream
2. Update dependency here
3. Update downstream tools

## Coordination

| Repo | Spec | Status |
|------|------|--------|
| upstream-lib | PR #123 | merged |
| this-repo | 2026-01-29-010-abc | ready |
| downstream-tool | Issue #456 | waiting |
```

## Emergency Hotfixes

For urgent production issues:

```bash
# Skip triage, go straight to reproduction
chant add "URGENT: Reproduce production crash" --type task --label hotfix

# Expedited research
chant add "URGENT: Research crash root cause" --type task --label hotfix

# Quick fix
chant add "URGENT: Fix production crash" --type code --label hotfix

# Fast track all
chant work --chain <repro-id> <research-id> <fix-id>
```

Still follow the research-first approach, just faster.

## See Also

- [Root Cause Analysis](03-research.md) — Research patterns
- [Implementation](04-implementation.md) — Implementation patterns
- [Complete Walkthrough](09-example.md) — Full example with advanced patterns
