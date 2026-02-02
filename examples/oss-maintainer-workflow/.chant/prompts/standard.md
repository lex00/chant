---
name: standard
purpose: Standard code implementation and modification tasks
---

# Standard Implementation Task

You are implementing a task for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

## Implementation Process

### 1. Understand the Requirements

Review your spec carefully:
- What is the goal of this task?
- What files need to be modified?
- What are the acceptance criteria?
- Are there any dependencies or related specs?

### 2. Read Relevant Code

Before making changes:
- Read files mentioned in `target_files`
- Understand the current implementation
- Identify patterns and conventions used in the codebase
- Check for related code that might be affected

### 3. Plan Your Approach

Consider:
- What needs to change to meet the requirements?
- What is the simplest approach?
- Are there any edge cases to handle?
- Will this break existing functionality?

### 4. Implement Changes

Make the necessary changes:
- Follow existing code patterns and conventions
- Keep changes focused on the spec requirements
- Don't refactor unrelated code
- Add comments only where logic isn't self-evident
- Ensure backward compatibility unless spec says otherwise

### 5. Verify Your Work

Before marking complete:
- Do all acceptance criteria pass?
- Does the code follow project conventions?
- Are there any errors or warnings?
- Does the implementation match the spec?

## Constraints

- Only modify files directly related to the spec requirements
- Don't add features or improvements beyond what's specified
- Don't refactor unrelated code
- Keep solutions simple and focused
- Follow the project's existing patterns and conventions

## Instructions

1. **Read** the relevant code first - never propose changes to code you haven't seen
2. **Plan** your approach before coding
3. **Implement** the changes according to the spec
4. **Run `cargo fmt`** to format the code (if this is a Rust project)
5. **Run `cargo clippy`** to fix any lint errors and warnings (if this is a Rust project)
6. **Run tests** if specified in the acceptance criteria
7. **Verify** the implementation works and all acceptance criteria are met
8. **Check off** each acceptance criterion in `{{spec.path}}` by changing `- [ ]` to `- [x]`
9. **Commit** with message: `chant({{spec.id}}): <description>`
10. **Verify git status is clean** - ensure no uncommitted changes remain

## Notes

- If you encounter issues outside the scope of this spec, create a new spec for them
- Ask for clarification if requirements are ambiguous
- Focus on completing the acceptance criteria
- Keep your implementation simple and maintainable
