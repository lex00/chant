# Approval Workflow

Your team is building a payments integration. Sarah creates a spec to refactor the auth system — a change that touches security-critical code. Before an agent touches anything, someone needs to sign off. This is what the approval workflow is for.

## Adding an Approval Gate

Sarah creates the spec with `--needs-approval`:

```bash
$ chant add "Refactor authentication to support OAuth2" --needs-approval
Created spec: 2026-01-28-001-abc
```

The spec is created with `approval.required: true` and `approval.status: pending` in frontmatter:

```yaml
---
type: code
status: pending
approval:
  required: true
  status: pending
---
```

If she tries to execute it now, chant blocks:

```bash
$ chant work 001
Error: Spec requires approval before work can begin

  Approval status: pending

  To approve:  chant approve 001 --by <name>
  To bypass:   chant work 001 --skip-approval
```

## Review and Approve

The tech lead reviews the spec and approves:

```bash
$ chant approve 001-abc --by marcos
```

Chant validates the approver name against git committers (warns if not found), updates the frontmatter, and appends a timestamped entry to the spec's "## Approval Discussion" section:

```markdown
## Approval Discussion

**marcos** - 2026-01-28 14:30 - APPROVED
```

The frontmatter now reads:

```yaml
approval:
  required: true
  status: approved
  by: marcos
  at: 2026-01-28T14:30:45Z
```

Now `chant work 001` proceeds normally.

## Rejection

But sometimes the spec isn't ready. Marcos reads the spec and sees it's too broad:

```bash
$ chant reject 001-abc --by marcos --reason "Scope too large — split auth token handling from session management"
```

The rejection is recorded in the approval discussion:

```markdown
## Approval Discussion

**marcos** - 2026-01-28 10:15 - REJECTED
Scope too large — split auth token handling from session management
```

The spec can't be worked until the issues are addressed. Sarah edits the spec, narrows the scope, and Marcos approves the revised version:

```bash
$ chant edit 001
# Narrow scope to just OAuth2 token handling

$ chant approve 001-abc --by marcos
```

The discussion section now has the full history — rejection reason, then approval — all tracked in git.

## Rejection Handling Modes

By default, rejection is manual — the author reads the feedback and edits the spec. But you can configure automatic responses to rejection:

### Dependency Mode

```yaml
# .chant/config.md
approval:
  rejection_action: dependency
```

When a spec is rejected, chant automatically creates a fix spec and blocks the original on it. Useful when rejection identifies prerequisite work — "you need to add the token refresh endpoint first."

### Group Mode

```yaml
approval:
  rejection_action: group
```

When a spec is rejected, chant converts it to a driver with member specs, distributing the acceptance criteria. Useful when the rejection is "this is too big, split it up."

## Finding Specs by Approval Status

```bash
# Specs waiting for review
$ chant list --approval pending

# Rejected specs that need attention
$ chant list --approval rejected

# Find specs where marcos participated
$ chant list --mentions marcos

# Specs with recent activity
$ chant list --activity-since 2h
```

In list output, approval status shows as colored indicators:

```
✓ 001-abc [approved]       Refactor auth           👤 sarah ↩ 1h 💬 2 ✓ marcos
⚠ 002-def [needs approval] Add rate limiting       👤 sarah ↩ 30m
✗ 003-ghi [rejected]       Rewrite session layer   👤 dave  ↩ 2h 💬 4
```

## Agent Co-Authorship Detection

When `require_approval_for_agent_work` is enabled, chant automatically adds an approval gate to any spec where the agent's commits include co-authorship signatures (`Co-Authored-By: Claude`, etc.):

```yaml
# .chant/config.md
approval:
  require_approval_for_agent_work: true
```

The workflow becomes:

```bash
$ chant work 001
# Agent completes work, commits include Co-Authored-By: Claude
# During finalization: "⚠ Agent co-authorship detected. Approval required before merge."

$ chant approve 001 --by marcos
# Now merge proceeds
```

This ensures a human reviews all agent-written code before it reaches main. Detected agents include Claude, GPT/ChatGPT, Copilot, and Gemini.

## Solo Developer Self-Review

Approval gates aren't just for teams. For risky changes, use them as a forced thinking checkpoint:

```bash
$ chant add "Migrate database schema" --needs-approval

# Come back after coffee, review with fresh eyes
$ chant show 001
# Think about rollback plan, edge cases, data loss risk

$ chant approve 001 --by me
$ chant work 001
```

## Emergency Bypass

When the approval process would cause unacceptable delays:

```bash
$ chant work 001 --skip-approval
```

Use sparingly. The bypass is visible in the execution log.

## Further Reading

- [Lifecycle](../concepts/lifecycle.md) — State transitions including approval gates
- [CLI Reference](../reference/cli.md) — Full command documentation
