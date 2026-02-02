---
approval:
  require_approval_for_agent_work: true
  rejection_action: manual
---

# Approval Workflow Example Configuration

This configuration demonstrates approval workflow settings for agent-assisted work.

## Settings Explained

### `require_approval_for_agent_work: true`

When enabled, chant automatically requires approval for any spec where commits include AI agent co-authorship (e.g., `Co-Authored-By: Claude`). This ensures human review of all agent-written code before it can be merged.

### `rejection_action: manual`

When a spec is rejected, it stays in rejected state. The author must:
1. Read the rejection reason in the "Approval Discussion" section
2. Edit the spec to address the feedback
3. Request re-approval

Other options:
- `dependency` - Creates a new spec to fix issues, blocks original on it
- `group` - Converts to driver spec with numbered member specs
