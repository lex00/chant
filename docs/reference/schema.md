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

## Output Schema Validation

For task specs that produce structured output (research reports, analysis results, etc.), you can enforce a JSON Schema on agent output.

### Defining an Output Schema

Add `output_schema` to spec frontmatter pointing to a JSON Schema file:

```yaml
---
type: task
status: ready
output_schema: .chant/schemas/research-report.json
---

# Research issue #1234

Investigate root cause and produce structured report.
```

### Creating Schema Files

Create JSON Schema files in `.chant/schemas/`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["spec_id", "findings", "recommendation"],
  "properties": {
    "spec_id": {
      "type": "string",
      "pattern": "^[A-Z]\\.[0-9]+\\.[0-9]+$"
    },
    "findings": {
      "type": "array",
      "items": {"type": "string"},
      "minItems": 1
    },
    "recommendation": {
      "type": "string"
    }
  }
}
```

### How It Works

1. **Prompt Injection**: When `output_schema` is present, chant automatically injects an "Output Format" section into the agent prompt with the schema definition, required fields, and an example.

2. **Post-Execution Validation**: After the agent completes, chant extracts JSON from the agent output and validates it against the schema.

3. **Linter Integration**: `chant lint` validates output for completed specs that have `output_schema` defined.

### Configuration

Control validation strictness in `config.md`:

```yaml
---
validation:
  strict_output_validation: false  # Default: warn but allow
---
```

When `strict_output_validation: true`:
- Specs fail if output doesn't match schema
- Status is set to `needs_attention`

When `strict_output_validation: false` (default):
- Warning is shown but spec proceeds to completion
- Useful for gradual adoption

### Validation Output

**Success:**
```bash
✓ Output validation passed (schema: .chant/schemas/research-report.json)
```

**Failure:**
```bash
✗ Output validation failed (schema: .chant/schemas/research-report.json)
  - missing required field 'spec_id'
  - at '/findings': expected array, got string
  → Review .chant/logs/2026-01-29-001-abc.log for details
```

### JSON Extraction

Chant uses multiple strategies to extract JSON from agent output:

1. **Code blocks**: ```` ```json ... ``` ```` or ```` ``` ... ``` ````
2. **Bare JSON**: Entire output is valid JSON
3. **Embedded JSON**: `{...}` or `[...]` patterns in text

### Example Workflow

1. Create schema:
   ```bash
   mkdir -p .chant/schemas
   cat > .chant/schemas/research.json << 'EOF'
   {
     "$schema": "https://json-schema.org/draft/2020-12/schema",
     "type": "object",
     "required": ["spec_id", "root_cause"],
     "properties": {
       "spec_id": {"type": "string"},
       "root_cause": {"type": "string"},
       "affected_files": {"type": "array", "items": {"type": "string"}}
     }
   }
   EOF
   ```

2. Create spec with schema reference:
   ```bash
   chant add "Research bug #123"
   # Edit spec to add: output_schema: .chant/schemas/research.json
   ```

3. Work the spec - agent sees schema in prompt
4. Validation runs automatically on completion
5. Check all specs: `chant lint`

