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

### Planned Tools

> **Status: Planned** - These tools are on the roadmap but not yet implemented.

| Tool | Description | Parameters |
|------|-------------|------------|
| `chant_commit` | Commit changes | `message`, `files` |
| `chant_verify` | Run verification | `id` |

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

#### Output Append Behavior

When `output` is provided, the text is appended to the spec body under an `## Output` section. Important characteristics:

- **No timestamp**: Unlike agent-driven workflow outputs, MCP appended output does not include automatic timestamps
- **No truncation**: Long output strings are not automatically truncated (the caller is responsible for managing output size)
- **Section header**: Output is placed under an `## Output` markdown header for organization
- **Formatting**: Output is appended as plain text without automatic code block wrapping

This differs from the standard `append_agent_output` function used in regular spec execution, which includes timestamps, truncation logic, and automatic code block formatting.

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

**Example Resulting Spec Body:**
```
# Feature Implementation

Some initial content...

## Output

Implementation complete. All tests passing.
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
      "description": "Update a chant spec status or append output. Output is appended under an '## Output' section without timestamps or automatic truncation.",
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
            "description": "Output text to append to spec body. Appended under '## Output' section without timestamps or automatic code block wrapping."
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

## Error Codes and Response Structures

### JSON-RPC 2.0 Error Codes

The chant MCP server uses standard JSON-RPC 2.0 error codes for protocol-level errors:

| Code | Message | Description | When It Occurs |
|------|---------|-------------|----------------|
| `-32700` | Parse error | Request JSON is malformed or not valid JSON | Invalid JSON sent to stdin |
| `-32600` | Invalid JSON-RPC version | Request has `jsonrpc` field != "2.0" | Version mismatch in request |
| `-32603` | Server error | Internal server error during tool execution | Tool function throws an exception or returns `Err` |

### Error Response Structure

All error responses follow the JSON-RPC 2.0 error format:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Error description",
    "data": null
  },
  "id": <request-id>
}
```

**Fields:**
- `jsonrpc`: Always `"2.0"`
- `error.code`: Integer error code
- `error.message`: Human-readable error message
- `error.data`: Optional additional error context (currently unused)
- `id`: Echo of the request ID

### Tool-Level Error Responses

Tools return structured error responses as MCP tool results (not JSON-RPC errors). Tool errors are wrapped in content objects:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Error description"
      }
    ],
    "isError": true
  },
  "id": <request-id>
}
```

**Common Tool-Level Errors:**

| Error | Condition | Tool(s) |
|-------|-----------|---------|
| "Chant not initialized" | `.chant/specs` directory doesn't exist | All tools |
| "Missing required parameter: id" | `id` parameter not provided | `chant_spec_get`, `chant_spec_update` |
| "Missing required parameter: name" | `name` parameter not provided | `tools/call` |
| "Missing tool name" | Tool `name` is not a string or missing | `tools/call` |
| "Missing arguments" | `arguments` not provided to `tools/call` | `tools/call` |
| "Method not found" | Unknown method requested | Protocol level |
| "Unknown tool" | Tool name doesn't match available tools | `tools/call` |
| "Invalid status" | Status string not in `[pending, in_progress, completed, failed]` | `chant_spec_update` |
| "No updates specified" | Neither `status` nor `output` parameter provided | `chant_spec_update` |

### Success Response Structure

Successful tool results use this format:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Success message or data"
      }
    ]
  },
  "id": <request-id>
}
```

**Fields:**
- `jsonrpc`: Always `"2.0"`
- `result.content`: Array of content objects
- `content[].type`: Currently always `"text"`
- `content[].text`: The response data as formatted text
- `id`: Echo of the request ID

### Notifications (No Response)

Requests without an `id` field are notifications and receive no response:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

Currently supported notifications:
- `notifications/initialized`: Client notifies server it's ready (no action taken)

### Response Content Types

The `content[].type` field in responses can be:
- `"text"`: Plain text or JSON-formatted data (current implementation)
- Future: `"tool_result"`, `"resource"` (per MCP spec)

### Error Handling Best Practices

1. **Check `jsonrpc` and `error` fields**: Distinguish between protocol errors and tool errors
   - If `error` is present, it's a protocol-level error
   - If `result` contains `isError: true`, it's a tool-level error

2. **Handle missing initialization**: Always check for "Chant not initialized" before using tools

3. **Validate parameters**: Tools will return descriptive errors for missing/invalid parameters

4. **Parse tool output**: Tool responses have JSON in the `text` field - parse it accordingly

### Example Error Scenarios

**Scenario 1: Parse Error**
```bash
echo 'invalid json' | chant mcp
```
Response:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32700,
    "message": "Parse error: expected value at line 1 column 1"
  },
  "id": null
}
```

**Scenario 2: Missing Required Parameter**
```bash
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"chant_spec_get","arguments":{}},"id":1}' | chant mcp
```
Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Missing required parameter: id"
      }
    ],
    "isError": true
  },
  "id": 1
}
```

**Scenario 3: Chant Not Initialized**
```bash
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"chant_spec_list","arguments":{}},"id":1}' | chant mcp
```
Response (when `.chant/specs` doesn't exist):
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Chant not initialized. Run `chant init` first."
      }
    ],
    "isError": true
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
