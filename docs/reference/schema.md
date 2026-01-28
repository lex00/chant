# Schema & Validation

## The "Messy Markdown" Problem

Criticism: Markdown is messy, JSONL is clean.

Counter: Messiness is a validation problem, not a format problem. Agents can lint.

## Validation Layers

### 1. Schema Definition

Config defines required fields and valid values:

```yaml
# In config.md frontmatter
schema:
  spec:
    required: [status]    # id comes from filename
    fields:
      status:
        type: string
        enum: [pending, in_progress, completed, failed]
      depends_on:
        type: array
        items: string
      labels:
        type: array
        items: string
```

### 2. Lint on Write

Prompt instructs agent to validate before commit:

```markdown
# In prompt file
Before committing, verify the spec file:
- [ ] Frontmatter has required fields: id, status
- [ ] Status is one of: pending, in_progress, completed, failed
- [ ] All depends_on IDs exist
- [ ] YAML is valid
```

### 3. Lint on Read

Parser validates and normalizes:

```rust
fn parse_spec(path: &Path) -> Result<Spec, ValidationError> {
    let content = read_file(path)?;
    let (frontmatter, body) = split_frontmatter(&content)?;
    let spec: Spec = serde_yaml::from_str(&frontmatter)?;

    // Validate
    validate_required(&spec)?;
    validate_status(&spec)?;
    validate_deps_exist(&spec)?;

    Ok(spec)
}
```

### 4. Pre-commit Hook

CLI provides lint command:

```bash
chant lint                    # Lint all specs
chant lint 2026-01-22-001-x7m            # Lint specific spec
chant lint --fix              # Auto-fix where possible
```

## Auto-Fix Capabilities

| Issue | Auto-fixable | Fix |
|-------|--------------|-----|
| Missing `status` | Yes | Default to `pending` |
| Invalid `status` value | No | Error, human decides |
| Missing `id` | Yes | Generate from filename |
| Trailing whitespace | Yes | Trim |
| Inconsistent indentation | Yes | Normalize to 2 spaces |
| Missing newline at EOF | Yes | Add newline |

## Validation Errors

```bash
$ chant lint
2026-01-22-001-x7m.md:
  error: status "open" not in enum [pending, in_progress, completed, failed]
  error: depends_on "2026-01-22-999-zzz" does not exist

2026-01-22-002-q2n.md:
  warning: missing optional field "labels"

2 errors, 1 warning
```

## Agent-Friendly Validation

Agents get structured feedback:

```bash
$ chant lint --json
{
  "valid": false,
  "errors": [
    {
      "file": "2026-01-22-001-x7m.md",
      "field": "status",
      "message": "value 'open' not in enum",
      "allowed": ["pending", "in_progress", "completed", "failed"]
    }
  ]
}
```

Agent can then fix and retry.

## Why This Works

1. **Agents write most specs** - they follow the prompt, which includes validation
2. **Humans can still edit** - lint catches mistakes before commit
3. **Parse errors are rare** - YAML frontmatter is simple, well-supported
4. **Recovery is easy** - fix the text file, re-run lint

The format is human-readable AND machine-validatable. Chant chooses human-first with machine validation.

