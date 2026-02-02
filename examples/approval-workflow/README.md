# Approval Workflow Example

## Overview

This example demonstrates Chant's approval workflow, showing how specs can require human review before execution begins. It includes three specs showing different approval states: pending, approved, and rejected.

## Structure

This example includes three specs:

1. **001-risky-refactor.md** - A spec requiring approval before work can begin (pending approval)
2. **002-approved-feature.md** - A spec that was reviewed and approved
3. **003-rejected-change.md** - A spec that was rejected with feedback

All three specs demonstrate independent execution patterns - each can be worked on separately with `chant work <spec-id>` after appropriate approval.

## Usage

View the configuration:
```bash
cd examples/approval-workflow
cat .chant/config.md
```

List specs with approval status:
```bash
chant list --approval pending
chant list --approval approved
chant list --approval rejected
```

View a specific spec:
```bash
chant show 001
```

Create a spec requiring approval:
```bash
chant add "Your task description" --needs-approval
```

Review and approve:
```bash
chant show spec-id
chant approve spec-id --by your-name
```

Or reject with feedback:
```bash
chant reject spec-id --by your-name --reason "Explanation of issues"
```

Execute after approval:
```bash
chant work spec-id
```

## Testing

The configuration demonstrates:
- `approval.require_approval_for_agent_work: true` - Auto-require approval for agent work
- `approval.rejection_action: manual` - Manual handling of rejected specs

Test the workflow by:
1. Creating a spec with `--needs-approval` flag
2. Reviewing with `chant show`
3. Approving or rejecting with reasons
4. Executing approved specs with `chant work`

Learn more in the full guide at `docs/guides/approval-workflow.md`.
