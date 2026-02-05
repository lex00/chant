# Quick Start

Get chant working in 5 minutes. By the end, you'll have an AI agent create a script for you.

## Prerequisites

- [chant installed](installation.md)
- One of these AI coding CLIs:
  - **Claude Code**: `claude` command ([install](https://claude.ai/code))
  - **Kiro CLI**: `kiro-cli-chat` command ([install](https://kiro.dev/docs/cli))

## Step 1: Initialize a Project

Create a test project:

```bash
mkdir chant-quickstart && cd chant-quickstart
git init
```

Run the wizard:

```bash
chant init
```

The wizard asks:
1. **Project name** - accept the default
2. **Provider** - choose `claude` or `kirocli`
3. **Model** - choose `sonnet` (fast and capable)
4. **Agent config** - choose your CLI (Claude Code or Kiro)

> **Quick setup**: Skip the wizard with flags:
> ```bash
> # For Claude Code
> chant init --provider claude --model sonnet --agent claude
>
> # For Kiro CLI
> chant init --provider kirocli --model sonnet --agent kiro
> ```

## Step 2: Create a Spec

Add a simple task:

```bash
chant add "Create a hello.sh script that prints Hello World"
```

This creates a minimal spec. Check what lint thinks:

```bash
chant lint
```

You'll see warnings like:
```
⚠ No acceptance criteria found
⚠ Description is too brief
```

## Step 3: Improve the Spec

Open the spec file (shown in the `chant add` output) and add acceptance criteria:

```bash
chant show 001  # View the spec
```

Edit `.chant/specs/[your-spec-id].md` to look like this:

```markdown
---
status: pending
---

# Create a hello.sh script that prints Hello World

Create a bash script that outputs a greeting.

## Acceptance Criteria

- [ ] Creates `hello.sh` in the project root
- [ ] Script is executable (`chmod +x`)
- [ ] Running `./hello.sh` prints "Hello World"
- [ ] Script includes a shebang line (`#!/bin/bash`)
```

Run lint again:

```bash
chant lint
```

Now it passes. The spec is ready for execution.

## Step 4: Start Your Agent CLI

Open a new terminal in the same directory and start your AI CLI:

**Claude Code:**
```bash
claude
```

**Kiro CLI:**
```bash
kiro-cli-chat chat
```

## Step 5: Use MCP Tools to Execute

Inside your agent CLI, the chant MCP tools are available. Try these commands:

**Check project status:**
```
Use chant_status to show the project status
```

You'll see: `1 pending | 0 in_progress | 0 completed`

**View the spec:**
```
Use chant_spec_get to show spec 001
```

This displays your spec with its acceptance criteria.

**Start working on it:**
```
Use chant_work_start to begin working on spec 001
```

The agent will:
1. Read the spec and acceptance criteria
2. Create `hello.sh` with the required content
3. Make it executable
4. Commit with message: `chant(001): Create hello.sh script`

**Monitor progress:**
```
Use chant_status to check progress
```

You'll see: `0 pending | 0 in_progress | 1 completed`

## Step 6: Verify the Result

Back in your original terminal:

```bash
# Check the script exists
ls -la hello.sh

# Run it
./hello.sh
# Output: Hello World

# View the commit
git log -1 --oneline

# Check spec status
chant list
```

## What Just Happened?

1. **chant init** - Set up project config and MCP integration
2. **chant add** - Created a spec (work intention)
3. **chant lint** - Validated spec quality
4. **MCP tools** - Let the agent discover and execute the spec
5. **Agent** - Read the spec, wrote code, committed changes

The key insight: **you defined the goal, the agent figured out how to achieve it**.

## Next Steps

| Want to... | Do this... |
|------------|------------|
| Add more specs | `chant add "your task"` |
| Run from CLI | `chant work 001` |
| Run multiple specs | `chant work --chain` |
| See all commands | `chant --help` |

## Learn More

- [Specs](../concepts/specs.md) - How to write effective specs
- [Prompts](../concepts/prompts.md) - Customize agent behavior
- [Providers](../reference/providers.md) - Configure different AI providers
- [MCP Tools](../reference/mcp.md) - All available MCP operations
