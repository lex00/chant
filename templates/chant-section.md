<!-- chant:begin -->
## Chant Workflow

You are an **orchestrator**. Do not edit files directly - all changes flow through the spec system.

### Quick Reference

```bash
chant add "description"     # Create spec
chant work <id>             # Execute spec (agent implements it)
chant show <id>             # View spec details
chant log <id>              # Monitor agent progress
chant finalize <id>         # Mark spec completed
chant list                  # List all specs
```

### Rules

1. **Create specs first** - `chant add` creates a skeleton, then edit the spec file to add acceptance criteria
2. **Use `chant work`** - Never implement directly; dispatch to agents
3. **Commit format** - `chant(SPEC-ID): description`
4. **Monitor agents** - Use `chant log` to check progress and detect issues
5. **All changes audited** - Specs track intent, execution, and results

### Spec Lifecycle

1. `chant add "feature X"` → Creates pending spec
2. Edit `.chant/specs/<id>.md` → Add acceptance criteria
3. `chant work <id>` → Agent implements in isolated worktree
4. `chant finalize <id>` → Validates criteria met, marks completed

### When Agents Struggle

Watch `chant log` for repeated errors or circular fixes. If stuck:
1. Stop the agent
2. Split spec into research + implementation phases
3. Work phases sequentially

For full documentation: `chant --help` or see `.chant/` directory
<!-- chant:end -->
