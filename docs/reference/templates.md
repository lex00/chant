# Templates

## Everything Has a Template

Chant uses markdown templates throughout:

| Thing | Template Location |
|-------|-------------------|
| Specs | `.chant/templates/spec.md` |
| Prompts | `.chant/templates/prompt.md` |
| Config | `.chant/templates/config.md` |
| Notifications | `.chant/templates/notification.md` |
| Hooks | `.chant/templates/hook.md` |

## Template Engine

Handlebars-style with simple extensions:

```handlebars
{{variable}}                  # Variable substitution
{{#if condition}}...{{/if}}   # Conditional
{{#each items}}...{{/each}}   # Iteration
{{> partial}}                 # Include partial
${ENV_VAR}                    # Environment variable
```

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

## Template Inheritance

Extend base templates:

```markdown
# .chant/templates/spec-bug.md
---
extends: spec.md
---

# Bug: {{description}}

## Reproduction Steps

1.

## Expected Behavior

## Actual Behavior

## Acceptance Criteria

- [ ] Bug no longer reproduces
- [ ] Regression test added
```

Usage:

```bash
chant add "Login fails on Safari" --template spec-bug
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

### Notification Templates

See [notifications.md](notifications.md) for notification-specific variables.

## Partials

Reusable template fragments:

```markdown
# .chant/templates/partials/criteria.md
## Acceptance Criteria

- [ ] All tests pass
- [ ] No linting errors
- [ ] Documentation updated (if applicable)
```

Include in template:

```markdown
# .chant/templates/spec.md
---
status: pending
---

# {{description}}

{{> criteria}}
```

## Conditional Templates

Project-specific templates via path patterns:

```yaml
# config.md
templates:
  spec:
    default: .chant/templates/spec.md
    patterns:
      "packages/auth/**": .chant/templates/spec-auth.md
      "packages/api/**": .chant/templates/spec-api.md
```

## Template Validation

Chant validates templates at init and runtime:

```bash
chant lint --templates
```

Checks:
- Required frontmatter fields
- Valid handlebars syntax
- Partials exist
- No undefined variables

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
