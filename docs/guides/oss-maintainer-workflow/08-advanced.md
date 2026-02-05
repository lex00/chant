# Advanced Patterns

## Working on Fix Branches

When working on a specific issue branch instead of main:

```yaml
# .chant/config.md
defaults:
  main_branch: "fix/issue-123"  # Target for merges
```

This lets you:
- Create specs for your fix
- Work in isolated worktrees
- Merge spec work into your fix branch (not main)

## Controlling Running Work

### Pausing Work

Stop a running agent without losing progress:

```bash
chant pause <spec-id>
```

The agent stops immediately and the spec status is set to `paused`. Use this when:
- You need to make a human decision before continuing
- The spec is blocked on external information
- You're taking a break and want to resume later

Resume with `chant work <spec-id>` or `chant resume <spec-id>`.

**Example:** You're running a research spec to evaluate libraries, but realize you need maintainer input on architectural constraints. Pause the spec, gather input, then resume.

### Taking Over Work

Pause and prepare a spec for manual continuation:

```bash
chant takeover <spec-id>
```

This command:
1. Pauses the running agent
2. Analyzes the execution log
3. Updates the spec with progress summary and next steps

Use takeover when:
- The agent is heading in the wrong direction
- You want to provide human guidance on how to proceed
- The work needs a different approach than the agent chose

**MCP integration:** The `chant_takeover` tool is available for agent-to-agent handoff scenarios.

**Example:** An implementation spec is repeatedly failing tests with the same approach. Take over, review what's been tried, and manually guide the next attempt or fix it yourself.
