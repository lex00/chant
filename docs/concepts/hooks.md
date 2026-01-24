# Hooks

## Hooks as Prompts

Hooks are prompts that run at specific points. Consistent with "everything is markdown."

```yaml
# config.md frontmatter
hooks:
  pre_work: prompts/setup.md
  post_work: prompts/cleanup.md
  on_fail: prompts/diagnose.md
```

## Hook Points

| Hook | When | Use Case |
|------|------|----------|
| `pre_work` | Before spec execution | Setup, validation |
| `post_work` | After successful completion | Cleanup, notification |
| `on_fail` | After spec failure | Diagnosis, rollback |
| `pre_commit` | Before git commit | Final validation |
| `post_split` | After creating members | Review decomposition |

## Hook Prompts

Hooks are just prompts with spec context:

```markdown
# .chant/prompts/setup.md
---
name: setup
purpose: Pre-work setup
---

Preparing to work on {{spec.id}}.

## Checks

1. Verify target files exist
2. Check for uncommitted changes
3. Ensure dependencies are met

Report any issues. Do not proceed if blocked.
```

## Example: Notification Hook

```markdown
# .chant/prompts/notify.md
---
name: notify
purpose: Post-work notification
---

Spec {{spec.id}} completed successfully.

Commit: {{spec.commit}}

Update any external trackers if configured.
```

## Example: Failure Diagnosis

```markdown
# .chant/prompts/diagnose.md
---
name: diagnose
purpose: Analyze failure
---

Spec {{spec.id}} failed.

Error: {{spec.error}}

## Instructions

1. Analyze the error
2. Check recent changes
3. Suggest fix or workaround
4. Update spec with findings

Add diagnosis to spec file under `## Failure Analysis`.
```

## Hook Execution

```rust
fn execute_spec(spec_id: &str) -> Result<()> {
    let config = load_config()?;
    let spec = load_spec(spec_id)?;

    // Pre-work hook
    if let Some(hook) = &config.hooks.pre_work {
        run_prompt(hook, &spec)?;
    }

    // Main execution
    let result = run_prompt(&spec.prompt_or_default(), &spec);

    match result {
        Ok(_) => {
            // Post-work hook
            if let Some(hook) = &config.hooks.post_work {
                run_prompt(hook, &spec)?;
            }
        }
        Err(e) => {
            // Failure hook
            if let Some(hook) = &config.hooks.on_fail {
                let _ = run_prompt(hook, &spec);  // Best effort
            }
            return Err(e);
        }
    }

    Ok(())
}
```

## Spec-Level Hooks

Override hooks per spec:

```yaml
# Spec frontmatter
---
status: pending
hooks:
  pre_work: prompts/security-check.md   # Extra validation for this spec
---
```

## Skipping Hooks

```bash
chant work 2026-01-22-001-x7m --no-hooks
```

Skips pre_work and post_work. Useful for debugging.

## Hook Failures

- `pre_work` failure → spec does not start
- `post_work` failure → warning only, spec still marked complete
- `on_fail` failure → logged, does not affect spec status

## Async Hooks (Future)

For notifications that shouldn't block:

```yaml
hooks:
  post_work:
    prompt: prompts/notify.md
    async: true   # Don't wait for completion
```
