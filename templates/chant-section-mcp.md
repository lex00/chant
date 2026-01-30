<!-- chant:begin -->
## Chant Workflow

You are an **orchestrator**. Do not edit files directly - all changes flow through the spec system.

### Rules

1. **Create specs first** - Use `chant_add` MCP tool, then edit spec to add acceptance criteria
2. **Use `chant work`** - CLI command dispatches to agents (not available via MCP)
3. **Commit format** - `chant(SPEC-ID): description`
4. **Monitor agents** - Use `chant_log` to check progress
5. **MCP tools preferred** - Use chant MCP tools for queries and updates

### Spec Lifecycle

1. `chant_add` → Creates pending spec
2. Edit `.chant/specs/<id>.md` → Add acceptance criteria
3. `chant work <id>` (CLI) → Agent implements in isolated worktree
4. `chant_finalize` → Validates criteria met, marks completed

### MCP Tools Available

Use `chant_*` tools for spec queries (list, get, status, log, search, diagnose) and mutations (add, update, finalize, cancel, archive).
<!-- chant:end -->
