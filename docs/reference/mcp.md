# MCP Server

## Overview

Chant exposes an MCP (Model Context Protocol) server for tool integration with AI agents.

**Role**: Chant is an MCP **server**, not client. Agents connect to Chant for tools.

## Why MCP

MCP provides a standardized way to expose tools to AI agents.

- **Some providers require MCP** - their only tool interface
- **Others benefit from MCP** - structured tool access vs text parsing
- **Some use native formats** - their own tool schemas

## Prior Art

Based on wetwire-core-go Kiro integration pattern:

```go
// Kiro uses MCP config files to discover tools
type AgentConfig struct {
    Name       string                     `json:"name"`
    Prompt     string                     `json:"prompt"`
    MCPServers map[string]MCPServerConfig `json:"mcpServers"`
    Tools      []string                   `json:"tools"`
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        AI Agent                             │
│                                                             │
│   Discovers tools via MCP protocol                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ MCP (stdio JSON-RPC)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                        chant-mcp                            │
│                                                             │
│   Exposes chant operations as MCP tools                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Internal
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                        Chant Core                           │
│                                                             │
│   Specs, state machine, git operations                      │
└─────────────────────────────────────────────────────────────┘
```

## Tools Exposed

The MCP server exposes these tools:

| Tool | Description | Parameters |
|------|-------------|------------|
| `chant_spec_get` | Get spec details | `id` |
| `chant_spec_list` | List specs | `status`, `labels` |
| `chant_spec_update` | Update spec status/output | `id`, `status`, `output` |
| `chant_commit` | Commit changes | `message`, `files` |
| `chant_verify` | Run verification | `id` |

## Provider Integration

Chant generates provider-specific MCP config files before invocation.

### Invocation Flow

```
1. chant work <id>
2. Chant writes MCP config files
3. Chant invokes provider CLI
4. Agent connects to chant-mcp for tools
5. Agent executes, uses chant tools
6. Chant captures output, updates spec
```

### Benefits with MCP

- Structured spec data (no markdown parsing)
- Direct status updates via tool calls
- Better error handling

## Implementation

### Binary

```bash
chant-mcp              # Standalone MCP server binary
# or
chant mcp-server       # Subcommand of main binary
```

### Protocol

Standard MCP over stdio:
- JSON-RPC 2.0
- Tools advertised via `tools/list`
- Tool calls via `tools/call`

### Example Tool Definition

```json
{
  "name": "chant_spec_get",
  "description": "Get details of a chant spec",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {
        "type": "string",
        "description": "Spec ID (full or partial)"
      }
    },
    "required": ["id"]
  }
}
```

### Example Tool Call

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "chant_spec_get",
    "arguments": {
      "id": "001"
    }
  },
  "id": 1
}
```

### Example Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"id\": \"2026-01-22-001-x7m\",\n  \"status\": \"in_progress\",\n  \"title\": \"Add user authentication\"\n}"
      }
    ]
  },
  "id": 1
}
```

## Configuration

```yaml
# config.md frontmatter
mcp:
  enabled: true           # Enable MCP server
  binary: chant-mcp       # Binary name/path
  tools:                  # Tools to expose
    - chant_spec_get
    - chant_spec_list
    - chant_spec_update
    - chant_commit
    - chant_verify
```

## Security

- MCP server runs locally (no network exposure)
- Inherits filesystem permissions from parent process
- Spec access limited to current project
- No credential exposure via MCP

## Phase

Part of Phase 1 (Git+ and MCP). Required before Kiro provider support.
