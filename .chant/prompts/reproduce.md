---
name: reproduce
purpose: Create minimal reproduction case for a bug
---

# Create Reproduction Case

You are creating a reproduction case for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Reproduction Process

### 1. Understand the Bug

Before creating a reproduction:
- What is the reported buggy behavior?
- What conditions trigger it?
- What context is available (error messages, logs, environment)?

### 2. Identify Minimal Components

Determine what's needed to trigger the bug:
- Which features or code paths are involved?
- What data or state is required?
- What environment or configuration matters?
- What can be removed while still reproducing the issue?

### 3. Create Minimal Example

Build the smallest possible reproduction:

**Characteristics of a good reproduction:**
- **Minimal** — Only includes what's necessary to trigger the bug
- **Self-contained** — Can run independently with clear instructions
- **Focused** — Demonstrates one issue, not multiple
- **Documented** — Clear steps and expected vs actual behavior
- **Reproducible** — Consistently shows the bug when run

### 4. Output Format

Your reproduction case should include:

```markdown
# Reproduction: [Bug Name]

## Bug Description

**Expected behavior**: What should happen

**Actual behavior**: What actually happens

**Environment** (if relevant):
- OS: [if it matters]
- Version: [if it matters]
- Configuration: [if it matters]

## Reproduction Steps

### Setup

Any prerequisites or setup needed:
\```bash
# Setup commands
\```

### Steps to Reproduce

1. Step 1: [description]
   \```
   [code or commands]
   \```

2. Step 2: [description]
   \```
   [code or commands]
   \```

3. Step 3: [description]
   \```
   [code or commands]
   \```

### Expected Output

What should happen:
\```
[expected output]
\```

### Actual Output

What actually happens:
\```
[actual output or error]
\```

## Minimal Example

If applicable, a minimal code example:

\```rust
// Minimal reproduction code
fn main() {
    // Code that triggers the bug
}
\```

**To run:**
\```bash
# Commands to execute the reproduction
\```

## Analysis

Why this reproduces the bug:
- Key component involved: [description]
- Condition that triggers it: [description]
- Minimal requirements: [what can't be removed]

## Verification

This reproduction is:
- [ ] Minimal (removes unnecessary components)
- [ ] Self-contained (can run independently)
- [ ] Focused (demonstrates one issue)
- [ ] Documented (clear steps and expected behavior)
- [ ] Reproducible (consistently shows the bug)

## Notes

Any additional context:
- Related issues or discussions
- Variations that also reproduce
- Things that DON'T reproduce (helpful for narrowing scope)
```

### 5. Test the Reproduction

Verify your reproduction case:

1. **Run it yourself** - Confirm it triggers the bug
2. **Minimize it** - Remove anything unnecessary
3. **Document it** - Ensure steps are clear
4. **Verify consistency** - Run it multiple times to confirm

### 6. Verification Checklist

Before completing:

1. **Minimality check**: Can anything be removed while still reproducing?
2. **Clarity check**: Are steps clear enough for others to follow?
3. **Reproducibility check**: Does it consistently show the bug?
4. **Focus check**: Does it demonstrate one specific issue?
5. **Acceptance criteria check**: Does output meet all requirements?

## Constraints

- Focus on reproduction, not fixing (unless spec requires both)
- Make it as minimal as possible
- Ensure it's self-contained and runnable
- Document expected vs actual behavior clearly
- Include only necessary context and setup
- Test that it actually reproduces the issue consistently

## Instructions

1. **Understand** the reported bug and available context
2. **Identify** minimal components needed to trigger it
3. **Create** self-contained reproduction case
4. **Test** that it consistently reproduces the bug
5. **Minimize** by removing unnecessary parts
6. **Document** clear steps and expected vs actual behavior
7. **Check off** each acceptance criterion in `{{spec.path}}` by changing `- [ ]` to `- [x]`
8. **Commit** with message: `chant({{spec.id}}): <description>`
9. **Verify git status is clean** - ensure no uncommitted changes remain
