# Prompts

A markdown file that defines agent behavior. Lives in `.chant/prompts/`.

```markdown
# .chant/prompts/standard.md
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
4. **Verify** each acceptance criterion
5. **Commit** with message: `chant({{spec.id}}): <description>`
```

## Template Variables

Prompts support variable substitution using `{{variable}}` syntax.

### Available Variables

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `{{project.name}}` | Project name from config | `my-app` |
| `{{spec.id}}` | Full spec identifier | `2026-01-24-001-x7m` |
| `{{spec.title}}` | Spec title (first `#` heading) | `Add user authentication` |
| `{{spec.description}}` | Full spec body content | The markdown content after frontmatter |
| `{{spec.acceptance}}` | List of acceptance criteria | Array of criterion strings |
| `{{spec.target_files}}` | List of target files from frontmatter | Array of file paths |
| `{{spec.context}}` | Content of referenced context files | Concatenated file contents |
| `{{spec.tracks}}` | Tracked source files (documentation specs) | Array of file paths |
| `{{spec.sources}}` | Source materials (research specs) | Array of URLs or references |
| `{{spec.data}}` | Input data files (research specs) | Array of file paths |
| `{{spec.driver}}` | Driver spec content (if group member) | Parent spec markdown |

### Iteration and Conditionals

```markdown
{{#each spec.target_files}}
- {{this}}
{{/each}}

{{#if spec.target_files}}
Focus on these files:
{{#each spec.target_files}}
- {{this}}
{{/each}}
{{/if}}
```

## Built-in Prompts

| Prompt | Purpose |
|--------|---------|
| `bootstrap` | **(Default)** Minimal prompt that delegates to `chant prep` |
| `standard` | Read → Plan → Implement → Verify → Commit |
| `minimal` | Just do it, minimal ceremony |
| `tdd` | Test first, then implement |
| `security` | Extra verification for sensitive code |
| `split` | Split driver into group members |
| `documentation` | Read tracked code, write documentation |
| `research-synthesis` | Synthesize sources into findings |
| `merge-conflict` | Resolve git conflicts during rebase |

## Prompt Selection

```bash
chant work 2026-01-22-001-x7m                # Uses default prompt
chant work 2026-01-22-001-x7m --prompt tdd   # Uses TDD prompt
chant split 2026-01-22-001-x7m               # Uses split prompt
```

### Selection Order

1. `--prompt` CLI flag (highest priority)
2. `prompt:` in spec frontmatter
3. `prompts.by_type.<type>` from config
4. `prompts.default` from config

## Custom Prompts

Create custom prompts in `.chant/prompts/`:

```markdown
# .chant/prompts/security-review.md
---
name: security-review
purpose: Security-focused code review
---

# Security Review

Review {{spec.target_files}} for security issues.

## Checklist
- [ ] No hardcoded secrets
- [ ] Input validation present
- [ ] SQL injection prevented
```

Use with: `chant work 2026-01-22-001-x7m --prompt security-review`

## Hooks and Retry Prompts

Prompts can run at lifecycle points:

```yaml
# config.md
hooks:
  pre_work: .chant/prompts/pre_work.md
  post_work: .chant/prompts/post_work.md
  on_fail: .chant/prompts/on_fail.md
```

Retry prompts include additional context:

| Variable | Description |
|----------|-------------|
| `{{last_error}}` | Error from previous attempt |
| `{{attempts}}` | Array of previous attempts |
| `{{attempt_number}}` | Current attempt (2, 3, ...) |

## Prompt Flow

```
Spec starts
    │
    ├──→ pre_work prompt (if configured)
    │
    ├──→ main prompt
    │
    ├──→ [success] ──→ post_work prompt (if configured)
    │
    └──→ [failure] ──→ on_fail prompt (if configured)
                           │
                           └──→ retry prompt (if attempts remaining)
```

---

For comprehensive prompt authoring guidance, examples, and advanced techniques, see the [Prompt Guide](../guides/prompts.md).
