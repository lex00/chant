# Ecosystem Integration

## Philosophy

Chant doesn't build walled gardens. It integrates with existing open source ecosystems.

| Ecosystem | Chant Integration |
|-----------|-------------------|
| Model hubs | Provider adapters |
| Prompt communities | Prompt registry |
| Package managers | Prompt packages |
| Git forges | Native workflow |

## Model Repositories

Chant supports various model hosting options:

- Local model runners
- Cloud model APIs
- Model aggregators
- Standard API endpoints

```yaml
# config.md
agent:
  provider: my-provider
  model: my-model
  endpoint: ${PROVIDER_ENDPOINT}
```

## Prompt Registry (Planned)

> **Status: Planned** - The prompt registry feature is on the roadmap but not yet implemented. Currently, prompts are managed by manually creating markdown files in `.chant/prompts/`.

### Installing Prompts

```bash
# From official registry
chant prompt add tdd
chant prompt add security-review
chant prompt add documentation

# From GitHub
chant prompt add --from github:user/repo/prompts/custom.md

# From URL
chant prompt add --from https://example.com/prompts/special.md

# From local file
chant prompt add --from ~/my-prompts/favorite.md
```

### Registry Sources

```yaml
# config.md
prompts:
  registries:
    - https://prompts.chant.dev          # Official
    - https://github.com/chant-prompts   # Community
    - https://internal.company.com/prompts  # Private
```

### Prompt Packages

Like npm packages, but for prompts. Lock file is markdown (of course):

```markdown
# .chant/prompts.lock.md
---
packages:
  tdd:
    version: 1.2.0
    source: chant-prompts/tdd
    sha256: abc123def456...
  security:
    version: 2.0.1
    source: github:acme/security-prompts
    sha256: 789abc012def...
---

# Installed Prompts

Prompts installed from registry. Edit frontmatter to pin versions,
or use `chant prompt update` to refresh.

## tdd (1.2.0)

Test-driven development workflow. Write tests first, then implement.

Source: chant-prompts/tdd

## security (2.0.1)

Security-focused code review and implementation.

Source: github:acme/security-prompts
```

The body is documentation. The frontmatter is the lock data. Git diffs show
exactly what changed.

```bash
chant prompt install        # Install from lock file
chant prompt update tdd     # Update specific prompt
chant prompt update         # Update all
```

### Publishing Prompts

```bash
# Package your prompt
chant prompt pack .chant/prompts/my-workflow.md

# Publish to registry (requires auth)
chant prompt publish my-workflow

# Or just push to GitHub
git push origin main  # Others can --from github:you/repo
```

## Community Prompt Categories

| Category | Examples |
|----------|----------|
| **Workflow** | tdd, code-review, documentation |
| **Language** | rust-idioms, go-patterns, python-best-practices |
| **Domain** | security-audit, performance, accessibility |
| **Framework** | react-components, django-views, rails-models |
| **Style** | minimal, verbose, educational |

## Prompt Composition

Prompts can extend other prompts:

```markdown
# .chant/prompts/my-tdd.md
---
name: my-tdd
extends: tdd          # From registry
---

# Additional Instructions

Also follow our team conventions:
- Use table-driven tests
- Mock external services
- Minimum 80% coverage
```

## Model + Prompt Recommendations

```yaml
# config.md - recommend pairings
recommendations:
  prompts:
    tdd:
      models: [high-capability]
      note: "Needs strong reasoning for test-first approach"

    quick-fix:
      models: [fast]
      note: "Fast models work fine for simple fixes"
```

## Template Repositories

GitHub template repos for common setups:

| Template | Use Case |
|----------|----------|
| `chant-prompts/solo-dev` | Individual developer |
| `chant-prompts/team-standard` | Team with PR workflow |
| `chant-prompts/enterprise` | Enterprise with compliance |
| `chant-prompts/monorepo` | Large-scale monorepo |

```bash
# Clone template
gh repo create my-project --template chant-prompts/team-standard

# Or copy prompts
chant init --template team-standard
```

## Version Compatibility

```yaml
# prompt frontmatter
---
name: security-audit
version: 2.0.0
chant: ">=0.5.0"        # Minimum chant version
models:
  required: [large-model, capable-model]  # Known working
  experimental: [local-model:34b]         # May work
---
```

## Discovery

```bash
# Search registry
chant prompt search "security"
chant prompt search --category workflow
chant prompt search --model ollama

# Popular prompts
chant prompt popular

# Recently updated
chant prompt recent
```

## Offline Mode

Prompts are just files. Work offline:

```bash
# Cache all prompts locally
chant prompt cache --all

# Work offline
chant work 001  # Uses cached prompts
```

## Contributing

### To Official Registry

1. Fork `github.com/chant-prompts/registry`
2. Add prompt to `prompts/category/name.md`
3. Open PR with:
   - Prompt file
   - README with use cases
   - Example spec + output

### Quality Standards

- Clear purpose statement
- Tested with multiple models
- Version constraints specified
- Example output included

## Future: Prompt Marketplace

Potential commercial feature:

- Curated enterprise prompts
- Domain expert prompts (security, compliance)
- Team-specific customization
- Usage analytics
