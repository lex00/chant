# Autonomous Workflows

## Self-Driving Specs

Specs that execute, verify, and correct themselves.

### Execute
`chant work` invokes an agent to implement the spec.

### Verify
`chant verify` re-checks acceptance criteria to detect when a completed spec's intent no longer holds true in the current codebase.

### Detect Drift
Continuous verification finds when reality diverges from intent.

### Replay
`chant replay` re-executes intent to fix drift.

---

## The Vision: Intent Durability

Chant enables [intent-first development](../getting-started/philosophy.md) - specifications that persist, verify themselves over time, detect drift, and can replay to self-correct.

This goes beyond "agent does work":

- **Durable** - Intent persists. Specs are specifications, not just todos.
- **Self-verifying** - Specs check if their intent still holds.
- **Drift-aware** - Know when reality diverges from what was specified.
- **Self-correcting** - Replay to restore intent without manual work.
- **Auditable** - Complete history of intent, execution, verification, drift.
- **Autonomous** - Agents execute. Humans review exceptions.

The AI part is just how specifications execute themselves. The real value is **intent that persists and remains true**.

The goal is to guide users toward more autonomy as trust builds.

```
Manual          Supervised        Autonomous
  │                 │                 │
  ▼                 ▼                 ▼
┌─────┐         ┌─────┐           ┌─────┐
│Human│────────▶│Human│──────────▶│Agent│
│works│         │reviews│         │works│
└─────┘         └─────┘           └─────┘
                    │
              Chant starts here,
              guides toward here ──────▶
```

## Autonomy Spectrum

| Level | Description | Approval Required |
|-------|-------------|-------------------|
| **Manual** | Human does work, chant tracks | N/A |
| **Assisted** | Agent suggests, human applies | Every change |
| **Supervised** | Agent works, human reviews before merge | PR review |
| **Trusted** | Agent works, auto-merge low-risk | High-risk only |
| **Autonomous** | Agent works, auto-merge, human notified | Exceptions only |

## Configuring Autonomy

```yaml
# config.md
autonomy:
  level: supervised          # assisted | supervised | trusted | autonomous

  # What triggers human review
  review_required:
    - files: ["*.md", "docs/**"]      # Never auto-merge docs
    - labels: [security, breaking]     # Always review these
    - cost_above: 5.00                 # Review expensive specs
    - files_changed_above: 10          # Review large changes

  # What can auto-merge
  auto_merge:
    - labels: [trivial, typo, deps]
    - files: ["*.lock", "generated/**"]
    - tests_passing: true              # Only if tests pass
```

## Decision Handling

During execution, agents encounter decision points: ambiguous requirements, multiple valid approaches, missing context, unexpected state.

### The Principle

**The spec is the decision point.** Decisions happen when writing the spec, not during execution.

- Clear spec + clear criteria → Agent executes
- Ambiguous spec → Agent interprets, documents reasoning
- Conflicting requirements → Agent fails, requests clarification

### Decision Authority

Specs can declare how the agent should handle ambiguity:

```yaml
# Spec frontmatter
decisions: autonomous   # autonomous | document | pause | fail
```

| Level | Behavior |
|-------|----------|
| `autonomous` | Decide and continue (default for trusted/autonomous) |
| `document` | Decide, document reasoning, continue |
| `pause` | Stop and request human input |
| `fail` | Mark spec failed if any ambiguity |

### Decision Framework

When facing ambiguity, agents follow this hierarchy:

1. **Spec criteria** - Does the acceptance criteria imply an answer?
2. **Existing patterns** - What does the codebase already do?
3. **Philosophy** - What does philosophy.md say?
4. **Simplicity** - Choose the simpler option
5. **Document** - Record the decision and reasoning

### Documenting Decisions

Decisions are recorded in the commit message:

```
chant(001): Add user authentication

Decision: Used JWT over sessions
Reasoning: Existing auth code uses JWT pattern, spec didn't specify
```

Or in the spec's amendments section:

```yaml
amendments:
  - date: 2026-01-24
    decision: "Used bcrypt for password hashing"
    reasoning: "Spec said 'secure hashing', bcrypt is codebase standard"
```

### Bootstrap Decisions

During self-bootstrap, chant building itself follows the same rules:

1. **Spec quality matters** - Vague specs produce vague results
2. **Philosophy guides** - philosophy.md is context for all decisions
3. **Document everything** - The bootstrap story is in the commit history
4. **Fail early** - Better to stop than build wrong thing

Example bootstrap spec with decision guidance:

```yaml
---
status: pending
decisions: document
---

# Add spec parser

Parse YAML frontmatter from markdown files.

## Acceptance Criteria
- [ ] Extracts frontmatter between `---` markers
- [ ] Returns structured data with all fields
- [ ] Handles missing frontmatter gracefully

## Decision Guidance
- Use serde_yaml (already in Cargo.toml)
- Follow existing error patterns in src/error.rs
- If unclear, check how similar parsers work in codebase
```

### Unplanned Decisions

What if something comes up that the spec doesn't cover?

| Situation | Action |
|-----------|--------|
| **Minor detail** | Decide, document, continue |
| **Affects acceptance criteria** | Pause or fail |
| **Contradicts other specs** | Fail, request clarification |
| **Security implication** | Fail unless `decisions: autonomous` |

The goal: agents can work autonomously on well-specified work. Ambiguity in specs should be fixed in the spec, not resolved during execution.

## Prompts for Autonomy

### The Autonomous Prompt

Built-in prompt optimized for autonomous execution:

```markdown
# .chant/prompts/autonomous.md

You are an autonomous agent. Complete the spec without human intervention.

## Principles

1. **Self-sufficient** - Don't ask questions. Make reasonable decisions.
2. **Verify before commit** - Run tests, lint, type-check.
3. **Small commits** - Checkpoint frequently.
4. **Fail gracefully** - If stuck, document what you tried and mark failed.

## Decision Framework

Check the spec's `decisions` field for authority level. When facing ambiguity:

1. Does acceptance criteria imply an answer? → Use it
2. Does codebase have a pattern? → Follow it
3. Does philosophy.md guide this? → Apply it
4. Neither? → Choose simpler option, document reasoning

Record decisions in commit message:
```
chant(001): Implement feature

Decision: Used X over Y
Reasoning: Codebase pattern, simpler
```

## Before Completing

- [ ] All tests pass
- [ ] No lint errors
- [ ] Changes are minimal (no scope creep)
- [ ] Commit message explains the "why"
```

### Prompt Hierarchy for Autonomy

```yaml
# config.md
prompts:
  default: supervised        # Safe default

  # Override by context
  by_label:
    trivial: autonomous
    security: assisted       # Extra caution

  by_project:
    docs: autonomous         # Docs are low-risk
    payments: supervised     # Payments need review
```

## Spec Design for Autonomy

### Good Autonomous Specs

Specs that work well autonomously:

```yaml
# ✓ Good: Clear, verifiable, bounded
---
status: pending
labels: [autonomous, trivial]
---
# Update copyright year in LICENSE

Change 2025 to 2026 in LICENSE file.

## Acceptance Criteria
- [ ] LICENSE shows 2026
- [ ] No other changes
```

```yaml
# ✓ Good: Specific, testable
---
status: pending
labels: [autonomous]
target_files: [src/utils/format.go]
---
# Add formatDuration helper

Add a function to format durations as human-readable strings.

Examples:
- 45 → "45s"
- 90 → "1m 30s"
- 3665 → "1h 1m 5s"

## Acceptance Criteria
- [ ] Function exists in src/utils/format.go
- [ ] Unit tests pass
- [ ] Handles edge cases (0, negative)
```

### Bad Autonomous Specs

Specs that need human judgment:

```yaml
# ✗ Bad: Ambiguous
---
# Improve the API

Make the API better.
```

```yaml
# ✗ Bad: Unbounded scope
---
# Refactor authentication

Clean up the auth code.
```

```yaml
# ✗ Bad: Requires design decisions
---
# Add caching

Add caching to improve performance.
# Which cache? Where? What TTL? Invalidation?
```

## Spec Decomposition

### The Split Prompt

Use an agent to break down large specs:

```bash
$ chant split 001
Using prompt: split

Agent analyzing spec 001...

Suggested breakdown:
  001.1 - Add User model and migration
  001.2 - Implement registration endpoint
  001.3 - Implement login endpoint
  001.4 - Add JWT token generation
  001.5 - Add authentication middleware
  001.6 - Write integration tests

Create these group members? [y/N]
```

### Decomposition Criteria

Good member specs are:
- **Independent** - Can be done in any order (or explicit deps)
- **Testable** - Clear pass/fail criteria
- **Small** - 1-2 files, <30 minutes agent time
- **Specific** - No ambiguity about what "done" means

## Chaining and Dependencies

### Sequential Chains

```yaml
# 001.md - Driver spec
---
status: pending
---
# Add user authentication

## Members (sequential)
- 001.1: Add User model (no deps)
- 001.2: Add registration (depends: 001.1)
- 001.3: Add login (depends: 001.1)
- 001.4: Add middleware (depends: 001.2, 001.3)
```

### Parallel Execution

```bash
$ chant work --parallel --max 3
# Runs 001.1 first
# Then 001.2 and 001.3 in parallel
# Then 001.4 after both complete
```

### Automatic Chaining

```yaml
# config.md
autonomy:
  chain:
    enabled: true
    # After completing a spec, auto-start next ready spec
    max_chain: 10            # Stop after 10 specs
    pause_on_failure: true   # Stop chain if spec fails
```

```bash
$ chant work --chain
Starting spec 001.1...
✓ 001.1 complete

Starting spec 001.2...
✓ 001.2 complete

Starting spec 001.3...
✓ 001.3 complete

Starting spec 001.4...
✓ 001.4 complete

Chain complete: 4 specs
```

## Monitoring Autonomous Work

### Watch Mode

```bash
$ chant watch --all
Watching 3 agents...

[001] ████████░░ 80% - Running tests
[002] ██████████ Done - Merged
[003] ██░░░░░░░░ 20% - Implementing

Press q to quit, p to pause all, c to cancel spec
```

### Notifications

```yaml
# config.md
autonomy:
  notifications:
    on_complete: true
    on_failure: true
    on_review_needed: true
    channel: slack           # slack | email | desktop
```

### Daily Summary

```bash
$ chant summary --yesterday
Autonomous Work Summary (2026-01-21)

Completed: 23 specs
Failed: 2 specs
Pending review: 5 specs

Auto-merged: 18 specs
Human-merged: 5 specs

Cost: $12.45
Agent time: 4.2 hours

Top failures:
  - 015: Tests failed (auth edge case)
  - 022: Merge conflict

Review queue:
  - 019: Security label, needs review
  - 021: Large change (15 files)
```

## Trust Building

### Start Supervised

New users should start supervised:

```yaml
# config.md (starter)
autonomy:
  level: supervised
  auto_merge: false
```

### Graduate to Trusted

After building confidence:

```yaml
# config.md (experienced)
autonomy:
  level: trusted
  auto_merge:
    - labels: [trivial, deps, docs]
    - tests_passing: true
```

### Full Autonomy

For mature workflows:

```yaml
# config.md (autonomous)
autonomy:
  level: autonomous
  review_required:
    - labels: [security, breaking]
    - cost_above: 10.00
```

## Autonomous Patterns

### Pattern: Nightly Batch

Run autonomous specs overnight:

```bash
# Cron: 0 2 * * *
chant work --chain --label autonomous --max 50
chant summary --today | mail -s "Chant Summary" team@company.com
```

### Pattern: PR-Triggered

When PR is created, run related specs:

```yaml
# .github/workflows/chant.yml
on:
  pull_request:
    types: [opened]

jobs:
  chant:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: |
          chant work --label "pr-prep" --chain
          chant work --label "tests" --parallel
```

### Pattern: Issue-to-Spec-to-PR

Fully autonomous from issue to merged PR:

```yaml
# config.md
scm:
  github:
    sync_issues: true
    issue_labels: [autonomous]    # Only these become specs

autonomy:
  level: autonomous
  auto_pr: true
  auto_merge:
    - tests_passing: true
    - approvals: 0                # No human approval needed
```

```
GitHub Issue → Chant Spec → Agent Work → PR Created → Auto-Merge
    (human)      (sync)      (autonomous)   (auto)      (auto)
```

### Pattern: Continuous Improvement

Agent finds and fixes issues:

```yaml
# .chant/prompts/improve.md
Scan the codebase for:
- TODO comments older than 30 days
- Functions longer than 50 lines
- Test files with low coverage

For each issue found, create a member spec.
```

```bash
$ chant work --prompt improve
Agent scanning...
Created 5 improvement specs:
  - 001.1: Resolve TODO in auth.go (45 days old)
  - 001.2: Split handleRequest (78 lines)
  - 001.3: Add tests for utils/format.go (40% coverage)
  ...
```

## Guardrails

### Cost Limits

```yaml
autonomy:
  limits:
    cost_per_spec: 5.00        # Pause if spec exceeds
    cost_per_day: 100.00       # Stop autonomous work
    cost_per_month: 2000.00
```

### Scope Limits

```yaml
autonomy:
  limits:
    files_per_spec: 10         # Flag if more files changed
    lines_per_spec: 500        # Flag large changes
    duration_minutes: 60       # Timeout long specs
```

### Rollback on Failure

```yaml
autonomy:
  on_failure:
    auto_rollback: true        # Revert if tests fail after merge
    notify: true
    pause_chain: true
```

## Getting Started with Autonomy

### Week 1: Supervised

```bash
chant init
# Edit config: autonomy.level = supervised
chant add "Fix typo in README"
chant work 001
# Review PR, merge manually
```

### Week 2: Trusted (Low-Risk)

```yaml
autonomy:
  level: trusted
  auto_merge:
    - labels: [trivial]
```

### Week 3: Trusted (More Scope)

```yaml
autonomy:
  auto_merge:
    - labels: [trivial, deps, docs]
    - tests_passing: true
```

### Week 4+: Autonomous

```yaml
autonomy:
  level: autonomous
  chain:
    enabled: true
```

## Durability: Specs as Executable Specifications

### The Insight: Intent Durability

A spec isn't just work to do once. It's a **specification of intent** that persists - this is **intent durability**.

```
Traditional: Spec → Work → Done → Forget
Chant:       Spec → Work → Done → Verify → Re-verify → Drift? → Replay → ...
```

The spec file is durable:
- Git-tracked forever
- Describes what should be true
- Can be re-verified at any time
- Can detect when reality drifts
- Can replay to restore intent

**Intent durability means your specifications remain true, not just documented.**

### Specs as Specifications

```yaml
---
status: completed
commit: abc123
completed_at: 2026-01-22T15:00:00Z
---

# Add rate limiting to API

All API endpoints must return 429 after 100 requests/minute per IP.

## Acceptance Criteria
- [ ] Rate limiter middleware exists
- [ ] Returns 429 with Retry-After header
- [ ] Limits are configurable
- [ ] Tests verify rate limiting works
```

This isn't just "what we did" - it's "what should remain true."

### Verification Over Time

```bash
# Verify a completed spec still holds
$ chant verify 001
Verifying spec 001: Add rate limiting to API

Checking acceptance criteria...
  ✓ Rate limiter middleware exists (src/middleware/ratelimit.go)
  ✓ Returns 429 with Retry-After header
  ✓ Limits are configurable (config.yaml)
  ✓ Tests verify rate limiting works (4 tests passing)

Spec 001: VERIFIED
```

### Continuous Verification

```yaml
# config.md
verification:
  continuous: true
  schedule: daily              # daily | weekly | on_commit
  scope: completed             # completed | all
  notify_on_drift: true
```

```bash
# Nightly cron
$ chant verify --all --completed
Verifying 147 completed specs...

  145 verified
  2 drifted:
    - 023: Rate limiting - tests now failing
    - 089: Auth middleware - file was deleted

Drift report sent to team@company.com
```

## Drift Detection

### What is Drift?

Drift is when reality diverges from intent:

| Intent (Spec) | Reality (Code) | Drift |
|---------------|----------------|-------|
| "Add rate limiting" | Rate limiter removed | Feature drift |
| "No external deps" | Added 3 npm packages | Dependency drift |
| "Max 100 lines" | Function is 200 lines | Quality drift |
| "Tests must pass" | Tests disabled | Verification drift |

### Drift Without Forcing Workflow

Chant detects drift **without requiring a specific workflow**:

```bash
# Check for drift whenever you want
$ chant drift

# Or on every commit (optional hook)
$ chant drift --since HEAD~1

# Or on a schedule
$ chant drift --schedule weekly
```

No mandatory gates. Information, not enforcement (unless you want it).

### Drift Sources

```yaml
# config.md
drift:
  detect:
    # File-based drift
    - type: file_deleted
      description: "Target file from spec was deleted"

    # Test-based drift
    - type: tests_failing
      description: "Tests that passed at completion now fail"

    # Criteria-based drift
    - type: criteria_unmet
      description: "Acceptance criteria no longer satisfied"

    # Dependency drift
    - type: dependency_added
      when: spec.labels.includes("no-deps")

    # Custom patterns
    - type: pattern
      pattern: "TODO.*HACK"
      description: "Hack introduced that should be removed"
```

### Drift Report

```bash
$ chant drift --report
Drift Report (2026-01-22)

## Feature Drift (2 specs)

### 023: Add rate limiting
- Completed: 2026-01-15
- Drift detected: 2026-01-20
- Cause: Middleware removed in commit def456
- Blame: alice (refactoring PR #89)
- Impact: API has no rate limiting

### 089: Auth middleware
- Completed: 2025-12-01
- Drift detected: 2026-01-18
- Cause: File deleted, not replaced
- Blame: bob (cleanup PR #102)
- Impact: Authentication may be broken

## Quality Drift (1 spec)

### 045: Refactor handleRequest
- Criteria: "Function under 50 lines"
- Current: 87 lines
- Drift introduced: commit ghi789

## Suggested Actions

1. Re-run spec 023 to restore rate limiting
2. Investigate spec 089 - was deletion intentional?
3. Create follow-up spec for 045
```

### Drift Notifications

```yaml
drift:
  notify:
    on_detection: true
    channel: slack
    threshold: 1               # Notify on any drift

    # Or batch
    schedule: weekly
    report: true
```

## Replay: Re-executing Intent

### What is Replay?

Replay means re-executing a spec's intent against current code:

```
Original: Spec → Agent → Code (2 months ago)
Replay:   Spec → Agent → Code (now, against current codebase)
```

Use cases:
- Fix drift by re-applying intent
- Verify spec would still succeed
- Update implementation for new patterns

### Replay a Single Spec

```bash
$ chant replay 023
Replaying spec 023: Add rate limiting

Original completion: 2026-01-15
Current codebase: 2026-01-22

Agent analyzing...
  - Original implementation was removed
  - Current patterns differ (new middleware framework)
  - Will re-implement using current patterns

Proceed? [y/N]

[Agent re-implements rate limiting]

Replay complete.
  Commit: xyz789
  Files changed: 2
  Tests: passing
```

### Replay vs Retry

| Replay | Retry |
|--------|-------|
| Re-execute completed spec | Re-attempt failed spec |
| Against current codebase | From where it left off |
| May produce different implementation | Same implementation goal |
| Fixes drift | Fixes failure |

### Replay a Driver

Replay all members of a driver spec to verify/restore:

```bash
$ chant replay 001 --recursive
Replaying driver 001: User Authentication (5 members)

  001.1: Add User model - VERIFIED (no changes needed)
  001.2: Add registration - VERIFIED (no changes needed)
  001.3: Add login - DRIFTED (re-implementing...)
  001.4: Add middleware - DRIFTED (re-implementing...)
  001.5: Add tests - VERIFIED (no changes needed)

Replay complete:
  3 verified (no changes)
  2 re-implemented

New commits:
  abc123: chant(001.3): replay - restore login endpoint
  def456: chant(001.4): replay - restore auth middleware
```

### Replay Modes

```yaml
# config.md
replay:
  mode: prompt                 # prompt | auto | dry-run

  # What to do when replaying
  strategy:
    verified: skip             # skip | re-run
    drifted: prompt            # prompt | auto-fix | fail

  # Preserve or update implementation
  implementation: update       # preserve | update
  # preserve: try to restore original
  # update: re-implement with current patterns
```

### Dry-Run Replay

See what would happen without making changes:

```bash
$ chant replay 023 --dry-run
Dry-run replay of spec 023: Add rate limiting

Would re-implement:
  - Create src/middleware/ratelimit.go
  - Add tests in src/middleware/ratelimit_test.go
  - Update src/api/router.go

Estimated:
  - Files: 3
  - Lines: ~150
  - Cost: ~$0.50

No changes made (dry-run).
```

## Audit Trail for Intent

### Complete History

Every spec maintains full history:

```yaml
---
status: completed
history:
  - event: created
    at: 2026-01-10T10:00:00Z
    by: alex

  - event: started
    at: 2026-01-10T14:00:00Z
    agent: provider/model-name

  - event: completed
    at: 2026-01-10T15:30:00Z
    commit: abc123

  - event: verified
    at: 2026-01-15T00:00:00Z
    result: passed

  - event: drift_detected
    at: 2026-01-20T00:00:00Z
    cause: file_deleted
    blame_commit: def456

  - event: replayed
    at: 2026-01-22T10:00:00Z
    commit: xyz789
    by: alex
---
```

### Tracing Drift to Cause

```bash
$ chant audit 023
Spec 023: Add rate limiting

Timeline:
  2026-01-10  Created by alex
  2026-01-10  Completed (commit abc123)
  2026-01-15  Verified ✓
  2026-01-18  Verified ✓
  2026-01-20  DRIFT DETECTED
              └── Cause: src/middleware/ratelimit.go deleted
              └── Commit: def456 "refactor: clean up old middleware"
              └── Author: alice
              └── PR: #89 "Middleware cleanup"
  2026-01-22  Replayed by alex (commit xyz789)
  2026-01-22  Verified ✓

Intent preserved: YES (after replay)
```

### Intent vs Implementation

Chant distinguishes:

- **Intent** (spec) - What should be true
- **Implementation** (commit) - How it was achieved
- **Verification** (check) - Is it still true?

```
Intent: "API must have rate limiting"
    │
    ├── Implementation 1 (Jan 10): Custom middleware
    │       └── Drifted (Jan 20): Deleted
    │
    └── Implementation 2 (Jan 22): Using new framework
            └── Verified (current)
```

The intent persists even as implementations change.

## Patterns for Drift Management

### Pattern: Verification Gates

Run verification before deploy:

```yaml
# CI/CD
deploy:
  requires:
    - chant verify --all --exit-code
```

### Pattern: Drift Budget

Allow some drift, alert on too much:

```yaml
drift:
  budget:
    max_drifted_specs: 5       # Alert if more than 5
    max_drift_age: 7d          # Alert if drift older than 7 days
```

### Pattern: Auto-Replay

Automatically fix drift:

```yaml
drift:
  auto_replay:
    enabled: true
    when:
      - type: file_deleted
      - type: tests_failing
    require_approval: true     # PR for replay
```

### Pattern: Intent Documentation

Use specs as living documentation:

```bash
$ chant intent auth
Intent for 'auth' (label):

Specs:
  001: Add user model
       "Users table with email, password_hash, created_at"
       Status: verified ✓

  002: Add registration
       "POST /register creates user, returns JWT"
       Status: verified ✓

  003: Add login
       "POST /login validates credentials, returns JWT"
       Status: drifted ⚠ (endpoint returns session, not JWT)

  004: Add middleware
       "All /api/* routes require valid JWT"
       Status: verified ✓

Overall: 3/4 verified, 1 drifted
```

This is living documentation that verifies itself.

## Success Metrics

| Metric | Supervised | Trusted | Autonomous |
|--------|------------|---------|------------|
| Human review time | 100% | 50% | 10% |
| Specs per day | 5 | 15 | 50+ |
| Cost per spec | Higher | Medium | Lower |
| Failure rate | Low | Low | Acceptable |
| Drift detection | Manual | Weekly | Continuous |
| Mean time to fix drift | Days | Hours | Minutes |

The goal: **More done, less human time, acceptable risk, intent preserved.**
