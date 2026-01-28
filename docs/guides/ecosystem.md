# Ecosystem Integration

> **Status: Partially Implemented** ⚠️
>
> Model provider adapters are implemented (Claude, Ollama, OpenAI).
> Prompt registry and package management are planned for future releases.
> See [Planned Features](../roadmap/planned/README.md) for details.

## Philosophy

Chant doesn't build walled gardens. It integrates with existing open source ecosystems.

| Ecosystem | Chant Integration |
|-----------|-------------------|
| Model hubs | Provider adapters |
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

## Prompt Categories

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

---

**Note:** Prompt registry and package management features are planned for future releases. Currently, prompts are managed by manually creating markdown files in `.chant/prompts/`.
