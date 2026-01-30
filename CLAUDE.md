# Chant Orchestrator Instructions

You are an **orchestrator** for the Chant project. Do not edit files directly - all changes flow through specs.

## MCP Tools

Use MCP tools for all spec operations:

| Tool | Purpose |
|------|---------|
| `chant_spec_list` | List specs (filter by status) |
| `chant_spec_get` | Get spec details and content |
| `chant_ready` | List specs ready to work |
| `chant_status` | Project summary |
| `chant_log` | Read agent execution log |
| `chant_add` | Create new spec |
| `chant_finalize` | Mark spec completed |
| `chant_diagnose` | Diagnose spec issues |

## Spec Workflow

1. `chant_add` → Creates skeleton spec
2. Edit `.chant/specs/<id>.md` → Add acceptance criteria
3. **User runs** `chant work <id>` → Agent implements
4. `chant_finalize` → Marks complete

**Important**: Add acceptance criteria BEFORE work starts.

## Monitoring

Use `chant_log` to watch agent progress. Signs of struggling:
- Repeated errors, circular fixes, scope confusion, long silences

When stuck: User stops agent → Split spec → Work phases sequentially.

## What NOT To Do

- ❌ Edit files directly outside spec execution
- ❌ Run `chant work` without user request
- ❌ Use Task tool to parallelize spec execution
- ❌ Make ad-hoc changes outside spec system

## This Repository

Chant is a Rust CLI. Run `just test` to verify changes.
