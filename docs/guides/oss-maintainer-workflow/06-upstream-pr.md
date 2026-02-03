# Phase 6: Upstream PR - Human gate before opening real PR

After the staging PR is approved in your fork, create the upstream PR to the main project.

## Why Human Gate?

The human gate ensures:

- **Quality control** — Human reviews staging PR before upstream exposure
- **Communication** — Human writes appropriate upstream PR description
- **Context** — Human decides when to submit (timing, release cycles)
- **Relationship** — Human maintains relationship with upstream maintainers

The agent does the implementation, but the human decides when and how to submit upstream.

## Upstream PR Workflow

```
Staging PR      Human Review        Upstream PR       Upstream Review
 (Fork)              │                    │                   │
   │                 ▼                    ▼                   ▼
   ▼           ┌──────────┐         ┌──────────┐       ┌───────────┐
┌────────┐     │ Review   │         │ Human    │       │ Upstream  │
│Fork-   │────▶│ staging  │────────▶│ creates  │──────▶│ maintainer│
│internal│     │ PR       │         │ real PR  │       │ reviews   │
│PR      │     └──────────┘         └──────────┘       └───────────┘
└────────┘          │
                    └─── Must pass before upstream PR
```

## Reviewing the Staging PR

Before creating upstream PR, review the staging PR:

### Automated Checks

```bash
# Ensure CI passes in staging PR
gh pr checks

# Ensure tests pass locally
just test

# Ensure no lint warnings
cargo clippy
```

### Manual Review

Review the staging PR for:

1. **Code quality** — Is the implementation clean and maintainable?
2. **Test coverage** — Are edge cases covered?
3. **Documentation** — Are docs updated?
4. **Research alignment** — Does it follow the research recommendations?
5. **Commit history** — Is the history clean and meaningful?

### Agent-Assisted Review (Optional)

Create a review spec for agent-based review:

```bash
chant add "Phase 6: Upstream PR - Human review for issue #1234" --type documentation
```

```yaml
---
type: documentation
status: ready
depends_on:
  - 005-fork-fix
prompt: standard
informed_by:
  - .chant/research/issue-1234-comprehension.md
  - .chant/research/issue-1234-root-cause.md
  - .chant/research/issue-1234-sprawl.md
---

# Review staging PR for issue #1234

## Context

Staging PR created in fork. Need to verify before creating upstream PR.

## Review Checklist

- [ ] Reproduction test passes
- [ ] All existing tests pass
- [ ] New tests adequately cover the fix
- [ ] Code follows research recommendations
- [ ] No obvious regressions
- [ ] Documentation updated
- [ ] Commit history is clean

## Acceptance Criteria

- [ ] Review document with clear verdict
- [ ] Approval or rejection with reasoning
```

## Creating the Upstream PR

Once staging PR is approved, create the upstream PR:

```bash
# Ensure you're on the fix branch
git checkout fix/issue-1234

# Create upstream PR
gh pr create \
  --repo upstream-org/upstream-repo \
  --base main \
  --head yourusername:fix/issue-1234 \
  --title "Fix #1234: Data loss on concurrent writes" \
  --body "$(cat .chant/upstream-pr-description.md)"
```

### Upstream PR Description Template

The PR description should reference the research:

```markdown
## Summary

Fixes #1234 - data loss during concurrent write operations.

## Root Cause

The storage layer used optimistic locking that didn't properly handle
concurrent writes. See research documentation in `.chant/research/` for
full analysis.

## Solution

Added pessimistic locking to the write path using the existing Lock module.

## Testing

- Added regression test: `tests/regression/issue_1234_test.rs`
- Added comprehensive concurrency tests
- All existing tests pass

## Research Trail

This fix was developed using research-driven workflow:
- Comprehension: `.chant/research/issue-1234-comprehension.md`
- Root cause: `.chant/research/issue-1234-root-cause.md`
- Sprawl analysis: `.chant/research/issue-1234-sprawl.md`

## Checklist

- [x] Tests pass locally
- [x] Staging PR reviewed and approved
- [x] Documentation updated
- [x] No breaking changes
```

## Human Decision Points

The human gate provides decision points:

### Timing

- **Wait for release cycle** — Don't submit if upstream is in code freeze
- **Coordinate with maintainers** — Check if they're expecting the PR
- **Bundle related fixes** — Wait for related fixes to complete

### Communication

- **Write clear description** — Explain the fix in upstream maintainer terms
- **Reference research** — Link to research docs if helpful
- **Set expectations** — Note if this is a breaking change

### Scope

- **Split or combine** — Decide if multiple fixes should be separate PRs
- **Breaking changes** — Decide if breaking changes are acceptable
- **Deprecation** — Add deprecation warnings if needed

## After Upstream PR

Once upstream PR is created:

1. **Monitor CI** — Ensure upstream CI passes
2. **Respond to reviews** — Address maintainer feedback
3. **Update staging PR** — Apply changes to staging PR first, then upstream
4. **Iterate** — Continue until upstream PR is merged

## Archive Research

After upstream PR is merged:

```bash
# Archive all specs for this issue
chant archive <comprehension-spec-id>
chant archive <repro-spec-id>
chant archive <root-cause-spec-id>
chant archive <sprawl-spec-id>
chant archive <implementation-spec-id>

# Or use label to archive all
chant archive --label issue-1234
```

## See Also

- [Fork Fix](05-fork-fix.md) — Previous step: implement and create staging PR
- [Comprehension Research](01-comprehension.md) — Start of the workflow
