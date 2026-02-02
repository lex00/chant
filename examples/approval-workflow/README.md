# Approval Workflow Example

This example demonstrates Chant's approval workflow, showing how specs can require human review before execution begins.

## What's Demonstrated

This example includes three specs showing different approval states:

1. **Pending approval** (`001-risky-refactor.md`) - A spec requiring approval before work can begin
2. **Approved spec** (`002-approved-feature.md`) - A spec that was reviewed and approved
3. **Rejected spec** (`003-rejected-change.md`) - A spec that was rejected with feedback

## Exploring the Example

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

## Key Features Shown

### Configuration
The `.chant/config.md` demonstrates:
- `approval.require_approval_for_agent_work: true` - Auto-require approval for agent work
- `approval.rejection_action: manual` - Manual handling of rejected specs

### Pending Approval
Spec `001-risky-refactor.md` shows:
- Created with `--needs-approval` flag
- `approval.required: true` and `approval.status: pending` in frontmatter
- Cannot be worked on until approved

### Approved Spec
Spec `002-approved-feature.md` shows:
- Approval frontmatter with approver and timestamp
- Entry in "Approval Discussion" section documenting the approval

### Rejected Spec
Spec `003-rejected-change.md` shows:
- Rejection frontmatter with rejector and timestamp
- Entry in "Approval Discussion" section with rejection reason
- Cannot be worked on until issues are addressed and re-approved

## Reproducing the Workflow

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

## Learn More

See the full guide at `docs/guides/approval-workflow.md` for:
- When to require approval
- Rejection handling modes (manual, dependency, group)
- Activity tracking
- Emergency bypass options
- Team and solo developer workflows
