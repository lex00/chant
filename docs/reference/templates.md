# Templates

> **Status: Partially Implemented** ⚠️
>
> Basic spec template substitution is implemented. Full Handlebars templating system
> (conditionals, helpers, partials, inheritance) is not yet implemented.
> See roadmap for future enhancements.

## Everything Has a Template

Chant uses markdown templates throughout:

| Thing | Template Location | Status |
|-------|-------------------|--------|
| Specs | `.chant/templates/spec.md` | ✅ Basic substitution |
| Prompts | `.chant/templates/prompt.md` | ✅ Basic substitution |
| Config | `.chant/templates/config.md` | ✅ Basic substitution |

## Template Engine

Handlebars-style with simple extensions:

```handlebars
{{variable}}                  # Variable substitution ✅ Implemented
{{#if condition}}...{{/if}}   # Conditional (planned)
{{#each items}}...{{/each}}   # Iteration (planned)
{{> partial}}                 # Include partial (planned)
${ENV_VAR}                    # Environment variable ✅ Implemented
```

> **Note:** Currently only `{{variable}}` substitution and `${ENV_VAR}` expansion are implemented. Conditionals, iteration, and partials are planned for the full Handlebars system.

## Spec Template

Default spec template for `chant add`:

```markdown
# .chant/templates/spec.md
---
status: pending
created: {{date}}
{{#if project}}
project: {{project}}
{{/if}}
---

# {{description}}

## Context

<!-- Why is this spec needed? -->

## Acceptance Criteria

- [ ]

## Target Files

<!-- Optional: files this spec will modify -->

## Notes

<!-- Optional: additional context -->
```

Usage:

```bash
chant add "Fix authentication bug"
# Creates spec from template with:
#   {{description}} = "Fix authentication bug"
#   {{date}} = "2026-01-22"
#   {{project}} = from config or path
```

## Prompt Template

Default prompt template:

```markdown
# .chant/templates/prompt.md
---
name: {{name}}
---

# {{name}}

You are an AI agent executing a chant spec.

## Instructions

1. Read the spec carefully
2. Understand the acceptance criteria
3. Implement the changes
4. Verify all criteria are met
5. Commit the changes

## Constraints

- Only modify files mentioned in target_files (if specified)
- Run tests before committing
- Follow existing code style
```

## Custom Templates per Project

Override defaults:

```yaml
# config.md
templates:
  spec: .chant/templates/my-spec.md
  prompt: .chant/templates/my-prompt.md
```

## Template Variables

### Spec Templates

| Variable | Description |
|----------|-------------|
| `{{description}}` | From `chant add` argument |
| `{{date}}` | Current date (YYYY-MM-DD) |
| `{{time}}` | Current time (HH:MM:SS) |
| `{{project}}` | Project from config or path |
| `{{user}}` | Git user.name |
| `{{branch}}` | Current git branch |
| `{{id}}` | Generated spec ID |

### Prompt Templates

| Variable | Description |
|----------|-------------|
| `{{name}}` | Prompt name |
| `{{project}}` | Project name |

## Built-in Templates

Chant ships with sensible defaults. `chant init` creates:

```
.chant/
  templates/
    spec.md           # Basic spec
    spec-bug.md       # Bug report
    spec-feature.md   # Feature request
  prompts/
    standard.md       # Default prompt
```

## Example: TDD Spec Template

```markdown
# .chant/templates/spec-tdd.md
---
status: pending
prompt: tdd
---

# {{description}}

## Test First

Write failing test:

```
{{test_location}}
```

## Implementation

After test passes:

## Acceptance Criteria

- [ ] Failing test written first
- [ ] Implementation makes test pass
- [ ] No other tests broken
- [ ] Coverage maintained
```

```bash
chant add "Add email validation" --template spec-tdd
```

---

**Note:** Full Handlebars templating features (conditionals, helpers, partials, inheritance) are planned for future releases. Currently, basic variable substitution (`{{variable}}`) and environment variable expansion (`${ENV_VAR}`) are supported.
