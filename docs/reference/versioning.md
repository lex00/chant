# Versioning

## What Gets Versioned

| Component | Where | Purpose |
|-----------|-------|---------|
| Chant CLI | Binary | Feature compatibility |
| Config schema | `config.md` | Configuration format |
| Spec schema | Validated by linter | Spec file format |

## Config Version

```yaml
# config.md frontmatter
---
version: 1                    # Schema version

project:
  name: my-app
# ...
---
```

CLI checks version on load:
- Same version → proceed
- Older version → warn, suggest migration
- Newer version → error, update CLI

## Spec Schema Version

Specs don't have explicit version. Schema is defined in config:

```yaml
# config.md
schema:
  version: 1
  spec:
    required: [status]
    status:
      enum: [pending, in_progress, completed, failed]
```

Linter validates against config's schema definition.

## When Linter Runs

| Trigger | Automatic | Purpose |
|---------|-----------|---------|
| `chant lint` | No | Explicit validation |
| `chant work` | Yes | Pre-execution check |
| `chant add` | Yes | Validate new spec |
| After agent writes | Yes | Validate output |

### Before Execution

```rust
fn work(spec_id: &str) -> Result<()> {
    // Lint before starting
    let errors = lint_spec(spec_id)?;
    if !errors.is_empty() {
        return Err(Error::LintFailed(errors));
    }

    // Proceed with execution
    execute(spec_id)
}
```

### After Agent Writes

Agent may create/modify specs. Validate after:

```rust
fn post_execution_lint(spec_id: &str) -> Result<()> {
    // Find all specs modified by agent
    let modified = git_diff_specs()?;

    for spec in modified {
        let errors = lint_spec(&spec)?;
        if !errors.is_empty() {
            warn!("Agent produced invalid spec: {}", spec);
            // Auto-fix if possible
            if let Err(e) = auto_fix(&spec) {
                return Err(Error::LintFailed(errors));
            }
        }
    }

    Ok(())
}
```

## Schema Migration

When schema changes between versions:

```bash
$ chant lint
Warning: Config schema version 1, current is 2

Migration available:
  - 'status: open' → 'status: pending' (v2 renamed)

Run 'chant migrate' to update.
```

### Migration Command

```bash
chant migrate              # Dry run, show changes
chant migrate --apply      # Apply changes
```

```
$ chant migrate
Schema migration v1 → v2

Changes:
  config.md:
    - Add 'version: 2'

  2026-01-22-001-x7m.md:
    - 'status: open' → 'status: pending'

  2026-01-22-002-q2n.md:
    - No changes needed

Run 'chant migrate --apply' to apply.
```

## Backwards Compatibility

### Reading Old Specs

CLI should read specs from older schema versions:

```rust
fn parse_spec(content: &str) -> Result<Spec> {
    let raw: RawSpec = parse_frontmatter(content)?;

    // Handle old field names
    let status = raw.status
        .or(raw.state)           // v0 used 'state'
        .unwrap_or("pending");

    // Normalize old values
    let status = match status {
        "open" => "pending",     // v1 used 'open'
        "done" => "completed",   // v1 used 'done'
        s => s,
    };

    Ok(Spec { status, ... })
}
```

### Writing Current Version

Always write current schema:

```rust
fn save_spec(spec: &Spec) -> Result<()> {
    // Always use current field names
    let frontmatter = format!(
        "status: {}\n",
        spec.status  // Not 'state', not 'open'
    );
    // ...
}
```

## Version History

| Version | Changes |
|---------|---------|
| 1 | Initial schema |
| 2 | (future) ... |

## CLI Version Check

```bash
$ chant version
chant 2.0.0
config schema: 1
rust: 1.75.0
```

## Deprecation Policy

### When Features Are Deprecated

Features may be deprecated when:
- Better alternatives exist
- Security or stability concerns arise
- Usage data shows minimal adoption
- Maintenance burden outweighs value

### Deprecation Process

1. **Announcement**: Deprecation notice in release notes and docs
2. **Warning period**: CLI shows warnings when deprecated features are used
3. **Removal**: Feature removed in next major version

### Minimum Support Period

| Component | Support Period |
|-----------|----------------|
| Config fields | 2 major versions |
| CLI commands | 2 major versions |
| Spec schema fields | 2 major versions |

Example timeline:
```
v2.0: Feature deprecated, warnings shown
v3.0: Feature still works, warnings shown
v4.0: Feature removed
```

### Deprecation Warnings

When using deprecated features:

```bash
$ chant work 001
Warning: 'state' field is deprecated, use 'status' instead
Will be removed in v4.0
```

Config check on load:

```bash
$ chant status
Warning: Deprecated config fields detected:
  - 'workers' → use 'parallelism'

Run 'chant migrate' to update.
```

### Migration Path

All deprecations include:
- Clear warning messages
- Documentation of replacement
- Automated migration via `chant migrate`
- Version where removal occurs

## Lockfile (Future)

For reproducible behavior:

```yaml
# .chant/lock.md
---
chant_version: 2.0.0
schema_version: 1
locked_at: 2026-01-22T15:30:00Z
---

Dependency versions locked for reproducibility.
```

Not implemented in v2. Consider for enterprise use.
