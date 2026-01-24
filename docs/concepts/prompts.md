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

## Acceptance Criteria

{{#each spec.acceptance}}
- [ ] {{this}}
{{/each}}

## Instructions

1. **Read** the relevant code first
2. **Plan** your approach before coding
3. **Implement** the changes
4. **Verify** each acceptance criterion
5. **Commit** with message: `chant({{spec.id}}): <description>`
```

## Template Variables

| Variable | Description |
|----------|-------------|
| `{{spec.id}}` | Spec identifier |
| `{{spec.title}}` | Spec title (first heading) |
| `{{spec.description}}` | Spec body content |
| `{{spec.acceptance}}` | List of acceptance criteria |
| `{{spec.target_files}}` | List of target files |
| `{{spec.context}}` | Content of context files (all types) |
| `{{spec.tracks}}` | Tracked source files (documentation) |
| `{{spec.sources}}` | Source materials (research) |
| `{{spec.data}}` | Input data files (research) |
| `{{spec.driver}}` | Driver spec content (if group member) |
| `{{project.name}}` | Project name from config |

## Built-in Prompts

| Prompt | Purpose |
|--------|---------|
| `standard` | Read → Plan → Implement → Verify → Commit |
| `minimal` | Just do it, minimal ceremony |
| `tdd` | Test first, then implement |
| `security` | Extra verification for sensitive code |
| `split` | Split driver into group members |
| `documentation` | Read tracked code, write documentation |
| `research-synthesis` | Synthesize sources into findings |
| `research-analysis` | Analyze data, generate insights |

## Prompt Selection

```bash
chant work 2026-01-22-001-x7m                # Uses default prompt
chant work 2026-01-22-001-x7m --prompt tdd   # Uses TDD prompt
chant split 2026-01-22-001-x7m               # Uses split prompt (shorthand)
```

### Selection by Spec Type

Different spec types can use different default prompts:

```yaml
# config.md
prompts:
  default: standard
  by_type:
    code: standard
    documentation: documentation
    research: research-synthesis
```

Selection order:
1. `--prompt` CLI flag (highest priority)
2. `prompt:` in spec frontmatter
3. `prompts.by_type.<type>` from config
4. `prompts.default` from config

### Built-in Type Prompts

| Prompt | Type | Purpose |
|--------|------|---------|
| `standard` | code | Read → Plan → Implement → Verify → Commit |
| `documentation` | documentation | Read origin code, write docs |
| `research-synthesis` | research | Synthesize sources into findings |
| `research-analysis` | research | Analyze data, generate insights |

Type prompts extend `standard` with type-specific behavior.

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
- [ ] XSS prevented

Report findings in the spec file.
```

Use with: `chant work 2026-01-22-001-x7m --prompt security-review`

## Prompt Types

Chant has several prompt types, all using the same format:

| Type | Purpose | When Used |
|------|---------|-----------|
| **Prompt** | Main agent behavior | Spec execution |
| **Hook** | Pre/post execution | Before/after spec |
| **Retry** | Recovery behavior | After failure |

## Hooks

Specialized prompts that run at lifecycle points:

```yaml
# config.md
hooks:
  pre_work: .chant/prompts/pre_work.md
  post_work: .chant/prompts/post_work.md
  on_fail: .chant/prompts/on_fail.md
```

Hooks ARE prompts, just triggered differently. See [hooks.md](hooks.md).

## Retry Prompts

When a spec fails, retry with specialized context:

```markdown
# .chant/prompts/retry.md
---
name: retry
---

# Retry

The previous attempt failed. Here's what happened:

## Previous Error

```
{{last_error}}
```

## Attempt History

{{#each attempts}}
- Attempt {{number}}: {{status}} - {{error}}
{{/each}}

## Instructions

1. Analyze why the previous attempt failed
2. Identify the root cause
3. Fix the issue and try again
4. Do not repeat the same mistake

## Original Spec

{{spec_body}}
```

Retry context variables:

| Variable | Description |
|----------|-------------|
| `{{last_error}}` | Error from previous attempt |
| `{{attempts}}` | Array of previous attempts |
| `{{attempt_number}}` | Current attempt (2, 3, ...) |

## Retry Configuration

```yaml
# config.md
retry:
  max_attempts: 3
  prompt: retry             # Prompt to use for retries
  backoff: exponential      # none, linear, exponential
  initial_delay: 30s
```

Or per-spec:

```yaml
# spec frontmatter
---
status: pending
retry:
  max_attempts: 5
  prompt: retry-aggressive
---
```

## Everything is a Prompt

| What | Is Actually |
|------|-------------|
| Main execution | Prompt |
| Pre-work hook | Prompt (triggered before) |
| Post-work hook | Prompt (triggered after) |
| On-fail hook | Prompt (triggered on failure) |
| Retry | Prompt (with retry context) |

The only difference is **when** they're invoked and **what context** they receive.

## Prompt Selection Flow

```
Spec starts
    │
    ├──→ pre_work prompt (if configured)
    │
    ├──→ main prompt (from spec.prompt or config.defaults.prompt)
    │
    ├──→ [success] ──→ post_work prompt (if configured)
    │
    └──→ [failure] ──→ on_fail prompt (if configured)
                           │
                           └──→ retry prompt (if attempts remaining)
                                    │
                                    └──→ [back to main prompt]
```

## Unified Model

All prompts share:

1. **Markdown format** - Human-readable, version-controlled
2. **YAML frontmatter** - Metadata (name, conditions)
3. **Template variables** - `{{spec_id}}`, `{{error}}`, etc.
4. **Same invocation** - Passed to agent the same way

## Example: Complete Prompt Chain

```bash
chant work 2026-01-22-001-x7m
```

1. Load spec
2. Execute `pre_work.md` → "Check dependencies are installed"
3. Execute `standard.md` → "Implement the spec"
4. Spec fails
5. Execute `on_fail.md` → "Analyze failure, clean up"
6. Execute `retry.md` → "Try again with error context"
7. Execute `standard.md` → "Implement the spec" (attempt 2)
8. Spec succeeds
9. Execute `post_work.md` → "Run final validation, format code"

## Non-Prompt Concepts

These are NOT prompts:

| Concept | What It Is |
|---------|------------|
| Template | Scaffolding for creating files (not executed) |
| Notification | Output formatting (not agent instruction) |
| Config | Settings (not behavior) |

## Simplification

Minimal model: one prompt defines how to execute specs. Everything else is optional.
