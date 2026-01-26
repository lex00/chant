# Audit Report: Plan Mode Confusion in CLAUDE.md

## Executive Summary

During parallel spec execution, the agent working on spec `01l-y81` (Document MCP error codes and response structures) repeatedly attempted to use `ExitPlanMode` instead of executing the actual implementation work.

**Root Cause**: CLAUDE.md lacks explicit guidance that agents should NOT use plan mode during spec execution, creating a gap between general Claude Code system instructions and the chant-specific workflow.

**Impact**: This confusion can cause:
- Inefficiency during spec execution
- Potential infinite loops in plan mode
- Problems with parallel spec execution
- Agent confusion about execution flow

---

## Analysis

### The Core Problem

Agents receive conflicting guidance:

1. **From Claude Code system prompt**:
   > "Use this tool proactively when you're about to start a non-trivial implementation task... Getting user sign-off on your approach before writing code prevents wasted effort."

2. **From CLAUDE.md**:
   - Explains that specs define work intentions
   - Instructs agents to read files, implement, test, and lint
   - **Never explicitly states**: "Do not use EnterPlanMode during spec execution"

3. **Agent's reasoning**:
   ```
   "This is non-trivial code work" → 
   "System prompt says use plan mode for non-trivial work" →
   "CLAUDE.md doesn't forbid it" →
   "I should enter plan mode to design my approach"
   ```

### Why This Is Wrong

Specs in chant are **pre-planned documents**:
- Spec creation is where planning happens (before `chant work`)
- Specs contain explicit acceptance criteria defining the requirements
- `chant work` invokes agents in **execution mode**, not planning mode
- Plan mode tools (EnterPlanMode/ExitPlanMode) are for creating/refining specs, not executing them

### The Inconsistency

| Context | What System Prompt Says | What CLAUDE.md Says | What CLAUDE.md Should Say |
|---------|-------------------------|-------------------|---------------------------|
| Creating a spec | Use plan mode for complex work | Create a spec with `chant add` | "Plan and document your approach in the spec" |
| Executing a spec | Use plan mode for non-trivial tasks | Execute per acceptance criteria | **"Do NOT use plan mode; the spec IS your plan"** |

---

## Problematic Sections of CLAUDE.md

### 1. Primary Rules (Lines 7-52)

**Current Text (Rule 3)**:
```markdown
### 3. Always Use a Spec for Every Operation

Even small changes require a spec. This ensures:
- All work is documented and auditable
- Changes are executed in isolated worktrees
- Work can be reviewed, rejected, or modified
- History is maintained in git
```

**Problem**: Doesn't clarify the execution model. Agents don't know if specs are just requirements documents or if they're meant to guide execution.

**Missing Context**: That when a spec is executed (`chant work`), the agent is already in execution mode and should not attempt planning.

---

### 2. Spec Format and Patterns (Lines 142-176)

**Current Text**:
The section describes frontmatter and spec types but doesn't explain the agent's role during execution.

**Problem**: No mention of how agents should approach executing a spec vs. creating one.

---

### 3. Important Constraints (Lines 178-202)

**Current Text** (excerpt):
```markdown
### For AI Agents Working on Specs

1. **Read before modifying** - Always read relevant files first...
2. **Write tests** - Validate behavior with tests...
3. **Lint everything** - Always run `just lint`...
```

**Problem**: Lists execution steps but never says "do NOT use plan mode."

**Impact**: An agent executing a "code" spec type (the most complex kind) would naturally think plan mode is appropriate, but it isn't.

---

### 4. Instructions Block (Spec Template)

**Current Text** (from spec headers):
```markdown
## Instructions

1. **Read** the relevant code first
2. **Plan** your approach before coding
3. **Implement** the changes
4. **Run `cargo fmt`** to format the code
...
```

**Problem**: Step 2 "Plan your approach" is ambiguous:
- Does it mean "think through mentally"?
- Or does it mean "use EnterPlanMode"?
- The spec itself is already the plan, so this creates ambiguity

---

## Recommended Fixes

### Fix 1: Add Explicit Guidance to Primary Rules (CRITICAL)

**Insert after Rule 3**, a new clarification:

```markdown
### Spec Execution Model

When a spec is created and ready for execution (`chant work <spec-id>`):
- **The spec IS the plan** - it contains acceptance criteria and requirements
- **Agents are in execution mode** - implement directly per the acceptance criteria
- **Do NOT use EnterPlanMode or ExitPlanMode** - these tools are for spec creation/refinement, not execution
- **Execute the work** according to the documented acceptance criteria

Plan mode tools should only be used by users/agents when CREATING or MODIFYING specs, not when executing them.
```

**Location**: After line 52 (after Rule 3)

---

### Fix 2: Clarify the "Plan your approach" Instruction

**Current** (from spec template):
```markdown
2. **Plan** your approach before coding
```

**Change to**:
```markdown
2. **Understand** the acceptance criteria and implementation scope (the spec is your plan - review it carefully before implementing)
```

**Alternative** (if you want to keep "plan"):
```markdown
2. **Plan** your approach mentally before coding (do NOT use EnterPlanMode - the spec's acceptance criteria define the scope)
```

---

### Fix 3: Add to Important Constraints Section

**Insert as new first constraint** (line 180):

```markdown
1. **Do NOT use plan mode** - Specs are pre-planned documents with acceptance criteria. 
   Execute them directly without using EnterPlanMode or ExitPlanMode tools. 
   These tools are for spec creation, not execution.
```

**Then renumber existing constraints 1-7 to 2-8**

---

### Fix 4: Expand the Workflow Section

**Current** (lines 54-62):
```markdown
## Workflow

When asked to implement something:

1. **Create a spec** with `just chant add "description of the task"`
2. **Work the spec** with `just chant work <spec-id>` (or let the spec system do it)
3. **Review the result** and check acceptance criteria

The spec system handles all file modifications, testing, and git management.
```

**Enhanced version**:
```markdown
## Workflow

### Creating Specifications

When asked to implement something:

1. **Create a spec** with `just chant add "description of the task"`
   - Use plan mode if needed to design the approach
   - Document acceptance criteria in the spec
2. **Refine the spec** if needed before execution
3. **Review the spec** to ensure requirements are clear

### Executing Specifications

When executing a spec with `chant work <spec-id>`:

1. **Execute directly** - The spec IS the plan; implement per acceptance criteria
2. **Do NOT use plan mode** - EnterPlanMode/ExitPlanMode are for spec creation, not execution
3. **Review the result** and verify all acceptance criteria are met

The spec system handles all file modifications, testing, and git management.
```

---

### Fix 5: Add a New "Spec Execution Guidelines" Section

**Insert new section** after "Spec Format and Patterns" and before "Important Constraints":

```markdown
## Spec Execution Guidelines

### Understanding the Execution Model

Chant uses a two-phase workflow:

1. **Specification Phase** (outside execution)
   - User or agent creates/refines a spec using plan mode if needed
   - Spec documents acceptance criteria and requirements
   - Spec is saved and marked as "ready"

2. **Execution Phase** (during `chant work`)
   - Agent receives spec as execution context
   - Spec IS the plan - contains all requirements upfront
   - Agent executes directly per acceptance criteria
   - Plan mode tools are NOT appropriate here

### Why Plan Mode is NOT Used During Execution

- **Specs are pre-planned**: The spec document contains the planning results
- **Requirements are explicit**: Acceptance criteria define the scope
- **Execution is direct**: Implement according to spec requirements
- **Plan mode creates loops**: Trying to plan when the plan exists creates confusion

### What to Do During Spec Execution

Instead of entering plan mode:
1. Read and understand the acceptance criteria
2. Read the relevant source code
3. Design your implementation mentally (don't enter plan mode)
4. Implement the changes
5. Test and verify against acceptance criteria
6. Commit and mark spec complete

This distinction is crucial for reliable, efficient spec execution.
```

---

## Summary of Changes Needed

| Priority | Section | Issue | Fix |
|----------|---------|-------|-----|
| CRITICAL | Primary Rules | No guidance on plan mode during execution | Add "Spec Execution Model" clarification after Rule 3 |
| HIGH | Instructions Block | Ambiguous "plan your approach" | Clarify it means understand (not EnterPlanMode) |
| HIGH | Important Constraints | Doesn't forbid plan mode | Add constraint 1: "Do NOT use plan mode" |
| HIGH | Workflow | Doesn't explain execution vs. creation | Enhance to show both phases |
| MEDIUM | (New Section) | No dedicated guidance on execution model | Add "Spec Execution Guidelines" section |

---

## Verification

These changes would be verified by:

1. ✅ Reading the updated CLAUDE.md shows clear guidance that plan mode should not be used during spec execution
2. ✅ New agents executing specs understand the execution model upfront
3. ✅ No more plan mode confusion during parallel spec execution
4. ✅ Better alignment between general Claude Code instructions and chant-specific workflow

---

## Related Specs

Implementation of these fixes should be tracked in a separate spec (e.g., `just chant add "Update CLAUDE.md with plan mode execution guidance"`), as this audit is investigative in nature.

