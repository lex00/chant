# Advanced Prompting Guide

This guide shows how to achieve sophisticated workflows through prompts rather than built-in features. Chant provides primitives; prompts provide behavior.

## Philosophy

Other tools build workflows *into* the product. Chant builds workflows *with* prompts.

| Approach | Example |
|----------|---------|
| Built-in wizard | "Run `/discover` to start the 5-question flow" |
| Prompt-based | "Use the `spec-review` prompt to improve your draft" |

The prompt approach is:
- **Flexible** - Modify the prompt, change the behavior
- **Transparent** - Read the prompt, understand what happens
- **Composable** - Combine prompts for complex workflows
- **Evolvable** - Improve prompts as you learn what works

## Specification Discovery

### The Problem

Vague specs produce poor results. "Improve the API" gives agents no direction.

### The Prompt Solution

Instead of a built-in wizard, use prompts that help refine specifications.

#### spec-critique Prompt

Reviews a draft spec and identifies gaps:

```markdown
# .chant/prompts/spec-critique.md

You are a specification reviewer. Analyze the spec and identify:

## Gaps to Address

1. **Ambiguity** - What decisions are left unstated?
2. **Scope** - Is this bounded or open-ended?
3. **Verification** - How will we know it's done?
4. **Dependencies** - What must exist first?
5. **Risks** - What could go wrong?

## Output Format

For each gap found:
- State the gap
- Ask a clarifying question
- Suggest a default if the user doesn't answer

## Complexity Assessment

Rate this spec:
- [ ] Quick (1-2 files, clear change)
- [ ] Standard (3-5 files, some decisions)
- [ ] Complex (6+ files, architectural impact)

If Complex, recommend decomposition.

## Do Not

- Implement anything
- Make assumptions without flagging them
- Add scope beyond what's described
```

**Usage:**

```bash
# Review a draft spec
chant work 001 --prompt spec-critique --dry-run

# Or as a pre-work hook
# config.md
hooks:
  pre_work:
    prompt: spec-critique
    require_approval: true
```

#### spec-expand Prompt

Expands a brief idea into a full specification:

```markdown
# .chant/prompts/spec-expand.md

You are a specification writer. Given a brief idea, produce a complete spec.

## Process

1. **Understand** - What is the user trying to achieve?
2. **Research** - Read relevant existing code
3. **Specify** - Write detailed requirements
4. **Bound** - Define what's NOT in scope
5. **Verify** - Write acceptance criteria

## Output Format

```yaml
---
status: pending
labels: [generated-spec]
target_files:
  - path/to/file.go
depends_on: []
---

# [Clear, action-oriented title]

## Context

[Why this spec exists. Link to existing code.]

## Requirements

1. [Specific requirement]
2. [Specific requirement]
3. [Specific requirement]

## Not In Scope

- [Explicitly excluded]

## Acceptance Criteria

- [ ] [Verifiable criterion]
- [ ] [Verifiable criterion]
- [ ] Tests pass
- [ ] No lint errors
```

## Adaptive Depth

Ask clarifying questions based on complexity:

**Quick specs (1-2 questions):**
- What's the exact change?
- Any constraints?

**Standard specs (3-4 questions):**
- What problem does this solve?
- What's the expected behavior?
- Any edge cases to handle?
- How should errors be handled?

**Complex specs (5+ questions):**
- What's the architectural context?
- What are the performance requirements?
- What are the security considerations?
- How does this integrate with existing systems?
- What's the migration/rollback strategy?
```

**Usage:**

```bash
# Start with a brief idea
echo "# Add rate limiting to the API" > .chant/tasks/draft.md

# Expand it
chant work draft --prompt spec-expand

# Review the expanded spec, edit as needed
chant edit draft

# When satisfied, rename to real spec
mv .chant/specs/draft.md .chant/specs/$(chant id).md
```

## Capturing Learnings (Evolve)

### The Problem

After completing work, insights are lost. The same patterns get rediscovered.

### The Prompt Solution

#### learnings Prompt

Analyzes completed work and captures reusable knowledge:

```markdown
# .chant/prompts/learnings.md

You are a knowledge curator. After work completes, analyze what was built and capture learnings.

## Analysis Steps

1. **Read the completed spec** - What was the intent?
2. **Read the changes** - What was actually built?
3. **Identify patterns** - What's reusable?

## What to Capture

### New Patterns
Code patterns that solved problems well:
- Name the pattern
- Show a minimal example
- Explain when to use it

### New Components
Reusable pieces created:
- What it does
- Where it lives
- How to use it

### Gotchas Discovered
Things that weren't obvious:
- What went wrong initially
- What the fix was
- How to avoid it next time

### Documentation Gaps
Things that should be documented:
- Missing README sections
- Undocumented APIs
- Tribal knowledge made explicit

## Output Format

Create or update: `LEARNINGS.md` in the relevant directory.

```markdown
# Learnings

## Patterns

### [Pattern Name]
**Added:** [date] | **From spec:** [id]

[Description and example]

## Components

### [Component Name]
**Location:** `path/to/component`
**Added:** [date] | **From spec:** [id]

[Usage example]

## Gotchas

### [Gotcha Title]
**Discovered:** [date] | **From spec:** [id]

[What happened and how to avoid]
```

## User Control

Present findings and ask:
- Apply all learnings?
- Select specific items?
- Skip this time?

Do not auto-commit. User must approve.
```

**Usage:**

```bash
# After completing a spec or epic
chant learn 001

# Or as post-completion hook
# config.md
hooks:
  post_complete:
    prompt: learnings
    optional: true  # User can skip
```

#### pattern-match Prompt

References captured learnings during new work:

```markdown
# .chant/prompts/pattern-match.md

Before implementing, check for relevant learnings.

## Process

1. Read the spec requirements
2. Search LEARNINGS.md files in relevant directories
3. Search completed specs with similar labels
4. Identify applicable patterns, components, gotchas

## Output

If matches found:
```
Relevant learnings for this spec:

**Pattern: [Name]**
From: spec 042
Applicable because: [reason]
[Brief example]

**Gotcha: [Title]**
From: spec 089
Watch out for: [warning]

**Component: [Name]**
Location: src/utils/[name].go
Consider reusing for: [reason]
```

If no matches: proceed without comment.

## Integration

This prompt should be composed with implementation prompts:

```yaml
# config.md
prompts:
  default: standard
  compose:
    - pattern-match  # Check learnings first
    - standard       # Then implement
```
```

## Test-Driven Development

### The Problem

Tests written after implementation often miss edge cases.

### The Prompt Solution

TDD is a workflow pattern, not a feature. Implement via spec structure + prompts.

#### Spec Structure for TDD

```yaml
# Parent spec
---
status: pending
labels: [tdd]
---
# Add user validation

## Children (TDD order)

1. 001.1 - Write validation tests (RED)
2. 001.2 - Implement validation (GREEN) - depends_on: 001.1
3. 001.3 - Refactor (REFACTOR) - depends_on: 001.2
```

#### tdd-red Prompt

Writes failing tests first:

```markdown
# .chant/prompts/tdd-red.md

You are writing tests BEFORE implementation. The code under test does not exist yet.

## Rules

1. **Tests must fail** - You're defining expected behavior
2. **No implementation** - Only test files
3. **Cover edge cases** - Think about what could go wrong
4. **Clear assertions** - Each test tests one thing

## Process

1. Read the driver spec's requirements
2. Identify testable behaviors
3. Write test cases that will fail
4. Commit the failing tests

## Output

Test files only. Implementation comes in the next spec.

## Verification

After committing, run tests. They MUST fail.
If any test passes, you wrote it wrong (testing existing behavior).
```

#### tdd-green Prompt

Implements minimum code to pass tests:

```markdown
# .chant/prompts/tdd-green.md

You are implementing code to make failing tests pass.

## Rules

1. **Minimum code** - Only what's needed to pass tests
2. **No new features** - If it's not tested, don't build it
3. **No premature optimization** - Make it work first
4. **Run tests frequently** - Verify progress

## Process

1. Run tests, see them fail
2. Implement the simplest solution
3. Run tests, see them pass
4. Commit

## Do Not

- Add functionality beyond what tests require
- Refactor (that's the next phase)
- Add tests (that was the previous phase)
```

#### tdd-refactor Prompt

Improves code while keeping tests green:

```markdown
# .chant/prompts/tdd-refactor.md

You are improving code quality. Tests must stay green.

## Rules

1. **Tests stay green** - Run after every change
2. **Small steps** - One refactor at a time
3. **No new behavior** - Refactor, don't extend

## Refactoring Targets

- Duplicate code → Extract function
- Long function → Split
- Magic values → Named constants
- Complex conditional → Simplify or extract
- Poor names → Rename

## Process

1. Identify improvement
2. Make change
3. Run tests
4. If green, commit
5. If red, revert and try smaller change
```

**Usage:**

```bash
# Create TDD spec structure
chant add "Add user validation" --tdd

# This creates:
# 001.md (driver)
# 001.1.md (red) - prompt: tdd-red
# 001.2.md (green) - prompt: tdd-green, depends_on: 001.1
# 001.3.md (refactor) - prompt: tdd-refactor, depends_on: 001.2

# Work executes in order due to dependencies
chant work 001 --all
```

## Workflow Composition

### The Problem

Fixed pipelines don't fit all projects. init→discover→approve→run→retro is one workflow, not THE workflow.

### The Prompt Solution

Build workflows from composable pieces.

#### Workflow Building Blocks

| Block | Purpose | Prompt |
|-------|---------|--------|
| Ideation | Capture rough ideas | `capture` |
| Specification | Refine into specs | `spec-expand`, `spec-critique` |
| Decomposition | Break into subspecs | `split` |
| Implementation | Do the work | `standard`, `tdd-*` |
| Review | Check quality | `review` |
| Learning | Capture insights | `learnings` |
| Retrospective | Analyze patterns | `retro` |

#### Example: Feature Workflow

```yaml
# .chant/workflows/feature.md (documentation, not config)

## Feature Development Workflow

1. **Capture** - Quick spec with rough idea
   ```bash
   chant add "Rate limiting for API" --label idea
   ```

2. **Specify** - Expand into full spec
   ```bash
   chant work 001 --prompt spec-expand
   chant edit 001  # Review and refine
   ```

3. **Decompose** - Break into subspecs
   ```bash
   chant work 001 --prompt split
   # Creates 001.1, 001.2, etc.
   ```

4. **Implement** - Execute subspecs
   ```bash
   chant work 001 --all
   ```

5. **Learn** - Capture insights
   ```bash
   chant learn 001
   ```
```

#### Example: Bug Fix Workflow

```yaml
## Bug Fix Workflow

1. **Reproduce** - Document the bug
   ```bash
   chant add "Fix: Users can't login with email containing +" --label bug
   ```

2. **Test First** - Write failing test
   ```bash
   chant work 001 --prompt tdd-red
   ```

3. **Fix** - Implement fix
   ```bash
   chant work 001 --prompt tdd-green
   ```

4. **Verify** - Confirm fix doesn't regress
   ```bash
   chant verify 001
   ```
```

#### Example: Exploration Workflow

```yaml
## Exploration Workflow (no implementation)

1. **Question** - What do we want to learn?
   ```bash
   chant add "Explore: How should we handle webhooks?" --label exploration
   ```

2. **Research** - Agent investigates
   ```bash
   chant work 001 --prompt explore
   # Prompt instructs: read code, summarize options, don't implement
   ```

3. **Decide** - Human reviews findings, creates implementation spec
   ```bash
   chant add "Implement webhook handling using Option B from 001"
   ```
```

#### Composing Prompts

Chain prompts for complex behaviors:

```yaml
# config.md
prompts:
  # Simple composition: run in sequence
  feature:
    compose:
      - pattern-match    # Check learnings
      - standard         # Implement
      - self-review      # Check own work

  # Conditional composition
  careful:
    compose:
      - pattern-match
      - standard
    post:
      - test-coverage    # Verify coverage
      - security-scan    # Check for issues
```

## Retrospective Analysis

### The Problem

Repeated failures indicate systemic issues. Without analysis, you fix symptoms not causes.

### The Prompt Solution

#### retro Prompt

Analyzes recent work for patterns:

```markdown
# .chant/prompts/retro.md

You are analyzing completed work to identify patterns and improvements.

## Data Sources

1. Recent completed specs (last 20 or specified range)
2. Failed specs and their causes
3. Spec durations and costs
4. Drift detection results

## Analysis

### Efficiency Metrics

- **Completion rate**: completed / (completed + failed)
- **Retry rate**: specs requiring multiple attempts
- **Avg duration**: mean time from start to complete
- **Cost efficiency**: avg cost per spec

### Failure Patterns

Group failures by cause:
- Test failures (which tests, which code)
- Merge conflicts (which files)
- Timeout (which specs take too long)
- Scope creep (specs that grew during execution)

### Success Patterns

What do successful specs have in common?
- Clear acceptance criteria?
- Specific target files?
- Certain labels?
- Certain size?

### Recommendations

Based on patterns:
1. **Process improvements** - What workflow changes would help?
2. **Prompt improvements** - What prompt guidance is missing?
3. **Spec quality** - What makes specs succeed or fail?

## Output Format

```markdown
# Retrospective: [Date Range]

## Summary
- Completed: X specs
- Failed: Y specs
- Efficiency: Z%

## Failure Patterns

### [Pattern 1]: [X occurrences]
**Tasks:** 001, 005, 012
**Common factor:** [what they share]
**Recommendation:** [how to prevent]

### [Pattern 2]: [Y occurrences]
...

## Success Patterns

Tasks with [characteristic] succeeded N% more often.
Consider: [recommendation]

## Action Items

- [ ] [Specific improvement]
- [ ] [Specific improvement]
```

## Do Not

- Blame individuals
- Recommend massive process changes
- Ignore small patterns (they compound)
```

**Usage:**

```bash
# Retrospective on recent work
chant retro

# Retrospective on specific epic
chant retro --epic 001

# Retrospective on date range
chant retro --since 2026-01-01 --until 2026-01-15

# Save to file
chant retro --output .chant/retros/2026-01-22.md
```

#### retro-apply Prompt

Turns retrospective insights into improvements:

```markdown
# .chant/prompts/retro-apply.md

Given a retrospective report, create actionable improvements.

## For Each Recommendation

1. **Assess impact** - How much will this help?
2. **Assess effort** - How hard to implement?
3. **Prioritize** - High impact + low effort first

## Types of Improvements

### Prompt Improvements
Create or modify prompts to address patterns:
- If "tests often missed edge cases" → improve test prompts
- If "scope creep common" → add scope-check to prompts

### Process Improvements
Suggest workflow changes:
- If "complex specs fail more" → recommend decomposition threshold
- If "certain files cause conflicts" → recommend locking or sequencing

### Documentation Improvements
Capture learnings:
- Add to LEARNINGS.md
- Update CONTRIBUTING.md
- Add gotchas to relevant READMEs

## Output

For each actionable item:
```yaml
---
status: pending
labels: [process-improvement]
---
# [Improvement title]

**From retro:** [date]
**Pattern:** [what was observed]
**Improvement:** [what to do]

## Acceptance Criteria
- [ ] [Measurable outcome]
```
```

## Putting It All Together

### Prompt Library Structure

```
.chant/prompts/
├── standard.md          # Default implementation
├── spec-critique.md     # Review specifications
├── spec-expand.md       # Expand brief ideas
├── split.md         # Break into subspecs
├── tdd-red.md           # Write failing tests
├── tdd-green.md         # Implement to pass
├── tdd-refactor.md      # Improve code quality
├── learnings.md         # Capture insights
├── pattern-match.md     # Reference learnings
├── retro.md             # Analyze patterns
├── retro-apply.md       # Act on insights
├── explore.md           # Research without implementing
├── review.md            # Quality check
└── autonomous.md        # Unattended execution
```

### Configuration for Composition

```yaml
# config.md
prompts:
  # Default prompt
  default: standard

  # Composed prompts
  careful:
    compose: [pattern-match, standard, self-review]

  tdd:
    compose: [tdd-red, tdd-green, tdd-refactor]

  # By label
  by_label:
    idea: spec-expand
    bug: tdd-red
    exploration: explore
    autonomous: autonomous

  # By phase
  phases:
    specify: spec-expand
    implement: standard
    review: review
    learn: learnings
```

### The Composability Principle

Chant doesn't build workflows as fixed features. Instead:

| Concept | Implementation |
|---------|----------------|
| Specification discovery | `spec-expand` + `spec-critique` prompts |
| Pattern evolution | `learnings` prompt + LEARNINGS.md |
| Test-driven development | `tdd-*` prompts + spec dependencies |
| Retrospective analysis | `retro` prompt + reports |
| Workflow orchestration | User-composed from primitives |

**The advantage:** When your workflow doesn't fit, change the prompt. No waiting for features. No fighting the tool.

## Intent Primitives

The building blocks that make this possible:

| Primitive | Purpose |
|-----------|---------|
| **Specs** | Executable specifications |
| **Prompts** | Agent behavior definitions |
| **Triggers** | Event-based activation |
| **Dependencies** | Execution ordering |
| **Verification** | Ongoing truth-checking |
| **Replay** | Self-correction mechanism |

These primitives compose into any workflow. The prompts in this guide are patterns built from primitives—not features locked into the tool.
