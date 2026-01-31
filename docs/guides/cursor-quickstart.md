# Cursor Quick Start Guide

A step-by-step guide for using Cursor with chant for spec-driven development.

## What You'll Learn

This guide walks you through:
- Installing chant
- Setting up Cursor to work with chant
- Creating and working your first spec
- Understanding the daily workflow

## Prerequisites

- **Cursor IDE**: Download and install from [cursor.com](https://cursor.com)
- **Git**: Required for chant's spec tracking
- **Terminal access**: For running chant commands

## Step 1: Install Chant

Choose the installation method that works for you:

### Homebrew (macOS/Linux)

```bash
brew install lex00/tap/chant
```

### Quick Install (Linux/macOS)

```bash
# Linux
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-linux-x86_64 -o chant
chmod +x chant
sudo mv chant /usr/local/bin/

# macOS Intel
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-macos-x86_64 -o chant
chmod +x chant
sudo mv chant /usr/local/bin/

# macOS Apple Silicon
curl -fsSL https://github.com/lex00/chant/releases/latest/download/chant-macos-aarch64 -o chant
chmod +x chant
sudo mv chant /usr/local/bin/
```

### Cargo (from source)

```bash
cargo install --git https://github.com/lex00/chant
```

**Verify installation:**

```bash
chant --version
```

For more installation options, see the [Installation Guide](../getting-started/installation.md).

## Step 2: Initialize Your Project

Navigate to your project directory and run:

```bash
chant init --agent cursor
```

This creates:
- `.chant/` directory with configuration and prompts
- `.cursorrules` file with AI instructions for Cursor
- `.cursor/mcp.json` for MCP (Model Context Protocol) integration

**What is MCP?** MCP lets Cursor's AI discover and use chant commands directly as tools, making spec management seamless and structured.

## Step 3: Restart Cursor

**Important:** After running `chant init --agent cursor`, you must restart Cursor to load the MCP configuration.

1. Quit Cursor completely
2. Reopen Cursor
3. Open your project folder

## Step 4: Verify MCP is Working

Open Cursor's AI chat and check that chant tools are available:

1. Open the Cursor chat panel
2. Look for MCP tools in the tool list (if available in UI)
3. Or ask Cursor: "What chant tools are available?"

Cursor should respond with a list of `chant_*` tools like:
- `chant_spec_list`
- `chant_spec_get`
- `chant_add`
- `chant_status`

If you don't see these tools, see [Troubleshooting](#troubleshooting) below.

## Step 5: Create Your First Spec

Create a simple spec to test the workflow:

```bash
chant add "Add a welcome message to the homepage"
```

This creates a spec file in `.chant/specs/` with a unique ID like `2026-01-30-001-abc.md`.

**Check the spec:**

```bash
chant list
```

You should see your new spec listed as `pending`.

## Step 6: Work the Spec

Execute the spec using an AI agent:

```bash
chant work 001
```

This launches an AI agent that:
1. Reads the spec and its acceptance criteria
2. Explores your codebase
3. Makes the necessary changes
4. Commits the work with a proper message

**What happens:**
- The agent spawns in a separate process
- You'll see live output as it works
- When complete, the spec status changes to `completed`

## Step 7: Review the Changes

After the agent completes:

```bash
# View git changes
git log -1

# Check the spec status
chant status

# View the completed spec
chant show 001
```

The agent creates a commit with the pattern `chant(SPEC-ID): description`.

## Daily Workflow

Once set up, your daily workflow looks like this:

### 1. Create Specs

```bash
chant add "Fix login validation bug"
chant add "Add dark mode toggle"
chant add "Update API documentation"
```

### 2. Work Specs

```bash
# Work a single spec
chant work 001

# Or work multiple specs in parallel
chant work --parallel
```

### 3. Review and Merge

```bash
# Check status
chant status

# View a spec's details
chant show 002

# List all specs
chant list
```

### 4. Use Cursor's AI with MCP

In Cursor's chat, you can ask:
- "What specs are ready to work?"
- "Create a new spec for adding user authentication"
- "What's the status of spec 003?"

Cursor will use the chant MCP tools automatically to answer these questions.

## Understanding the Setup

### `.cursorrules`

This file tells Cursor's AI how to work with chant. It contains instructions for:
- Understanding the chant workflow
- Working within specs
- Using proper commit messages
- Testing and code quality standards

**You can customize this file** to match your project's specific needs (linting tools, test commands, coding standards).

### `.cursor/mcp.json`

This file connects Cursor to the chant MCP server:

```json
{
  "mcpServers": {
    "chant": {
      "type": "stdio",
      "command": "chant",
      "args": ["mcp"]
    }
  }
}
```

**How it works:**
- Cursor starts the `chant mcp` process
- Chant exposes spec management tools
- Cursor's AI can query specs, create specs, update status, etc.

### `.chant/` Directory

```
.chant/
├── config.md             # Project configuration
├── prompts/              # Prompt templates for agents
│   ├── bootstrap.md      # Minimal bootstrap prompt
│   ├── standard.md       # Default implementation prompt
│   └── split.md          # Prompt for breaking down large specs
├── specs/                # Your spec files
├── .locks/               # PID files (gitignored)
└── .store/               # Index cache (gitignored)
```

## Using Cursor with Chant

### Option 1: Let `chant work` Run the Agent

Run agents from the terminal:

```bash
chant work 001
```

This is the recommended approach for spec execution. The agent runs autonomously and commits when done.

### Option 2: Use Cursor's AI for Spec Management

Use Cursor's chat with MCP tools to:
- Browse specs: "Show me all pending specs"
- Create specs: "Create a spec for adding pagination to the user list"
- Check status: "What's the status of the project?"
- Search specs: "Find specs related to authentication"

**Important:** `chant work` is not available via MCP (it spawns an agent process). Use the terminal for running work.

## MCP Tools Reference

When using Cursor's AI, these tools are available:

| Tool | Purpose |
|------|---------|
| `chant_spec_list` | List all specs, optionally filtered by status |
| `chant_spec_get` | Get full details of a spec |
| `chant_ready` | List specs ready to work (no unmet dependencies) |
| `chant_status` | Get project summary with spec counts |
| `chant_add` | Create a new spec |
| `chant_spec_update` | Update a spec's status or add output |
| `chant_finalize` | Mark a spec as completed |
| `chant_search` | Search specs by title and content |
| `chant_diagnose` | Diagnose issues with a spec |
| `chant_log` | Read execution log for a spec |

For full details, see [MCP Reference](../reference/mcp.md).

## Next Steps

- **Learn the philosophy**: Read [Philosophy](../getting-started/philosophy.md) to understand chant's approach
- **Explore prompts**: See [Prompts](../concepts/prompts.md) to customize agent behavior
- **Advanced workflows**: Check out [Examples](examples.md) for real-world usage patterns
- **Cursor rules reference**: See `.cursorrules` for the complete AI behavior guide

## Troubleshooting

### MCP Tools Not Showing Up

**Symptoms:** Cursor doesn't recognize `chant_*` tools

**Solutions:**
1. **Restart Cursor completely** - Quit and reopen
2. **Verify `.cursor/mcp.json` exists** - Should be in your project root
3. **Check chant is in PATH** - Run `which chant` in terminal
4. **Check Cursor logs** - Look for MCP-related errors in Cursor's output

### "Chant not initialized" Error

**Symptoms:** MCP tools return "Chant not initialized"

**Solution:** Run `chant init --agent cursor` in your project directory

### Agent Fails to Start with `chant work`

**Symptoms:** `chant work 001` fails immediately

**Solutions:**
1. **Check spec exists** - Run `chant list` to verify
2. **Verify model provider** - Check `.chant/config.md` for provider/model settings
3. **Check logs** - Look in `.chant/specs/2026-01-30-001-abc.log` for errors

### Cursor Doesn't Follow Spec Guidelines

**Symptoms:** Cursor makes changes outside of specs or doesn't commit properly

**Solution:** Ensure `.cursorrules` exists and is up to date. You can regenerate it:

```bash
# Backup existing file if you've customized it
cp .cursorrules .cursorrules.backup

# Regenerate
chant init --agent cursor --force
```

### Git Commits Don't Have Proper Format

**Symptoms:** Commits don't follow `chant(SPEC-ID): description` pattern

**Solution:** Remind Cursor of the commit format in chat, or check that `.cursorrules` has the commit guidelines.

## Common Mistakes

1. **Forgetting to restart Cursor** after running `chant init --agent cursor`
2. **Trying to run `chant work` from Cursor's AI** - Use the terminal instead
3. **Editing specs directly** while an agent is working them
4. **Not committing changes** before switching specs

## Tips for Success

1. **Write clear acceptance criteria** in your specs
2. **Keep specs focused** - One logical unit of work per spec
3. **Review agent output** before merging to main
4. **Use Cursor's AI for exploration** and spec management, not execution
5. **Customize `.cursorrules`** to match your project's standards

## Additional Resources

- [Chant Documentation](https://lex00.github.io/chant)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Cursor Documentation](https://docs.cursor.com)
- [GitHub Issues](https://github.com/lex00/chant/issues) - For bugs or feature requests
