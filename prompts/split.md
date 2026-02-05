---
name: split
purpose: Split a driver spec into members with detailed acceptance criteria
---

# Split Driver Specification into Member Specs

You are analyzing a driver specification for the {{project.name}} project and proposing how to split it into smaller, ordered member specs.

**IMPORTANT: This is an analysis task. Do NOT use any tools, do NOT explore the codebase, do NOT make any changes, do NOT commit anything. ONLY output text in the exact format specified below.**

## Driver Specification to Split

**ID:** {{spec.id}}
**Title:** {{spec.title}}

{{spec.description}}

## Your Task

You will complete this analysis in THREE PHASES:

### Phase 1: Dependency Analysis

Before generating any member specs, analyze the work described in the driver spec:

1. **Identify logical tasks** - List all distinct pieces of work needed
2. **For each task, determine:**
   - **Inputs:** What information, types, functions, or infrastructure does this task need?
   - **Outputs:** What does this task produce? (functions, types, configs, modules, etc.)
   - **Shared types/interfaces:** What data structures or contracts are used across tasks?
3. **Categorize tasks:**
   - **Infrastructure:** Logging, config, types, error handling, utilities (should have minimal dependencies)
   - **Features:** Business logic, user-facing functionality (often depend on infrastructure)
   - **Integration:** Wiring, main entry points (usually depend on features)

### Phase 2: DAG Construction

Based on the dependency analysis:

1. **Determine actual dependencies** - For each task, justify which other tasks it depends on
2. **Identify parallel tasks** - Which tasks can execute simultaneously after their dependencies?
3. **Detect ordering errors** - Warn if infrastructure tasks depend on feature tasks
4. **Build the DAG** - Show the dependency graph structure (not a linear chain unless truly necessary)

### Phase 3: Generate Member Specs

For each task identified in Phase 1-2:

1. Create a member spec with:
   - A descriptive title (5-15 words) that describes the specific change, not just the category
     - **Good:** "Add unit tests for user authentication flow"
     - **Good:** "Implement error handling for invalid configuration files"
     - **Bad:** "Add tests" (too terse, not specific)
     - **Bad:** "Implement error handling" (missing context on what/where)
   - Description of what should be implemented
   - Explicit acceptance criteria with checkboxes for verification
   - Edge cases that should be considered
   - Example test cases where applicable
   - List of affected files (if identifiable from the spec)
   - Clear "done" conditions that can be verified
   - **Inherited Context** section (constraints/principles from parent)
   - **Provides** section (if this task produces shared functions/types)
   - **Requires** section (if this task consumes outputs from other tasks)
2. Ensure each member:
   - Leaves code in a compilable state
   - Is independently testable and valuable
   - Respects common patterns (add new alongside old → update callers → remove old)

## Complexity Thresholds (Linting-Aware)

Each resulting member spec should meet these thresholds to pass linting:
- **Acceptance Criteria:** ≤ 5 items (allows haiku to verify completion)
- **Target Files:** ≤ 5 files (keeps scope focused, minimal coupling)
- **Description Length:** ≤ 200 words (haiku-friendly, clear intent)

These thresholds ensure the split produces specs that are:
- **Independently executable** by Claude Haiku
- **Verifiable** with clear, specific acceptance criteria
- **Self-contained** without cross-references

## Why Thorough Acceptance Criteria?

These member specs will be executed by Claude Haiku, a capable but smaller model. A strong model (Opus/Sonnet) doing the split should think through edge cases and requirements thoroughly. Each member must have:

- **Specific checkboxes** for each piece of work (not just "implement it")
- **Edge case callouts** to prevent oversights
- **Test scenarios** to clarify expected behavior
- **Clear success metrics** so Haiku knows when it's done
- **Within complexity thresholds** so the spec stays manageable for haiku

This way, Haiku has a detailed specification to follow and won't miss important aspects.

## Cross-References and Interface Contracts

When member specs share functionality:
- **Use explicit contracts** - If spec A creates a function spec B uses, define the contract
- **Producer specs** include `## Provides` section with signatures (name, params, return type, errors)
- **Consumer specs** include `## Requires` section referencing the contract
- **Format:** "Uses `ConfigType` from Member 2" or "Calls `detect()` from Member 3"
- **No spec ID references in main description** - only in Requires/Provides sections

This makes dependencies explicit while keeping specs self-contained.

## Context Inheritance

Extract and propagate parent spec's constraints to members:
- Identify sections like: "Constraints", "Design Principles", "Out of Scope", "What X Cannot Do"
- Add `## Inherited Context` section to each member with a concise summary
- Don't duplicate verbatim - summarize relevant constraints for each member

## Output Format

**CRITICAL: You MUST output in TWO SECTIONS:**

### Section 1: Dependency Analysis (Required)

Start with `# Dependency Analysis` and include:

```markdown
# Dependency Analysis

## Tasks Identified

1. **Task Name** (Infrastructure/Feature/Integration)
   - Inputs: What this needs
   - Outputs: What this produces
   - Shared types: Any contracts or interfaces

2. **Task Name** (Infrastructure/Feature/Integration)
   - Inputs: What this needs
   - Outputs: What this produces
   - Shared types: Any contracts or interfaces

...

## Dependency Graph

```
Task 1 (infra)
   ├── Task 2 (feature)
   ├── Task 3 (feature)
   │      │
   │      v
   └───> Task 4 (integration) <─── Task 2
```

**Justification:**
- Task 2 depends on Task 1 because: [reason]
- Task 3 depends on Task 1 because: [reason]
- Task 4 depends on Tasks 2 and 3 because: [reason]
- Tasks 2 and 3 can run in parallel after Task 1
```

### Section 2: Member Specs (Required)

After the dependency analysis, output member specs in this format:

```markdown
## Member 1: <title>

<description of what this member accomplishes>

### Inherited Context

- Constraint/principle from parent spec
- Another relevant constraint

### Acceptance Criteria

- [ ] Specific criterion 1
- [ ] Specific criterion 2
- [ ] Specific criterion 3

### Edge Cases

- Edge case 1: Describe what should happen and how to test it
- Edge case 2: Describe what should happen and how to test it

### Example Test Cases

For this feature, verify:
- Case 1: Input X should produce Y
- Case 2: Input A should produce B

### Provides

*Include this section if this member produces shared functionality:*
- `function_name(params) -> return_type` - Description and error cases
- `TypeName` - Description of the type

### Requires

*Include this section if this member needs outputs from other members:*
- Uses `TypeName` from Member 2
- Calls `function_name()` from Member 3

**Affected Files:**
- file1.rs
- file2.rs

**Dependencies:** Member 2, Member 3 *(list member numbers this depends on)*

## Member 2: <title>

... (continue with same format)
```

**Important notes:**
- Omit sections that don't apply (Provides, Requires, Inherited Context if none)
- If no files identified, omit Affected Files
- If no dependencies, set Dependencies to "None"
- Create as many members as needed (typically 3-7 for a medium spec)

**Remember: Output BOTH sections - Dependency Analysis first, then Member Specs. No preamble, no "I will create..." statements.**
