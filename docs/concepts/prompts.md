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

Prompts support variable substitution using `{{variable}}` syntax. When a prompt is assembled, Chant replaces these placeholders with actual values from the spec and project configuration.

### Substitution Syntax

Variables are enclosed in double curly braces: `{{variable}}`

```markdown
You are implementing a spec for {{project.name}}.

## Your Spec

**{{spec.title}}**

{{spec.description}}

Commit with: `chant({{spec.id}}): <description>`
```

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

### Examples

**Spec file:**
```markdown
---
status: pending
target_files:
  - src/auth/login.ts
  - src/auth/logout.ts
---

# Add user authentication

Implement JWT-based authentication for the API.

## Acceptance Criteria

- [ ] Login endpoint returns JWT
- [ ] Logout invalidates token
```

**In your prompt:**
```markdown
# Task: {{spec.title}}

{{spec.description}}

## Files to Modify

{{#each spec.target_files}}
- {{this}}
{{/each}}

## When Done

Commit with message: `chant({{spec.id}}): {{spec.title}}`
```

**Rendered output:**
```markdown
# Task: Add user authentication

Implement JWT-based authentication for the API.

## Acceptance Criteria

- [ ] Login endpoint returns JWT
- [ ] Logout invalidates token

## Files to Modify

- src/auth/login.ts
- src/auth/logout.ts

## When Done

Commit with message: `chant(2026-01-24-001-x7m): Add user authentication`
```

### Iteration with `{{#each}}`

Use Handlebars-style iteration for list variables:

```markdown
{{#each spec.target_files}}
- {{this}}
{{/each}}

{{#each spec.acceptance}}
- [ ] {{this}}
{{/each}}
```

### Conditionals with `{{#if}}`

Check if variables exist before using them:

```markdown
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
| `standard` | Read → Plan → Implement → Verify → Commit |
| `minimal` | Just do it, minimal ceremony |
| `tdd` | Test first, then implement |
| `security` | Extra verification for sensitive code |
| `split` | Split driver into group members |
| `documentation` | Read tracked code, write documentation |
| `research-synthesis` | Synthesize sources into findings |
| `research-analysis` | Analyze data, generate insights |

## When to Use Each Prompt

### standard.md (Default)

**What it does:** Instructs the agent to thoroughly read relevant code, plan an approach, implement changes carefully, verify each acceptance criterion, and commit with a proper message.

**When to use:**
- Regular feature implementation
- Bug fixes
- Code refactoring
- Default choice for most work

**Command:**
```bash
chant work 2026-01-24-001-abc     # Uses standard by default
chant work 2026-01-24-001-abc --prompt standard  # Explicit
```

### split.md

**What it does:** Analyzes a driver spec and proposes how to split it into smaller, ordered member specs. Each member spec is independently testable and valuable.

**When to use:**
- Breaking down large drivers into manageable pieces
- Planning complex feature work before implementation
- Ensuring specs leave code in compilable state at each step

**Key characteristics:**
- Proposes sequence of members
- Includes detailed acceptance criteria for each member
- Documents edge cases and test scenarios
- Ensures minimal dependencies between members

**Command:**
```bash
chant split 2026-01-24-001-abc    # Shorthand for split prompt
chant work 2026-01-24-001-abc --prompt split
```

**Example output structure:**
```
## Member 1: Setup initial component structure
- [ ] Create component files
- [ ] Add basic structure
...

## Member 2: Implement core functionality
- [ ] Add feature X
- [ ] Add feature Y
...
```

### Other Type-Specific Prompts

**documentation.md:** Used for specs that track and document existing code. The agent reads the tracked files and writes documentation.

**research-synthesis.md:** Used for research specs to synthesize findings from multiple sources into cohesive insights.

**research-analysis.md:** Used for research specs to analyze data and generate insights.

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

## Prompt Files in `.chant/prompts/`

This directory contains all prompts for your project. Built-in prompts are in your repository, and you can view and customize them.

### File Structure

Each prompt file has:

```markdown
---
name: <prompt-name>         # How to reference this prompt
purpose: <description>      # What this prompt does
---

# Prompt Title

Instructions and behavior for the agent...

{{spec.title}}
{{spec.description}}
...
```

### Viewing Prompt Files

```bash
# View the standard execution prompt
cat .chant/prompts/standard.md

# View the split prompt
cat .chant/prompts/split.md
```

### Creating Custom Prompts

Create new files in `.chant/prompts/` for custom behaviors:

```markdown
# .chant/prompts/my-custom.md
---
name: my-custom
purpose: Custom workflow
---

# Custom Prompt

Your instructions here...
```

Use with: `chant work 2026-01-24-001-abc --prompt my-custom`

## Simplification

Minimal model: one prompt defines how to execute specs. Everything else is optional.
