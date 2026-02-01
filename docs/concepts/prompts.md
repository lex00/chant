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

| Variable | Description |
|----------|-------------|
| `{{project.name}}` | Project name from config |
| `{{spec.id}}` | Full spec identifier |
| `{{spec.title}}` | Spec title (first `#` heading) |
| `{{spec.description}}` | Full spec body content |
| `{{spec.target_files}}` | Target files from frontmatter |
| `{{worktree.path}}` | Worktree path (parallel execution) |
| `{{worktree.branch}}` | Branch name (parallel execution) |

> **Note:** Advanced templating (`{{#if}}`, `{{#each}}`) is not implemented.
> Only simple `{{variable}}` substitution is supported.

## Built-in Prompts

| Prompt | Purpose |
|--------|---------|
| `bootstrap` | **(Default)** Minimal prompt that delegates to `chant prep` |
| `standard` | Read → Plan → Implement → Verify → Commit |
| `split` | Split driver into group members |
| `documentation` | Read tracked code, write documentation |
| `research-synthesis` | Synthesize sources into findings |
| `merge-conflict` | Resolve git conflicts during rebase |

## Prompt Selection

```bash
chant work 001                # Uses default prompt (bootstrap)
chant work 001 --prompt tdd   # Uses TDD prompt
```

### Selection Order

1. `--prompt` CLI flag (highest priority)
2. `prompt:` in spec frontmatter
3. `defaults.prompt` from config

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
```

Use with: `chant work 001 --prompt security-review`

---

For comprehensive guidance, examples, and advanced techniques, see the [Prompt Guide](../guides/prompts.md).
