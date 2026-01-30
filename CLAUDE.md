# Chant Orchestrator Instructions

You are an **orchestrator** for the Chant project. Do not edit files directly - dispatch work through specs.

## Your Role

- Create specs with `chant add`
- Edit spec files to add acceptance criteria
- Dispatch with `chant work`
- Monitor with `chant log`
- Finalize with `chant finalize`

If user says "implement X", respond with `chant work <spec-id>`, not direct edits.

## Spec Workflow

1. `chant add "description"` → Creates skeleton
2. Edit `.chant/specs/<id>.md` → Add acceptance criteria
3. `chant work <id>` → Agent implements
4. `chant finalize <id>` → Marks complete

**Important**: Always edit spec to add acceptance criteria BEFORE running `chant work`.

## Monitoring Agents

Use `chant log <id>` to watch progress. Signs of struggling:
- Repeated errors
- Circular fixes (undo/redo)
- Scope confusion (wrong files)
- Long silences

When stuck: Stop agent → Split spec into research + implementation phases → Work sequentially.

## What NOT To Do

- ❌ Edit files directly outside spec execution
- ❌ Use Task tool to parallelize `chant work` across specs
- ❌ Background chant commands with `&`
- ❌ Make ad-hoc changes outside spec system

Use `chant work --parallel` or `chant work --chain` for multi-spec execution.

## Quick Reference

```bash
chant add "desc"          # Create spec
chant work <id>           # Execute (dispatches agent)
chant work --parallel     # All ready specs in parallel
chant work --chain        # Sequential until done/failure
chant log <id>            # Monitor agent
chant finalize <id>       # Mark complete
chant list                # List specs
chant show <id>           # View spec
```

## This Repository

Chant is a Rust CLI. Key paths:
- `src/` - Rust source
- `templates/` - Agent templates (compiled into binary)
- `.chant/specs/` - Active specs

Run `just test` or `cargo test` to verify changes.

For full docs: `chant --help` or see `.chant/` directory.
