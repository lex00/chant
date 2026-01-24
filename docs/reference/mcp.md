# MCP Server

## Overview

Chant exposes an MCP (Model Context Protocol) server for tool integration with AI agents.

**Role**: Chant is an MCP **server**, not client. Agents connect to Chant for tools.

## Usage

```bash
# Start MCP server (reads from stdin, writes to stdout)
chant mcp
```

The server reads JSON-RPC 2.0 requests from stdin and writes responses to stdout.

### Testing Manually

```bash
# List available tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | chant mcp

# Initialize the server
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | chant mcp

# List specs
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"chant_spec_list","arguments":{}},"id":1}' | chant mcp

# Get a specific spec (partial ID match)
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"chant_spec_get","arguments":{"id":"001"}},"id":1}' | chant mcp
```

## Why MCP

MCP provides a standardized way to expose tools to AI agents.

- **Some providers require MCP** - their only tool interface
- **Others benefit from MCP** - structured tool access vs text parsing
- **Some use native formats** - their own tool schemas

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
│                        chant mcp                            │
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

## Tools

The MCP server exposes these tools:

| Tool | Description | Parameters |
|------|-------------|------------|
| `chant_spec_list` | List all specs | `status` (optional) |
| `chant_spec_get` | Get spec details | `id` (required, partial match supported) |
| `chant_spec_update` | Update spec status/output | `id` (required), `status`, `output` (optional) |

### chant_spec_list

List all chant specs in the current project.

**Parameters:**
- `status` (optional): Filter by status - `pending`, `in_progress`, `completed`, `failed`

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "chant_spec_list",
    "arguments": {
      "status": "in_progress"
    }
  },
  "id": 1
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "[\n  {\n    \"id\": \"2026-01-22-001-x7m\",\n    \"title\": \"Add user authentication\",\n    \"status\": \"in_progress\",\n    \"type\": \"feature\"\n  }\n]"
      }
    ]
  },
  "id": 1
}
```

### chant_spec_get

Get details of a specific chant spec.

**Parameters:**
- `id` (required): Spec ID (full or partial match)

**Example Request:**
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

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"id\": \"2026-01-22-001-x7m\",\n  \"title\": \"Add user authentication\",\n  \"status\": \"in_progress\",\n  \"type\": \"feature\",\n  \"body\": \"## Description\\n\\nImplement user auth...\"\n}"
      }
    ]
  },
  "id": 1
}
```

### chant_spec_update

Update a chant spec status or append output.

**Parameters:**
- `id` (required): Spec ID (full or partial match)
- `status` (optional): New status - `pending`, `in_progress`, `completed`, `failed`
- `output` (optional): Output text to append to spec body

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "chant_spec_update",
    "arguments": {
      "id": "001",
      "status": "completed",
      "output": "Implementation complete. All tests passing."
    }
  },
  "id": 1
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Updated spec: 2026-01-22-001-x7m"
      }
    ]
  },
  "id": 1
}
```

## Tool Schemas

Full JSON schemas as returned by `tools/list`:

```json
{
  "tools": [
    {
      "name": "chant_spec_list",
      "description": "List all chant specs in the current project",
      "inputSchema": {
        "type": "object",
        "properties": {
          "status": {
            "type": "string",
            "description": "Filter by status (pending, in_progress, completed, failed)"
          }
        }
      }
    },
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
    },
    {
      "name": "chant_spec_update",
      "description": "Update a chant spec status or add output",
      "inputSchema": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string",
            "description": "Spec ID (full or partial)"
          },
          "status": {
            "type": "string",
            "description": "New status (pending, in_progress, completed, failed)"
          },
          "output": {
            "type": "string",
            "description": "Output text to append to spec body"
          }
        },
        "required": ["id"]
      }
    }
  ]
}
```

## Protocol

Standard MCP over stdio:
- JSON-RPC 2.0
- Tools advertised via `tools/list`
- Tool calls via `tools/call`
- Server info via `initialize`

### Initialize

```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "id": 1
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {}
    },
    "serverInfo": {
      "name": "chant",
      "version": "0.1.0"
    }
  },
  "id": 1
}
```

## Provider Integration

Chant generates provider-specific MCP config files before invocation.

### Invocation Flow

```
1. chant work <id>
2. Chant writes MCP config files
3. Chant invokes provider CLI
4. Agent connects to chant mcp for tools
5. Agent executes, uses chant tools
6. Chant captures output, updates spec
```

### Benefits with MCP

- Structured spec data (no markdown parsing)
- Direct status updates via tool calls
- Better error handling

## Security

- MCP server runs locally (no network exposure)
- Inherits filesystem permissions from parent process
- Spec access limited to current project
- No credential exposure via MCP
