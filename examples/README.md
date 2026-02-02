# Chant Examples

Real-world examples demonstrating Chant's spec-driven development workflow.

## Available Examples

### [Approval Workflow](./approval-workflow/)

Demonstrates Chant's approval workflow with three example specs showing pending, approved, and rejected states. Shows how to:
- Require human review before spec execution
- Document approval decisions in spec files
- Handle rejected specs with feedback
- Configure automatic approval requirements for agent work

Coming soon - additional examples:
- Breaking down complex projects into specs
- Using driver specs to coordinate multi-step workflows
- Handling dependencies between specs

## What These Examples Show

Each example includes:

- **Complete `.chant/` folder** - Specs, config, and logs from an actual run
- **Driver spec** - The high-level goal that gets split into actionable work
- **Member specs** - Auto-generated detailed specs with acceptance criteria
- **Execution logs** - What the agent did at each step

## Running an Example

Examples will include:

- **Complete `.chant/` folder** - Specs, config, and logs from actual runs
- **Driver spec** - The high-level goal that gets split into actionable work
- **Member specs** - Auto-generated detailed specs with acceptance criteria
- **Execution logs** - What the agent did at each step

## Contributing Examples

Have a great Chant workflow to share? Examples should:

1. Solve a real problem (not a toy example)
2. Include the complete `.chant/` folder
3. Have a README explaining the context and results
4. Show measurable outcomes (tests passing, issues closed, etc.)
