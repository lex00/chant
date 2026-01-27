# Roadmap

## Core Principles

- Markdown IS the UI
- Everything is markdown (tasks, prompts, config, templates)
- CLI-native, git-native
- Agents do the work, humans review

## What's Designed

| Feature | Document |
|---------|----------|
| Spec model | [specs.md](../concepts/specs.md) |
| Spec types | [spec-types.md](../concepts/spec-types.md) |
| ID format | [ids.md](../concepts/ids.md) |
| Spec groups | [groups.md](../concepts/groups.md) |
| Dependencies | [deps.md](../concepts/deps.md) |
| Triggers | [triggers.md](../concepts/triggers.md) |
| Autonomy | [autonomy.md](../concepts/autonomy.md) |
| Prompts | [prompts.md](../concepts/prompts.md) |
| Hooks | [hooks.md](../concepts/hooks.md) |
| Templates | [templates.md](../reference/templates.md) |
| Git integration | [git.md](../reference/git.md) |
| Git hooks | [git-hooks.md](../reference/git-hooks.md) |
| Worktree isolation | [isolation.md](../scale/isolation.md) |
| Locking | [locks.md](../scale/locks.md) |
| Search | [search.md](../reference/search.md) |
| Daemon mode | [daemon.md](../scale/daemon.md) |
| Metrics | [metrics.md](../scale/metrics.md) |
| Observability | [observability.md](../scale/observability.md) |
| Notifications | [notifications.md](../reference/notifications.md) |
| Agent protocol | [protocol.md](../architecture/protocol.md) |
| MCP server | [mcp.md](../reference/mcp.md) |
| Security | [security.md](../enterprise/security.md) |
| Cost tracking | [costs.md](../reference/costs.md) |
| Enterprise | [enterprise.md](../enterprise/enterprise.md) |
| Scale patterns | [scale.md](../scale/scale.md) |
| Queue architecture | [queue.md](../scale/queue.md) |
| Multi-project | [multi-project.md](../scale/multi-project.md) |
| Architecture | [architecture.md](../architecture/architecture.md) |
| Ecosystem | [ecosystem.md](../guides/ecosystem.md) |
| Approvals | [approvals.md](../guides/approvals.md) |
| Recovery | [recovery.md](../guides/recovery.md) |
| Data lifecycle | [lifecycle.md](../concepts/lifecycle.md) |
| Errors | [errors.md](../reference/errors.md) |
| Schema | [schema.md](../reference/schema.md) |
| Reports | [reports.md](../reference/reports.md) |
| Configuration | [config.md](../reference/config.md) |
| CLI | [cli.md](../reference/cli.md) |

## Architecture Layers

The system has a minimal core with optional layers built on top:

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 7: Autonomy                              ⏳ PARTIAL      │
│  Drift ✓ | Replay ❌, verify ❌                                 │
├─────────────────────────────────────────────────────────────────┤
│  Layer 6: Scale                                 ⏳ PARTIAL      │
│  Locks ✓, parallel ✓, rotation ✓ | Daemon ❌, Tantivy ❌       │
├─────────────────────────────────────────────────────────────────┤
│  Layer 5: Observability                         ✅ COMPLETE     │
│  lint, status, log, diagnose commands                           │
├─────────────────────────────────────────────────────────────────┤
│  Layer 4: Structure                             ✅ COMPLETE     │
│  Dependencies, groups, labels, split                            │
├─────────────────────────────────────────────────────────────────┤
│  Layer 3: Multi-Repo                            ⏳ PARTIAL      │
│  Global config ✓ | Cross-repo deps ❌                           │
├─────────────────────────────────────────────────────────────────┤
│  Layer 2: MCP + Providers                       ✅ COMPLETE     │
│  MCP server, Claude/Ollama/OpenAI providers                     │
├─────────────────────────────────────────────────────────────────┤
│  Layer 1: Git+                                  ✅ COMPLETE     │
│  Branches, PRs, worktree isolation, merge                       │
├─────────────────────────────────────────────────────────────────┤
│  Layer 0: CORE                                  ✅ COMPLETE     │
│  Specs, states, prompts, work, commit                           │
└─────────────────────────────────────────────────────────────────┘
```

### Layer 0: Core (Required)

The minimum to be useful:

| Component | Description | ~Lines |
|-----------|-------------|--------|
| Spec parser | Markdown + YAML frontmatter | 100 |
| Spec ID parser | Handles optional `repo:` prefix | 50 |
| State machine | pending → in_progress → completed/failed | 50 |
| Prompt assembler | Load prompt, inject spec | 100 |
| Agent invoker | Call Claude (or any LLM) | 150 |
| Git commit | Commit with message format | 50 |
| CLI | init, add, list, show, work | 200 |

**Total: ~700 lines of meaningful code**

The prompt does the heavy lifting. Core just orchestrates.

**Multi-repo aware from day one**: Spec ID parser handles `repo:` prefix even in Phase 0, avoiding future refactoring.

### Layer Dependencies

| Layer | Status | Adds |
|-------|--------|------|
| 0: Core | ✅ | Spec CRUD, state machine, prompts, agent execution |
| 1: Git+ | ✅ | `--branch`, `--pr`, worktree isolation, merge |
| 2: MCP + Providers | ✅ | MCP server, Claude/Ollama/OpenAI |
| 3: Multi-Repo | ⏳ | Global config (cross-repo deps pending) |
| 4: Structure | ✅ | `depends_on`, groups, labels, split |
| 5: Observability | ✅ | lint, status, log, diagnose |
| 6: Scale | ⏳ | Locks, parallel, agent rotation (daemon/Tantivy pending) |
| 7: Autonomy | ⏳ | Drift detection (replay/verify pending) |

## Version Roadmap

### Current Status: v0.1.13

Development moved faster than the original phased plan. Most features from Phases 0-8 are implemented.

| Phase | Status | Features |
|-------|--------|----------|
| Phase 0: Core | ✅ Complete | Spec CRUD, state machine, prompts, agent invocation, git commits |
| Phase 1: Git+ | ✅ Complete | `--branch`, `--pr`, worktree isolation, merge command |
| Phase 2: MCP + Providers | ✅ Complete | MCP server, Claude/Ollama/OpenAI providers |
| Phase 3: Multi-Repo | ⏳ Partial | Global config exists; cross-repo deps not yet |
| Phase 4: Structure | ✅ Complete | `depends_on`, groups (`.N` suffix), labels, `split` command |
| Phase 5: Observability | ✅ Complete | `lint`, `status`, `log`, `diagnose` commands |
| Phase 6: Scale | ⏳ Partial | Locks, `--parallel`, agent rotation; NOT: daemon, Tantivy |
| Phase 7: Autonomy | ⏳ Partial | `drift` command; NOT: replay, verify |
| Phase 8: Polish | ✅ Complete | Interactive wizards, export, cancel, config validation |

### Upcoming Releases

| Version | Focus |
|---------|-------|
| v0.2.0 | Complete autonomy (`verify`, `replay` commands) |
| v0.3.0 | Full-text search (Tantivy indexing) |
| v0.4.0 | Full multi-repo support (cross-repo deps) |
| v0.5.0 | Daemon mode for background execution |
| v1.0.0 | Stable API, complete documentation |

### Spec Types (Implemented)

Specialized spec types enable different workflows:

| Type | Purpose | Key Fields |
|------|---------|------------|
| `code` | Implementation, bug fixes, refactoring | `target_files:` |
| `task` | Manual work, research, planning | - |
| `driver`/`group` | Orchestrate multiple specs | `members:` (auto-populated) |
| `documentation` | Keep docs in sync with code | `tracks:` (drift trigger) |
| `research` | Analysis, synthesis, investigation | `informed_by:`, `origin:`, `schedule:` |

See [spec-types.md](../concepts/spec-types.md) for details.

## Implementation Phases

### Phase 0: Bootstrap Core ✅ COMPLETE

Built Core manually (using AI agent directly):

- ✅ Spec parser (read/write markdown + YAML frontmatter)
- ✅ State machine (pending → in_progress → completed/failed)
- ✅ CLI (init, add, list, show, work)
- ✅ Prompt assembler
- ✅ Agent invoker (Claude provider)
- ✅ Git commit with `chant(spec-id):` format

**After Phase 0, chant builds chant.**

### Phase 1: Git+ ✅ COMPLETE

- ✅ `--branch` flag creates feature branches
- ✅ `--pr` flag creates pull requests via `gh` CLI
- ✅ Worktree isolation for parallel execution
- ✅ `chant merge` command
- ✅ Config defaults for branch/pr

See [git.md](../reference/git.md) and [isolation.md](../scale/isolation.md).

### Phase 2: MCP + Providers ✅ COMPLETE

- ✅ MCP server with spec tools (`chant mcp`)
- ✅ Claude provider (default)
- ✅ Ollama provider (local models)
- ✅ OpenAI provider
- ✅ Provider selection via config or `--provider` flag

See [mcp.md](../reference/mcp.md) and [protocol.md](../architecture/protocol.md).

### Phase 3: Multi-Repo ⏳ PARTIAL

- ✅ Global config at `~/.config/chant/`
- ❌ `repo:` prefix parsing for cross-repo specs
- ❌ Cross-repo dependencies

See [multi-project.md](../scale/multi-project.md).

### Phase 4: Structure ✅ COMPLETE

- ✅ `depends_on` field with dependency checking
- ✅ `--label` filter on list command
- ✅ Spec groups via `.N` filename suffix
- ✅ `chant split` command for decomposing specs
- ✅ Driver/group spec types with auto-completion

See [deps.md](../concepts/deps.md), [groups.md](../concepts/groups.md), [spec-types.md](../concepts/spec-types.md).

### Phase 5: Observability ✅ COMPLETE

- ✅ `chant lint` command for spec validation
- ✅ `chant status` command for project overview
- ✅ `chant log` command for execution logs
- ✅ `chant diagnose` command for troubleshooting
- ✅ Exit codes and error messages

See [observability.md](../scale/observability.md), [errors.md](../reference/errors.md).

### Phase 6: Scale ⏳ PARTIAL

**Implemented:**
- ✅ PID-based locking to prevent concurrent work
- ✅ `--parallel` flag for concurrent spec execution
- ✅ `chant archive` command
- ✅ Agent rotation strategies (`none`, `random`, `round-robin`)
- ✅ Multi-agent configuration with weighted selection

**Not yet implemented:**
- ❌ Daemon mode (background service)
- ❌ Tantivy full-text search indexing
- ❌ Queue architecture

See [scale.md](../scale/scale.md), [locks.md](../scale/locks.md).

### Phase 7: Autonomy ⏳ PARTIAL

**Implemented:**
- ✅ `chant drift` command for detecting spec staleness

**Not yet implemented:**
- ❌ `chant verify` command for specification verification
- ❌ `chant replay` command for re-executing specs

See [autonomy.md](../concepts/autonomy.md).

### Phase 8: Polish ✅ COMPLETE

**Implemented:**
- ✅ `chant ready` shortcut command
- ✅ `chant delete` command with `--cascade` flag
- ✅ `chant cancel` command (soft-delete with status change)
- ✅ `chant export` command (JSON, CSV, Markdown formats)
- ✅ `chant config --validate` command
- ✅ `chant search` interactive wizard
- ✅ Interactive wizards for `work`, `export`, `merge` commands
- ✅ `Blocked` status (auto-applied for unmet dependencies)
- ✅ `Cancelled` status for soft-deleted specs
- ✅ List filtering by `--status` (including blocked, cancelled)

**Not yet implemented:**
- ❌ Notifications (webhooks, email, Slack)
- ❌ Approvals workflow
- ❌ Template/prompt registry

See [ecosystem.md](../guides/ecosystem.md), [approvals.md](../guides/approvals.md).

## Testing

See [Testing Strategy](../reference/testing.md) for test specifications.

Current test coverage:
- 275+ tests (unit, integration, end-to-end)
- Unit tests in `src/`
- Integration tests in `tests/`

```bash
just test      # Run all tests
just check     # Run fmt, clippy, and tests
```

## Self-Bootstrap

### CRITICAL: The Bootstrap Boundary

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   Phase 0: Built manually (AI agent, no chant)             │
│   ─────────────────────────────────────────────────────    │
│   ~700 lines                                                │
│   Multi-repo aware from day one (repo: prefix parsing)     │
│                                                             │
├─ ═══════════════════════════════════════════════════════ ──┤
│   ⛔ HARD BOUNDARY - DO NOT CROSS WITHOUT VALIDATION ⛔     │
├─ ═══════════════════════════════════════════════════════ ──┤
│                                                             │
│   Phases 1-8: Built EXCLUSIVELY with chant                 │
│   ─────────────────────────────────────────────────────    │
│   Every. Single. Feature. Through. Specs.                   │
│   No exceptions. No shortcuts. No direct implementation.   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Mandatory Workflow (Phase 1+)

After Phase 0 validation passes, ALL development follows this pattern:

```bash
# 1. Create a spec (this is mandatory, not optional)
chant add "Description of feature"

# 2. Execute the spec (THE AGENT implements it)
chant work <id>

# 3. Verify completion
chant show <id>  # Must show status: completed

# 4. Test manually if needed

# 5. Move to next feature (ONE AT A TIME)
```

**FORBIDDEN after Phase 0:**
- ❌ Writing code directly to implement features
- ❌ Editing source files without a corresponding spec
- ❌ Making commits that don't follow `chant(<spec-id>): description`
- ❌ Implementing multiple features at once
- ❌ Skipping the `chant work` step

**The entire value of this project is the provenance** - proving that AI built the software through tracked, reproducible specs.

### Why This Works

1. **Core is tiny** - ~700 lines of focused work
2. **Prompts do the work** - Core just orchestrates
3. **Immediate dogfooding** - Phase 1+ uses the tool
4. **Each phase is specs** - Natural fit for the model
5. **Parallel development** - Layers are independent
6. **Future-proof** - Multi-repo parsing built in from start

### Bootstrap Validation

Phase 0 is complete when:

```bash
$ chant add "Test spec"
Created: 2026-01-22-001-x7m

$ chant work 001
[Agent executes spec]
Committed: abc123

$ chant show 001
status: completed
commit: abc123
```

That's it. Everything else is enhancement.

### Multi-Repo Support

Chant supports managing specs across multiple repositories:

```
~/projects/
├── backend/           # API services
└── frontend/          # Web application
```

Configure as multi-repo:

```yaml
# ~/.config/chant/config.yaml
repos:
  - path: ~/projects/backend
    name: backend
  - path: ~/projects/frontend
    name: frontend
```

Benefits:
- **Cross-repo dependencies** - specs can depend on work in other repos
- **Global visibility** - see all specs across projects
- **Unified workflow** - one CLI for all repositories

### Self-Improvement Loop

Once bootstrapped, chant improves itself:

```bash
# Find issues with chant
chant add "Fix: work command hangs on large files"

# Implement fix
chant work 001

# Check for completed
chant show 001  # status: completed

# Detect drift in documentation specs
chant drift

# Resume failed specs
chant resume 023 --work
```

Chant maintains chant.

**Future additions** (v0.2.0):
- `chant verify` - Verify spec acceptance criteria still pass
- `chant replay` - Re-execute a spec from scratch

## Non-Goals

Will not build:
- Web UI / dashboard (use Grafana)
- Team collaboration features (use GitHub)
- Time tracking
- Sprint planning
- External issue tracker integration (maybe plugins later)

Chant is a developer tool, not a project management platform.

## Competitive Position

Chant differentiates through:

- **Markdown IS the UI** - Human-readable, any editor
- **CLI-native** - Developer-focused workflow
- **Agent execution** - Specs execute themselves

## Future Considerations

### Prompt Ecosystem

Open source prompt sharing:
- Community registry (GitHub-based)
- Domain-specific collections (security, TDD, docs)
- Framework-specific prompts (React, Rails, etc.)
- Model-specific optimizations

See [ecosystem.md](../guides/ecosystem.md) for design.

Commercial layer (potential):
- Curated enterprise prompts
- Private team registries
- Usage analytics


### Wetwire Integration

Type-safe config generation from Go structs (internal tool: `wetwire`).

**What wetwire generates:**
- GitHub Actions / GitLab CI
- Prometheus / Alertmanager rules
- Grafana dashboards
- Kubernetes manifests
- Cloud provider configs (AWS, GCP, Azure)

**Chant use cases:**
- `chant init --github` → generates `.github/workflows/chant.yml`
- `chant init --gitlab` → generates `.gitlab-ci.yml`
- `chant init --k8s` → generates daemon deployment manifests
- `chant init --prometheus` → generates alerting rules
- `chant init --grafana` → generates dashboard JSON

**Advantage:**
- Type-safe: compile-time errors, not runtime YAML failures
- Consistent: same source generates all platforms
- Maintainable: update Go struct, regenerate all configs

Deferred until core CLI stable. Natural Phase 4+ integration.

## Success Metrics

| Metric | Target |
|--------|--------|
| Dogfooding | 100% (use chant to build chant) |
| Test coverage | 80% core, 90% parser |
| CLI response | <100ms (no daemon) |
| Search response | <10ms (with daemon) |
| Documentation | Complete before 1.0 |

## Lessons Learned (Bootstrap Attempt #1)

This section captures lessons from a failed bootstrap attempt where the agent implemented features directly instead of using chant specs. These lessons inform the stricter process documented above.

### What Went Wrong

1. **Treated spec commands as suggestions, not requirements**
   - The roadmap showed `chant add "..."` commands but didn't emphasize they were mandatory
   - Agent implemented features directly, bypassing the spec system entirely

2. **No Phase 0 validation gate**
   - Agent moved to Phase 1+ without verifying `chant work` actually invoked the Claude CLI
   - The core loop was never validated end-to-end before building on top of it

3. **"Parallel phases" encouraged shortcuts**
   - "Phases 1-4 can run in parallel" was interpreted as "implement everything at once"
   - Agent jumped around implementing features directly rather than spec-by-spec

4. **Missing explicit STOP checkpoint**
   - No hard boundary between Phase 0 (manual) and Phase 1+ (spec-driven)
   - The transition point was too subtle

5. **Roadmap focused on WHAT, not HOW**
   - Listed features to build but assumed reader knew to use specs
   - AI agents need explicit instructions about the process, not just the goals

### Fixes Applied

1. **BOOTSTRAP_PROMPT.md** - Explicit prompt for AI agents with forbidden actions clearly listed
2. **Hard boundary** - Visual "DO NOT CROSS" marker between Phase 0 and Phase 1+
3. **Phase 0 validation checklist** - Specific commands that must work before proceeding
4. **Mandatory workflow section** - Step-by-step process that must be followed
5. **One feature at a time** - Explicitly forbid implementing multiple features at once

### Validation Questions

Before any implementation after Phase 0, ask:
- Did I run `chant add` to create a spec?
- Is there a spec file in `.chant/specs/` for this feature?
- Will `chant work <id>` be used to implement it?
- Will the commit message follow `chant(<spec-id>): description` format?

If any answer is "no", STOP and correct the approach.
