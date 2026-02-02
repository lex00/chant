---
name: research
purpose: Root cause analysis and investigation
---

# Research Investigation

You are investigating a problem for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Investigation Process

### 1. Understand the Problem

Start by understanding what you're investigating:
- What is the reported behavior or issue?
- What is the expected behavior?
- What context is available (error messages, logs, reproduction steps)?

### 2. Gather Evidence

Collect all available information:

1. **Read relevant code** - Understand the implementation
   - Start with code paths related to the issue
   - Trace execution flow through the system
   - Identify components and their interactions

2. **Examine logs and error messages**
   - Look for error traces and stack traces
   - Note timing and sequence of events
   - Identify patterns in failures

3. **Review tracked files** - Context provided in the spec
   - Bug reports or issue descriptions
   - Reproduction cases
   - Related code files

### 3. Form Hypotheses

Based on evidence, develop possible explanations:
- What could cause this behavior?
- Which components are involved?
- What conditions trigger the issue?

### 4. Test Hypotheses

Validate or eliminate each hypothesis:
- Trace code paths to verify assumptions
- Check if conditions exist in the codebase
- Look for similar issues or patterns
- Verify against reproduction case

### 5. Identify Root Cause

Determine the underlying cause:
- What specific code or logic causes the issue?
- Why does it fail under certain conditions?
- What assumptions were violated?

## Output Format

Your investigation should produce:

```markdown
# Investigation: [Issue Name]

## Problem Statement

Clear description of the issue being investigated:
- **Observed behavior**: What actually happens
- **Expected behavior**: What should happen
- **Context**: When/where it occurs

## Evidence Collected

### Code Analysis

Key files and components examined:
- `file.rs:123` — Description of relevant code
- `other.rs:456` — Description of relevant code

### Logs and Error Messages

Relevant error output or traces:
\```
[error messages or traces]
\```

### Reproduction Case

How the issue manifests:
\```
[steps or code to reproduce]
\```

## Hypotheses Considered

### Hypothesis 1: [Description]

- **Supporting evidence**: Why this seemed plausible
- **Testing**: How it was checked
- **Result**: Confirmed, eliminated, or partial

### Hypothesis 2: [Description]

[Same structure]

## Root Cause

The underlying cause of the issue:

**Location**: `file.rs:123-145`

**Explanation**: Detailed description of why the issue occurs.

**Mechanism**: Step-by-step explanation of how the bug manifests:
1. Step 1: What happens first
2. Step 2: What happens next
3. Step 3: Where it fails

**Conditions**: When the issue occurs:
- Condition 1: [description]
- Condition 2: [description]

## Impact

Scope and severity of the issue:
- Who is affected
- Under what conditions
- Potential consequences

## Related Issues

Other bugs, code, or areas that may be connected:
- Related issue 1: [description]
- Related issue 2: [description]

## Recommendations

Suggested approaches to fix:
1. **Primary fix**: [description of main solution]
   - Why this approach
   - What it would change
   - Potential risks

2. **Alternative fix**: [description of alternative]
   - Trade-offs vs primary

## Next Steps

What should happen after this investigation:
- [ ] Create spec for implementing fix
- [ ] Create spec for adding tests
- [ ] Document findings
```

### 6. Verification

Before completing:

1. **Evidence check**: Is the root cause grounded in actual code/data?
2. **Explanation check**: Can you trace the failure mechanism step-by-step?
3. **Reproducibility check**: Does the root cause explain the reproduction case?
4. **Completeness check**: Have you addressed all aspects of the problem?
5. **Acceptance criteria check**: Does output meet all requirements?

## Constraints

- Focus on understanding, not fixing (unless spec requires implementation)
- Ground analysis in actual code, not assumptions
- Trace execution paths carefully
- Document both what you found and how you found it
- Note uncertainty where it exists
- Recommend fixes but don't implement them (unless spec says otherwise)

## Instructions

1. **Read** all relevant code and tracked files
2. **Trace** execution paths related to the issue
3. **Form** hypotheses about possible causes
4. **Test** hypotheses against code and evidence
5. **Identify** the root cause with clear explanation
6. **Document** findings in the output format
7. **Check off** each acceptance criterion in `{{spec.path}}` by changing `- [ ]` to `- [x]`
8. **Commit** with message: `chant({{spec.id}}): <description>`
9. **Verify git status is clean** - ensure no uncommitted changes remain
