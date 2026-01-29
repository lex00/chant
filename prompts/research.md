---
name: research
purpose: Deep root cause analysis for issues
---

# Root Cause Analysis

You are conducting deep root cause analysis for an issue.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Your Goal

Thoroughly investigate WHY the issue exists, not just what to change:

1. **Understand the symptom:**
   - Read the reproduction spec (in `informed_by`)
   - Understand exactly what the failing test shows
   - Note the specific failure mode

2. **Investigate the codebase:**
   - Trace the code path from entry point to failure
   - Identify affected components and their relationships
   - Look for related code that might have similar issues
   - Check git history for context (when/why code was written)

3. **Identify the root cause:**
   - Find the exact location where incorrect behavior originates
   - Understand WHY the code behaves incorrectly
   - Distinguish symptoms from causes

4. **Evaluate approaches:**
   - Identify 2-3 potential fix approaches
   - Consider trade-offs for each (complexity, performance, risk)
   - Recommend one approach with clear justification

5. **Document findings:**
   - Write comprehensive RCA document
   - Include code references (file:line)
   - List all files that need modification
   - Document edge cases to consider

## Output

Create an RCA document at the target file location with:

1. **Executive Summary:** One paragraph explaining the root cause
2. **Root Cause:** Detailed explanation with code references
3. **Affected Components:** Table of files/modules and their impact
4. **Potential Approaches:** 2-3 approaches with pros/cons
5. **Recommendation:** Chosen approach with justification
6. **Files to Modify:** List for implementation spec
7. **Edge Cases:** Potential issues to handle

## Instructions

1. Read all `informed_by` references thoroughly
2. Explore the codebase extensively before forming conclusions
3. Use git blame to understand historical context
4. Be thorough - this analysis guides implementation
5. Mark acceptance criteria as complete in `{{spec.path}}`
6. Commit with message: `chant({{spec.id}}): <description>`

## Research Questions to Answer

- Where exactly does the bug occur? (file:line)
- What is the incorrect assumption or logic error?
- When was this code introduced and why?
- Are there similar patterns elsewhere with the same bug?
- What are the performance/complexity trade-offs of fixes?
- What edge cases might a fix need to handle?

## Constraints

- Do NOT implement a fix, only research and document
- Be thorough - incomplete research leads to incomplete fixes
- Include code snippets to illustrate the problem
- Always provide multiple approaches, even if one is clearly better
- Reference specific files and line numbers
