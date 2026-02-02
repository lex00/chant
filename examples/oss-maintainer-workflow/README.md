# OSS Maintainer Workflow Example

## Overview

This example demonstrates a 6-phase research-driven bug fix workflow for open source maintainers, showing how to systematically investigate and fix a complex concurrent write data loss bug (issue #42). The workflow uses sequential chained specs to build understanding before implementing fixes, reducing wasted effort and building confidence through structured research.

## Structure

Six sequential specs forming a chain execution pattern:

1. **001-comprehension.md** - Understand what the issue is about before jumping to conclusions
2. **002-reproduction.md** - Create a failing test or reproduction instructions
3. **003-root-cause.md** - Determine what needs to be fixed through RCA
4. **004-sprawl.md** - Expand investigation to assess impact and scope
5. **005-fork-fix.md** - Implement the fix and create a staging PR in fork
6. **006-upstream-pr.md** - Human gate before creating the real upstream PR

Research outputs in `.chant/research/`:
- `issue-42-comprehension.md` - Output from phase 1
- `issue-42-root-cause.md` - Output from phase 3
- `issue-42-sprawl.md` - Output from phase 4

Each phase uses `informed_by:` to reference previous research, creating a chain of dependencies.

## Usage

Execute the sequential workflow:
```bash
cd examples/oss-maintainer-workflow

# Run phases in order
chant work 001  # Phase 1: Comprehension
chant work 002  # Phase 2: Reproduction
chant work 003  # Phase 3: Root Cause Analysis
chant work 004  # Phase 4: Sprawl
chant work 005  # Phase 5: Fork Fix
# Phase 6: Read 006-upstream-pr.md for human review checklist
```

Or use dependencies to enforce order:
```bash
chant work 005  # Will error if phases 1-4 not completed due to depends_on
```

## Testing

Test the workflow by:
1. Running phase 1 to see comprehension research output
2. Running phase 2 to see failing test created
3. Running phase 3 to see root cause analysis
4. Running phase 4 to see impact assessment
5. Running phase 5 to see fix implementation
6. Reviewing phase 6 checklist for human gate

Verify the chain pattern:
- Each phase builds on previous phases through `informed_by:`
- Research specs (1, 3, 4) write to `.chant/research/` using `target_files:`
- Implementation specs (2, 5) reference research outputs
- Phase 6 documents human decision point

Outcomes demonstrate:
- Bug understood through systematic research
- Failing test created before fixing
- Root cause identified and documented
- Impact assessed (found 2 other similar patterns)
- Fix implemented and tested
- Human gate ensures quality before upstream PR
