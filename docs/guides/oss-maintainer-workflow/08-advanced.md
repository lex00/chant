# Advanced Patterns

## Working on Fix Branches

If you're developing the fix on a dedicated branch instead of main, configure chant to merge worktree results there:

```yaml
# .chant/config.md
defaults:
  main_branch: "fix/issue-1234"
```

Now `chant work` merges completed specs into your fix branch rather than main.

## Pausing and Taking Over

### Pausing Work

If you need to stop an agent mid-investigation -- say, you realize it needs information you haven't provided -- pause it:

```bash
$ chant pause 005
Paused spec 005-k9w. Agent stopped, progress preserved.
```

The spec moves to paused status. Resume later with `chant work 005`.

### Taking Over

If the agent is heading in the wrong direction, take over the spec entirely:

```bash
$ chant takeover 005
Stopping agent for 005-k9w...
Analyzing execution log...

Progress summary:
- Tested 3 hypotheses (all eliminated)
- Currently investigating filesystem cache coherency
- No root cause identified yet

Spec updated with progress notes and suggested next steps.
```

Takeover stops the agent, reads its log, and updates the spec body with a summary of what was accomplished and what remains. You can then fix the approach manually or edit the spec and re-run `chant work 005`.

## Single-Spec Investigation Mode

The full six-phase workflow provides excellent auditability and enables handoffs between people. But for a solo investigator working one issue in a single session, six specs can feel ceremonial.

Single-spec mode consolidates the research phases into one document with stage markers:

```bash
$ chant add "Investigation: issue #1234 concurrent write data loss"
Created spec: 2026-02-08-001-abc
```

```yaml
---
type: research
labels: [investigation, issue-1234]
target_files:
  - .chant/research/issue-1234-investigation.md
---
```

The target file uses stage markers to organize findings:

```markdown
# Investigation: Issue #1234

## Stage 1: Comprehension
[Issue summary, affected components, initial observations]

## Stage 2: Reproduction
[Failing test, reproduction steps, environment details]

## Stage 3: Root Cause
[Hypothesis table, root cause identification, evidence]

## Stage 4: Impact Map
[Affected components, similar patterns, test gaps]

## Summary
Root Cause: Unprotected read-modify-write in store.rs:145
Fix Strategy: Pessimistic locking using existing Lock module
Target Files: src/storage/store.rs, src/storage/batch.rs
```

The agent completes all four research stages in one pass. When it finishes, you create a single implementation spec referencing the investigation output:

```bash
$ chant add "Fix issue #1234: add locking to concurrent writes"
Created spec: 2026-02-08-002-def
```

### When to Use Each Mode

**Use the full six-spec workflow when:**
- Multiple people will work the issue (one person does comprehension, another picks up root cause)
- Investigation spans multiple days or sessions (each spec is a resumable checkpoint)
- The issue is complex: multiple hypotheses, more than two affected files, or unclear reproduction steps

**Use single-spec mode when:**
- You're the only person working the issue, in a single sitting
- You can describe the bug and likely fix direction in a paragraph
- The fix will touch one or two files with a clear test strategy

## Investigation Heuristics

Across both modes, watch for signs that your investigation approach needs adjustment:

- **Hypotheses aren't converging.** Multiple theories tested, all eliminated, and new ones don't build on previous findings. Broaden your search to adjacent modules.
- **Stuck in one file.** Re-reading the same code repeatedly. Look at callers, dependencies, and configuration instead.
- **Reproduction keeps failing.** Your test may not match the actual reported symptoms. Re-read the issue with fresh eyes.

The goal is deliberate re-orientation when progress stalls -- not premature abandonment or endless persistence on an unproductive path.
