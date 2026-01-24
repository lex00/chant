---
name: standard
purpose: Default execution prompt
---

# Execute Spec

You are implementing a spec for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Instructions

1. **Read** the relevant code first
2. **Plan** your approach before coding
3. **Implement** the changes
3a. write tests that validate behavior
3b. iterate on test until it works
3c. lint all and fix errors
3d. run all tests and fix errors
3e. ensure chant binary builds
4. **Verify** the implementation works
5. **Check off** each acceptance criterion by changing `- [ ]` to `- [x]` in the spec file
6. **Commit** with message: `chant({{spec.id}}): <description>`

## Constraints

- Always stream your output to .chant/logs/{{spec.id}}.log
- Always use "just chant" if available otherwise use ./target/debug/chant
- Only modify files related to this spec
- Follow existing code patterns
- Do not refactor unrelated code
