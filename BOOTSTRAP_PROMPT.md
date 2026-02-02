# Chant Bootstrap Prompt

Use this prompt to bootstrap chant from scratch in a new repository.

---

## Context

You are implementing chant from the documentation at this repository.

- Full documentation is in `docs/` - read it to understand the architecture
- Language: **Rust** (see `docs/architecture/stack.md`)
- This project proves that AI can build software through executable specs

## Phase 0: Bootstrap Core (Manual Implementation)

Build the ~700 line core manually. This is the ONLY phase where you write code directly.

### Deliverables

1. Spec parser (markdown + YAML frontmatter)
2. State machine (pending → in_progress → completed/failed)
3. CLI skeleton (init, add, list, show, work)
4. Prompt assembler
5. Agent invoker (Claude Code CLI)
6. Git commit integration

### Workflow

For each component:
1. Read relevant docs to understand requirements
2. Implement in Rust following `docs/architecture/` patterns
3. Write tests
4. Commit with descriptive message
5. Continue to next component

### Phase 0 Validation (REQUIRED before proceeding)

Phase 0 is complete when ALL of these work:

```bash
# 1. Initialize
chant init

# 2. Create a test spec
chant add "Test spec"

# 3. Execute the spec (THIS MUST ACTUALLY INVOKE THE AGENT)
chant work 001

# 4. Verify completion
chant show 001  # Should show status: completed, commit: <hash>
```

**CRITICAL**: Step 3 must actually invoke the Claude CLI and have it execute the spec.
If this doesn't work, fix it before proceeding. The entire project depends on this loop.

---

## STOP - READ THIS BEFORE PHASE 1

After Phase 0 passes validation, you are **FORBIDDEN** from:

- ❌ Writing code directly to implement features
- ❌ Editing source files without a corresponding spec
- ❌ Making commits that don't follow `chant(spec-id): description` format
- ❌ Implementing multiple features at once

From this point forward, **chant builds chant**.

---

## Phase 1+: Self-Building (Spec-Driven Implementation)

### The Workflow (MANDATORY)

For EVERY feature, no matter how small:

```bash
# 1. Create a spec
chant add "Description of the feature"

# 2. OPTIONAL: Edit the spec to add details
# (Open .chant/specs/<id>.md and add acceptance criteria)

# 3. Execute the spec - THE AGENT IMPLEMENTS IT
chant work <id>

# 4. Verify it worked
chant show <id>  # status: completed

# 5. Test the feature manually if needed

# 6. Continue to next feature
```

### Phase 1: Git+ (One Feature at a Time)

```bash
chant add "Add feature branch creation based on defaults.branch config"
chant work <id>
# Test: chant work <spec> creates a branch when defaults.branch is true

chant add "Add --pr flag to create pull requests via gh CLI"
chant work <id>
# Test: chant work --pr <spec> creates a PR

chant add "Add branch and pr settings to config.md"
chant work <id>
# Test: config defaults work
```

### Phase 2: MCP Server

```bash
chant add "Add MCP server with chant_spec_list tool"
chant work <id>

chant add "Add chant_spec_get tool to MCP server"
chant work <id>

chant add "Add chant_spec_update tool to MCP server"
chant work <id>

chant add "Add chant mcp command to start server"
chant work <id>
```

### Phase 3: Multi-Repo

```bash
chant add "Add global config support at ~/.config/chant/"
chant work <id>

chant add "Add repo: prefix parsing for cross-repo specs"
chant work <id>
```

### Phase 4: Structure

```bash
chant add "Add depends_on field with dependency checking"
chant work <id>

chant add "Add --label filter to list command"
chant work <id>

chant add "Add spec groups via .N filename suffix"
chant work <id>
```

### Phase 5: Observability

```bash
chant add "Add chant lint command for spec validation"
chant work <id>

chant add "Add chant status command for project overview"
chant work <id>

chant add "Add exit codes and error recovery hints"
chant work <id>
```

### Phase 6: Scale

```bash
chant add "Add PID-based locking to prevent concurrent work"
chant work <id>

chant add "Add chant lock list command"
chant work <id>
```

### Phase 7: Autonomy

```bash
chant add "Add chant verify command for drift detection"
chant work <id>
```

### Phase 8: Polish

```bash
chant add "Add chant ready shortcut command"
chant work <id>
```

---

## Success Criteria

The git history should show:

1. **Phase 0 commits**: Direct implementation commits (manual bootstrap)
2. **Phase 1+ commits**: All in format `chant(<spec-id>): <description>`

Example git log:
```
abc1234 chant(2024-01-15-003-x7m): Add --pr flag for PR creation
def5678 chant(2024-01-15-002-q2n): Add feature branch support
ghi9012 chant(2024-01-15-001-a3k): Test spec execution
jkl3456 Phase 0: Implement bootstrap core
mno7890 Initial commit: documentation
```

---

## Troubleshooting

### "chant work doesn't invoke the agent"

The `work` command shells out to `claude` CLI. Ensure:
1. Claude Code CLI is installed
2. It's in your PATH
3. It's authenticated

Test: `claude --version` and `claude --print "Hello"`

### "Agent doesn't commit properly"

Check the prompt in `.chant/prompts/standard.md` - it should instruct the agent to commit with the chant message format.

### "Spec stays in_progress"

The agent may have failed. Check:
1. The spec file for error messages
2. Git status for uncommitted changes
3. Run `chant show <id>` for details

---

## Remember

The value of this project is the **provenance** - proving that an AI built the software through tracked, reproducible specs. If you implement directly, that proof is lost.

Every. Feature. Through. Specs.
