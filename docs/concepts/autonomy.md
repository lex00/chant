# Autonomous Workflows

## Self-Driving Specs

Specs that execute, verify, and correct themselves:

- **Execute**: `chant work` invokes an agent to implement the spec
- **Verify**: `chant verify` re-checks acceptance criteria
- **Detect Drift**: Find when reality diverges from intent

## Intent Durability

Specs persist after completion and can be re-verified. When reality diverges from the specification (code changes, tests fail, files deleted), the spec detects this drift and can be re-executed.

```
Traditional: Spec → Work → Done → Forget
Chant:       Spec → Work → Done → Verify → Drift? → Re-work → ...
```

## Autonomy Spectrum

| Level | Description | Approval Required |
|-------|-------------|-------------------|
| **Supervised** | Agent works, human reviews before merge | PR review |
| **Trusted** | Agent works, auto-merge low-risk | High-risk only |
| **Autonomous** | Agent works, auto-merge, human notified | Exceptions only |

## Decision Handling

**The spec is the decision point.** Decisions happen when writing the spec, not during execution.

## Manual Intervention

You can interrupt and take control of running work:

- **`chant takeover <spec-id>`** - Pause the agent, analyze progress, and update the spec for manual continuation.

See the [Advanced Patterns](../guides/oss-maintainer-workflow/08-advanced.md#controlling-running-work) guide for detailed usage examples.

## Spec Design for Autonomy

### Good Specs

```yaml
# ✓ Clear, verifiable, bounded
---
labels: [autonomous, trivial]
---
# Update copyright year
Change 2025 to 2026 in LICENSE file.
## Acceptance Criteria
- [ ] LICENSE shows 2026
```

### Bad Specs

```yaml
# ✗ Ambiguous, unbounded
---
# Improve the API
Make the API better.
```

## Decomposition

Use `chant split` to break down large specs:

```bash
$ chant split 001
Suggested breakdown:
  001.1 - Add User model
  001.2 - Implement registration
  001.3 - Add middleware
```

Good member specs are: independent, testable, small (<30 min), specific.

## Verification

```bash
# Verify a completed spec
$ chant verify 001
  ✓ Rate limiter middleware exists
  ✓ Returns 429 with Retry-After header
  ✓ Tests passing
Spec 001: VERIFIED

# Verify all completed specs
$ chant verify --all
```

## Drift Detection

Drift is when reality diverges from intent (feature removed, tests disabled, etc.).

```bash
$ chant drift
Drift Report:
  023: Rate limiting - middleware removed
  089: Auth middleware - file deleted
```



## Patterns

### Nightly Batch
```bash
# Cron: 0 2 * * *
chant work --chain --label autonomous --max 50
```

### Verification Gate
```yaml
# CI/CD
deploy:
  requires:
    - chant verify --all --exit-code
```


## Audit Trail

Specs track verification:

```yaml
---
status: completed
completed_at: 2026-01-10T15:30:00Z
last_verified: 2026-01-15T00:00:00Z
---
```

Use git to trace history:
```bash
$ git log --oneline -- .chant/specs/001.md
```

## Trust Building

Start supervised, graduate to autonomous:

1. **Week 1**: `autonomy.level: supervised` - review all PRs
2. **Week 2**: Auto-merge `[trivial]` labels
3. **Week 3**: Auto-merge `[trivial, deps, docs]`
4. **Week 4+**: Full autonomous with guardrails

**Goal: More done, less human time, acceptable risk, intent preserved.**
