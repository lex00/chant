# Monorepo Support

Chant supports running multiple independent projects within a monorepo. Each subdirectory with its own `.chant/` directory is treated as a separate chant project.

## Overview

In a monorepo, you can organize work by creating multiple `.chant/` directories:

```
my-monorepo/
├── frontend/
│   └── .chant/
│       ├── config.md
│       └── specs/
├── backend/
│   └── .chant/
│       ├── config.md
│       └── specs/
└── infrastructure/
    └── .chant/
        ├── config.md
        └── specs/
```

Each project operates independently with its own specs, configuration, and worktrees.

## Project Configuration

Each project needs unique configuration to avoid conflicts.

### Project Name

Set a unique `project.name` in each project's `.chant/config.md`. This name is used to namespace worktree directories to prevent collisions:

```markdown
# frontend/.chant/config.md
---
project:
  name: frontend
---
```

```markdown
# backend/.chant/config.md
---
project:
  name: backend
---
```

Worktrees will be created at:
- `/tmp/chant-frontend-{spec-id}`
- `/tmp/chant-backend-{spec-id}`

### Branch Prefix

Set a unique `defaults.branch_prefix` to isolate branches between projects:

```markdown
# frontend/.chant/config.md
---
project:
  name: frontend

defaults:
  branch_prefix: "chant/frontend/"
---
```

```markdown
# backend/.chant/config.md
---
project:
  name: backend

defaults:
  branch_prefix: "chant/backend/"
---
```

This ensures branches are organized:
- `chant/frontend/2026-02-06-001-abc`
- `chant/backend/2026-02-06-001-xyz`

## MCP Server Setup

Each project needs its own MCP server instance. MCP discovery automatically walks up the directory tree to find the nearest `.chant/` directory, but you need to configure separate server instances in your MCP client.

### Claude Desktop Configuration

Configure multiple MCP servers in your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "chant-frontend": {
      "command": "chant",
      "args": ["mcp"],
      "env": {
        "CHANT_CWD": "/path/to/monorepo/frontend"
      }
    },
    "chant-backend": {
      "command": "chant",
      "args": ["mcp"],
      "env": {
        "CHANT_CWD": "/path/to/monorepo/backend"
      }
    }
  }
}
```

The `CHANT_CWD` environment variable tells the MCP server which directory to operate in.

### Command Line MCP

When running MCP from the command line, specify the working directory:

```bash
# For frontend project
cd frontend && chant mcp

# For backend project
cd backend && chant mcp
```

Or use the `--cwd` flag if available:

```bash
chant mcp --cwd frontend
chant mcp --cwd backend
```

## Cross-Project Visibility

Use `chant list --global` to see specs across all projects in your repository:

```bash
# From any directory in the monorepo
chant list --global
```

This scans the entire git repository for `.chant/` directories and lists specs from all projects, showing which project each spec belongs to.

## Example Setup

Complete monorepo configuration:

```
my-app/
├── web/
│   ├── .chant/
│   │   ├── config.md
│   │   └── specs/
│   │       └── 2026-02-06-001-abc.md
│   └── src/
├── api/
│   ├── .chant/
│   │   ├── config.md
│   │   └── specs/
│   │       └── 2026-02-06-002-xyz.md
│   └── src/
└── shared/
    └── src/
```

**web/.chant/config.md:**
```markdown
---
project:
  name: web

defaults:
  branch_prefix: "chant/web/"
  prompt: bootstrap
---

# Web Application

Frontend React application.
```

**api/.chant/config.md:**
```markdown
---
project:
  name: api

defaults:
  branch_prefix: "chant/api/"
  prompt: bootstrap
---

# API Service

Backend API service.
```

## Working with Projects

### Starting Work

Navigate to the project directory and run chant commands:

```bash
cd web
chant list
chant work --ready

cd ../api
chant list
chant work --ready
```

### Viewing Status

Check individual project status:

```bash
cd web
chant status
```

Or view all projects:

```bash
# From repository root
chant list --global
```

### Creating Specs

Create specs in the context of the project:

```bash
cd web
chant add "Add dark mode toggle"

cd ../api
chant add "Add rate limiting middleware"
```

## Best Practices

1. **Unique names**: Always set unique `project.name` values to prevent worktree collisions
2. **Branch isolation**: Use distinct `branch_prefix` values for clean branch organization
3. **Independent configs**: Each project should have its own configuration that matches its workflow
4. **MCP instances**: Run separate MCP servers for each project when using MCP integration
5. **Clear boundaries**: Structure projects to minimize cross-project dependencies in specs

## Limitations

- Each MCP server instance can only operate on one project at a time
- Cross-project specs are not supported (specs cannot span multiple `.chant/` directories)
- `chant watch` can only monitor one project per process
- Parallel work (`chant work --parallel`) operates within a single project
