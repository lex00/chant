# Operations Layer

The operations layer provides canonical business logic for spec manipulation, preventing the common anti-pattern where CLI and MCP re-implement the same logic with subtle differences.

## The Problem

Before the operations layer existed, chant's MCP server duplicated CLI command logic. When we fixed a bug in `chant finalize`, the same bug persisted in the MCP handler. When we added validation to `chant reset`, the MCP endpoint had different behavior. Every feature required two implementations that inevitably diverged.

This violated DRY and created maintenance burden: bug fixes didn't propagate, validation rules were inconsistent, and behavior differed depending on whether you used the CLI or MCP.

## Architecture

The operations layer sits between interface handlers (CLI/MCP) and the domain layer (spec files, state machine, repository):

```
CLI (clap)  ──┐
               ├──▶ operations/ ──▶ domain layer
MCP (JSON-RPC)─┘
```

Both CLI commands and MCP handlers route through the same operations functions. This ensures:

- **Single source of truth**: Business logic lives in one place
- **Consistent validation**: All interfaces enforce the same rules
- **Unified behavior**: CLI and MCP behave identically
- **Easier testing**: Test operations once instead of per-interface

## Current Operations

| Module | Description |
|--------|-------------|
| `archive.rs` | Move completed specs to archive directory |
| `cancel.rs` | Cancel specs and mark them as cancelled |
| `commits.rs` | Auto-detect and associate git commits with specs |
| `create.rs` | Create new specs with ID generation and template application |
| `finalize.rs` | Mark specs as completed with validation and state checks |
| `model.rs` | Update model configuration for specs |
| `pause.rs` | Pause running work processes for specs |
| `reset.rs` | Reset failed or in-progress specs back to pending |
| `update.rs` | Update spec frontmatter fields and append output |
| `verify.rs` | Verify specs meet their acceptance criteria |

### `create.rs` — Spec Creation

Creates new specs with ID generation, template application, derivation, and git auto-commit.

**Key responsibilities:**
- Generate unique spec ID based on current date
- Split long descriptions into title + body
- Apply prompt templates if specified
- Run derivation engine for enterprise fields
- Auto-commit to git (unless disabled or `.chant/` is gitignored)

**Usage:**
```rust
use chant::operations::create::{create_spec, CreateOptions};

let (spec, path) = create_spec(
    "Add user authentication",
    &specs_dir,
    &config,
    CreateOptions {
        prompt: Some("feature".to_string()),
        needs_approval: false,
        auto_commit: true,
    },
)?;
```

### `finalize.rs` — Spec Completion

Marks specs as completed with full validation and state consistency checks.

**Key responsibilities:**
- Check for uncommitted changes in worktree
- Validate driver/member relationships (drivers can't complete with incomplete members)
- Auto-detect commits (or accept provided list)
- Check for agent co-authorship and set approval requirements
- Update status, `completed_at` timestamp, model field
- Verify persistence (reload and validate saved state)

**Usage:**
```rust
use chant::operations::finalize::{finalize_spec, FinalizeOptions};

finalize_spec(
    &mut spec,
    &spec_repo,
    &config,
    &all_specs,
    FinalizeOptions {
        allow_no_commits: false,
        commits: Some(vec!["abc123".to_string()]),
        force: false,
    },
)?;
```

**Validation performed:**
- Uncommitted changes block finalization
- Driver specs require all members completed first
- Completed specs must have valid ISO timestamps
- Persistence is verified by reloading from disk

### `reset.rs` — Failure Recovery

Resets failed or in-progress specs back to pending status.

**Key responsibilities:**
- Validate spec is in `failed` or `in_progress` state
- Transition to `pending` via state machine
- Persist the status change
- Optionally re-execute (parameter exists but not yet implemented)

**Usage:**
```rust
use chant::operations::reset::{reset_spec, ResetOptions};

reset_spec(
    &mut spec,
    &spec_path,
    ResetOptions {
        re_execute: false,
        prompt: None,
        branch: None,
    },
)?;
```

**Constraints:**
- Only `failed` and `in_progress` specs can be reset
- State transitions are validated by the spec state machine

### `update.rs` — Field Mutations

Updates spec frontmatter fields with selective preservation.

**Key responsibilities:**
- Update status (using `force_status` for MCP compatibility)
- Set dependencies, labels, target files, model
- Append output text to spec body
- Persist changes

**Usage:**
```rust
use chant::operations::update::{update_spec, UpdateOptions};

update_spec(
    &mut spec,
    &spec_path,
    UpdateOptions {
        status: Some(SpecStatus::InProgress),
        labels: Some(vec!["bug".to_string(), "p0".to_string()]),
        output: Some("Progress update: completed phase 1".to_string()),
        ..Default::default()
    },
)?;
```

**Output handling:**
- Appends text to spec body with `## Output\n\n` header
- Preserves existing body content
- Ensures proper newline spacing

## How to Add a New Operation

When you need a new spec operation (e.g., `archive`, `split`, `merge`):

1. **Create `src/operations/{operation}.rs`**
   - Define an `Options` struct for parameters
   - Implement the operation function taking `&mut Spec`, options, and dependencies
   - Use the spec state machine for status transitions
   - Perform all validation and business logic here

2. **Export from `src/operations/mod.rs`**
   ```rust
   pub mod archive;
   pub use archive::{archive_spec, ArchiveOptions};
   ```

3. **Add CLI command in `src/cmd/`**
   - Parse arguments with clap
   - Load spec and dependencies
   - Call operation function
   - Handle errors and output

4. **Add MCP handler in `src/mcp/tools/`**
   - Parse JSON-RPC parameters
   - Load spec and dependencies
   - Call the **same operation function**
   - Return JSON response

5. **Write tests in `tests/operations/`**
   - Test the operation directly (not via CLI/MCP)
   - Cover validation, state transitions, edge cases
   - Use `TestHarness` and `SpecFactory` for setup

## What Goes Where

**Operations layer** (`src/operations/`):
- Spec manipulation business logic
- Validation rules
- State transitions and persistence
- Anything that should behave identically across interfaces

**CLI layer** (`src/cmd/`):
- Argument parsing (clap)
- Terminal output formatting
- Interactive prompts
- Shell-specific concerns (exit codes, colored output)

**MCP layer** (`src/mcp/tools/`):
- JSON-RPC request/response handling
- Parameter deserialization
- Error formatting for MCP protocol
- MCP-specific features (notifications, progress)

**Domain layer** (`src/spec/`, `src/repository/`, etc.):
- Core data structures (`Spec`, `SpecStatus`)
- File I/O and parsing
- State machine transitions
- Low-level primitives

**Rule of thumb**: If CLI and MCP need to do it the same way, it belongs in operations. If it's interface-specific (formatting, protocol details), it belongs in the handler.

## Examples

### Adding an Archive Operation

```rust
// src/operations/archive.rs
use anyhow::Result;
use std::path::Path;
use crate::spec::{Spec, SpecStatus};

pub struct ArchiveOptions {
    pub archive_dir: PathBuf,
}

pub fn archive_spec(
    spec: &Spec,
    spec_path: &Path,
    options: ArchiveOptions,
) -> Result<()> {
    // Validation
    if spec.frontmatter.status != SpecStatus::Completed {
        anyhow::bail!("Only completed specs can be archived");
    }

    // Business logic
    let archive_path = options.archive_dir.join(spec_path.file_name().unwrap());
    std::fs::rename(spec_path, &archive_path)?;

    Ok(())
}
```

### Using from CLI

```rust
// src/cmd/archive.rs
use clap::Args;
use chant::operations::archive::{archive_spec, ArchiveOptions};

#[derive(Args)]
pub struct ArchiveArgs {
    id: String,
}

pub fn run(args: ArchiveArgs, config: &Config) -> Result<()> {
    let spec = load_spec(&args.id)?;
    let spec_path = get_spec_path(&args.id)?;

    archive_spec(
        &spec,
        &spec_path,
        ArchiveOptions {
            archive_dir: config.archive_dir.clone(),
        },
    )?;

    println!("Archived spec {}", args.id);
    Ok(())
}
```

### Using from MCP

```rust
// src/mcp/tools/lifecycle.rs
pub fn tool_chant_archive(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = mcp_ensure_initialized()?;
    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;
    let id = args.get("id").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let options = operations::ArchiveOptions::default();
    match operations::archive_spec(&specs_dir, id, &options) {
        Ok(dest_path) => Ok(json!({
            "content": [{ "type": "text", "text": format!("Archived spec: {}", id) }]
        })),
        Err(e) => Ok(json!({
            "content": [{ "type": "text", "text": e.to_string() }],
            "isError": true
        })),
    }
}
```

Notice how both CLI and MCP call the **same** `archive_spec` function with identical parameters. Validation, business logic, and file operations happen once in the operations layer.
