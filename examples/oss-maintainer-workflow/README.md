# OSS Maintainer Workflow Example

This example demonstrates a 6-phase research-driven bug fix workflow for open source maintainers. It shows how to use Chant to systematically investigate and fix a complex concurrent write data loss bug (issue #42).

## The 6-Phase Fork-Staging Pattern

### Phase 1: Comprehension
**Goal:** Understand what the issue is about before jumping to conclusions.

- Read the issue report and any discussion
- Document what the user is experiencing
- Identify what they expect vs. what actually happens
- Output: Research document capturing the problem statement

**Spec:** `001-comprehension.md`
- Uses `target_files:` pointing to `.chant/research/issue-42-comprehension.md`
- Agent reads issue, analyzes user reports, documents understanding

### Phase 2: Reproduction
**Goal:** Create a failing test or reproduction instructions.

- Write a test that demonstrates the bug
- If the bug can't be tested easily, create step-by-step reproduction instructions
- Verify the test/instructions actually show the problem
- Output: Failing test in the codebase

**Spec:** `002-reproduction.md`
- Uses `informed_by:` referencing phase 1 comprehension
- Uses `target_files:` for the test file
- Agent writes test that fails with the bug

### Phase 3: Root Cause Analysis
**Goal:** Determine what needs to be fixed.

- Run the failing test with debugging
- Trace through the code to find the actual bug
- Document the specific code causing the issue
- Output: Research document with RCA findings

**Spec:** `003-root-cause.md`
- Uses `informed_by:` referencing phases 1+2
- Uses `target_files:` for `.chant/research/issue-42-root-cause.md`
- Agent investigates and documents the root cause

### Phase 4: Sprawl
**Goal:** Expand investigation based on root cause.

- Check if the bug pattern exists elsewhere in the codebase
- Identify related systems that might be affected
- Assess impact and scope of the fix
- Output: Research document with impact analysis

**Spec:** `004-sprawl.md`
- Uses `informed_by:` referencing phase 3 RCA
- Uses `target_files:` for `.chant/research/issue-42-sprawl.md`
- Agent searches for similar patterns and documents scope

### Phase 5: Fork Fix
**Goal:** Implement the fix and create a staging PR.

- Fix the identified bug
- Make the test pass
- Create a PR in your fork for review
- Get feedback before touching upstream
- Output: Working fix + staging PR

**Spec:** `005-fork-fix.md`
- Uses `informed_by:` referencing phases 3+4 research outputs
- Agent implements fix based on RCA and sprawl analysis
- Fix makes phase 2 test pass

### Phase 6: Upstream PR
**Goal:** Human gate before creating the real PR.

- Review the fork PR
- Verify tests pass
- Check for edge cases or issues
- Create upstream PR only when confident
- Output: Documentation of human review checklist

**Spec:** `006-upstream-pr.md`
- Documents the human gate pattern
- Lists what to verify before opening upstream PR
- Not automated - requires human judgment

## The Bug: Concurrent Write Data Loss

**Issue #42:** When two processes write to the same key simultaneously, one write silently disappears.

**User Report:** "We're seeing intermittent data loss in production. When two API workers update the same user profile at the same time, sometimes only one update is persisted. No errors are logged."

**Root Cause:** `store.py` uses read-modify-write without locking. Between reading the value and writing it back, another process can write, causing the first write to clobber the second.

**Fix:** Use file locking or atomic operations to prevent race conditions.

## Directory Structure

```
examples/oss-maintainer-workflow/
├── README.md                          # This file
├── .chant/
│   ├── specs/
│   │   ├── 001-comprehension.md       # Phase 1: understand issue
│   │   ├── 002-reproduction.md        # Phase 2: create failing test
│   │   ├── 003-root-cause.md          # Phase 3: find the bug
│   │   ├── 004-sprawl.md              # Phase 4: expand investigation
│   │   ├── 005-fork-fix.md            # Phase 5: implement fix
│   │   └── 006-upstream-pr.md         # Phase 6: human gate
│   └── research/
│       ├── issue-42-comprehension.md  # Output from phase 1
│       ├── issue-42-root-cause.md     # Output from phase 3
│       └── issue-42-sprawl.md         # Output from phase 4
├── src/
│   └── storage/
│       └── store.py                   # Sample code with concurrency bug
└── tests/
    └── regression/
        └── test_issue_42.py           # Failing test from phase 2
```

## Why This Workflow?

Traditional bug fixing often jumps straight to "fix it" without understanding:
- What the user actually experienced
- Whether you can reproduce it
- What the real cause is
- How widespread the problem might be

This workflow:
- **Reduces wasted effort** - Don't fix the wrong thing
- **Builds confidence** - Research phases give you certainty
- **Stages risk** - Fork PR before upstream PR
- **Creates documentation** - Research outputs help future maintainers
- **Enables delegation** - Each phase is a discrete task for an agent

## Key Patterns Demonstrated

### Research Specs with `target_files:`
Phases 1, 3, and 4 are research specs that write findings to `.chant/research/`. Using `target_files:` tells the agent where to write output.

### Informed Chain with `informed_by:`
Each phase builds on previous phases:
- Phase 2 uses phase 1's comprehension
- Phase 3 uses phases 1+2
- Phase 4 uses phase 3's RCA
- Phase 5 uses phases 3+4 research

### Human Gates
Phase 6 documents a human decision point. Not everything should be automated. The upstream PR requires human judgment about timing, messaging, and readiness.

### Dependencies with `depends_on:`
Some specs have hard dependencies:
- Can't reproduce (phase 2) without comprehension (phase 1)
- Can't RCA (phase 3) without reproduction (phase 2)
- Can't implement fix (phase 5) without RCA (phase 3)

## Running This Example

```bash
cd examples/oss-maintainer-workflow

# Phase 1: Understand the issue
chant work .chant/specs/001-comprehension.md

# Phase 2: Create failing test
chant work .chant/specs/002-reproduction.md

# Phase 3: Root cause analysis
chant work .chant/specs/003-root-cause.md

# Phase 4: Assess impact
chant work .chant/specs/004-sprawl.md

# Phase 5: Implement fix
chant work .chant/specs/005-fork-fix.md

# Phase 6: Human review before upstream PR
# (Read 006-upstream-pr.md for checklist)
```

## Outcomes

- ✅ Bug understood through systematic research
- ✅ Failing test created before fixing
- ✅ Root cause identified and documented
- ✅ Impact assessed (found 2 other similar patterns)
- ✅ Fix implemented and tested
- ✅ Human gate ensures quality before upstream PR
