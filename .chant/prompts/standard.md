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
4. **Verify** the implementation works
5. **Check off** each acceptance criterion by changing `- [ ]` to `- [x]` in the spec file
6. **Commit** with message: `chant({{spec.id}}): <description>`

## Constraints

- Always write tests that validate behavior and run them until passing
- Always lint all and fix errors and warnings
- When complete, Always run all tests and fix errors
- Always use "just chant" if available otherwise use ./target/debug/chant
- Only modify files related to this spec
- Do not refactor unrelated code
- Always add model: {{spec.model}} to frontmatter after all acc criteria met
- Always  ensure chant binary builds
