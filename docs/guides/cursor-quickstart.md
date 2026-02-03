# Cursor Quick Start Guide

A step-by-step guide for using Cursor with chant for spec-driven development.

## Prerequisites

- **Cursor IDE**: Download and install from [cursor.com](https://cursor.com)
- **Chant installed**: See the [Quick Start Guide](../getting-started/quickstart.md) for installation instructions
- **Git**: Required for chant's spec tracking

## Step 1: Initialize Cursor Integration

Navigate to your project directory and run:

```bash
chant init --agent cursor
```

This creates:
- `.cursorrules` file with AI instructions for Cursor
- `.cursor/mcp.json` for MCP (Model Context Protocol) integration

**What is MCP?** MCP lets Cursor's AI discover and use chant commands directly as tools, making spec management seamless and structured.

## Step 2: Restart Cursor

**Important:** After running `chant init --agent cursor`, you must restart Cursor to load the MCP configuration.

1. Quit Cursor completely
2. Reopen Cursor
3. Open your project folder

## Step 3: Verify MCP is Working

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

## Using Cursor with Chant

Cursor works with chant in two ways:

### Terminal Commands: Execute Specs

Run agents from the terminal:

```bash
chant work 001
```

This is the recommended approach for spec execution. The agent runs autonomously and commits when done.

### Cursor's AI: Manage Specs via MCP

Use Cursor's chat with MCP tools to:
- Browse specs: "Show me all pending specs"
- Create specs: "Create a spec for adding pagination to the user list"
- Check status: "What's the status of the project?"
- Search specs: "Find specs related to authentication"

**Important:** `chant work` is not available via MCP (it spawns an agent process). Use the terminal for running work.

## Understanding the Cursor Integration

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

## What's Next

- **Learn the basics**: See [Quick Start Guide](../getting-started/quickstart.md) for core concepts and workflows
- **Explore prompts**: Read [Prompts](../concepts/prompts.md) to customize agent behavior
- **Advanced workflows**: Check [Examples](examples.md) for real-world usage patterns
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
