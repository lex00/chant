# Templates

Chant uses markdown templates for specs and prompts with simple variable substitution.

## Template Syntax

```markdown
{{variable}}     # Variable substitution
${ENV_VAR}       # Environment variable
```

## Spec Templates

Default spec template for `chant add`:

```markdown
# .chant/templates/spec.md
---
status: pending
---

# {{description}}

## Context

<!-- Why is this spec needed? -->

## Acceptance Criteria

- [ ]

## Notes

<!-- Optional: additional context -->
```

Usage:

```bash
chant add "Fix authentication bug"
# Creates spec with {{description}} = "Fix authentication bug"
```

### Spec Template Variables

| Variable | Description |
|----------|-------------|
| `{{description}}` | From `chant add` argument |
| `{{date}}` | Current date (YYYY-MM-DD) |
| `{{project}}` | Project name from config |
| `{{id}}` | Generated spec ID |

## Prompt Templates

Prompts use the same substitution:

```markdown
# .chant/prompts/standard.md
---
name: standard
---

You are implementing a spec for {{project.name}}.

**{{spec.title}}**

{{spec.description}}
```

### Prompt Template Variables

| Variable | Description |
|----------|-------------|
| `{{project.name}}` | Project name from config |
| `{{spec.id}}` | Full spec identifier |
| `{{spec.title}}` | Spec title (first heading) |
| `{{spec.description}}` | Full spec body content |
| `{{spec.target_files}}` | Target files from frontmatter |
| `{{worktree.path}}` | Worktree path (parallel execution) |
| `{{worktree.branch}}` | Branch name (parallel execution) |

## Custom Templates

Override defaults in config:

```yaml
# config.md
templates:
  spec: .chant/templates/my-spec.md
```

Or use `--template` flag:

```bash
chant add "Add feature" --template spec-tdd
```
