# Chant Examples

Real-world examples demonstrating Chant's spec-driven development workflow.

## Available Examples

| Example | Description | Complexity |
|---------|-------------|------------|
| [SwiftyJSON Swift 6 Upgrade](./swiftyjson/) | Upgrade a 23k-star library to Swift 6 with Sendable support | Medium |

## What These Examples Show

Each example includes:

- **Complete `.chant/` folder** - Specs, config, and logs from an actual run
- **Driver spec** - The high-level goal that gets split into actionable work
- **Member specs** - Auto-generated detailed specs with acceptance criteria
- **Execution logs** - What the agent did at each step

## Running an Example

To explore an example without executing:

```bash
cd examples/swiftyjson
ls chant/specs/           # View the specs
cat chant/specs/swift6-upgrade.md   # Read the driver
```

To reproduce the workflow on the actual project:

```bash
# Clone the target project
git clone https://github.com/SwiftyJSON/SwiftyJSON.git
cd SwiftyJSON

# Copy the example's chant folder
cp -r /path/to/examples/swiftyjson/chant .chant

# Reset specs to pending and run
# (or just chant init and create the driver spec)
chant init
chant add "Upgrade to Swift 6 with Sendable support"
chant split <spec-id>
chant work <spec-id>
```

## Contributing Examples

Have a great Chant workflow to share? Examples should:

1. Solve a real problem (not a toy example)
2. Include the complete `.chant/` folder
3. Have a README explaining the context and results
4. Show measurable outcomes (tests passing, issues closed, etc.)
