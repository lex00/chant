# Chant Examples

Real-world examples demonstrating Chant's spec-driven development workflow.

## Available Examples

### [Approval Workflow](./approval-workflow/)

Demonstrates Chant's approval workflow with three example specs showing pending, approved, and rejected states. Shows how to:
- Require human review before spec execution
- Document approval decisions in spec files
- Handle rejected specs with feedback
- Configure automatic approval requirements for agent work

### [KPI/OKR Workflow](./kpi-okr-workflow/)

Demonstrates using Chant to tackle a business KPI (reducing customer churn from 8% to 5%). Shows how to:
- Ingest external data into `.chant/context/` for agent analysis
- Use research specs with `informed_by:` to reference context files
- Coordinate parallel implementation with driver specs and member specs
- Bridge business objectives to technical implementation

### [TDD Workflow](./tdd-workflow/)

Demonstrates Test-Driven Development with Chant. Shows how to:
- Analyze test coverage gaps with research specs
- Store team test standards in `.chant/context/` for agent reference
- Write acceptance criteria as test cases
- Coordinate parallel test implementation with driver and member specs
- Increase test coverage systematically (38% → 85%)

### [Research Workflow](./research-workflow/)

Demonstrates using Chant for research with two main patterns. Shows how to:
- Use `informed_by:` to reference source materials for synthesis or analysis
- Structure research questions as checkboxes for tracking progress
- Synthesize multiple papers into findings (academic path)
- Analyze codebase for technical debt (developer path)
- Leverage drift detection when source materials change

### [OSS Maintainer Workflow](./oss-maintainer-workflow/)

Demonstrates a 6-phase research-driven bug fix workflow for open source maintainers. Shows how to:
- Systematically investigate complex bugs through comprehension and reproduction phases
- Conduct root cause analysis and impact assessment before implementing fixes
- Use research specs with `target_files:` to document findings
- Build informed chains with `informed_by:` across multiple investigation phases
- Stage fixes in fork before creating upstream PRs
- Implement human gates for critical decision points
- Handle concurrent write bugs with proper analysis and testing

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
