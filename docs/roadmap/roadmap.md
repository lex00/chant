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
│  Layer 7: Autonomy                                              │
│  Drift detection, replay, verification, coaching                │
├─────────────────────────────────────────────────────────────────┤
│  Layer 6: Scale                                                 │
│  Daemon, queue, locks, Tantivy search                           │
├─────────────────────────────────────────────────────────────────┤
│  Layer 5: Observability                                         │
│  Logging, cost tracking, metrics, audit                         │
├─────────────────────────────────────────────────────────────────┤
│  Layer 4: Structure                                             │
│  Dependencies, groups, labels                                   │
├─────────────────────────────────────────────────────────────────┤
│  Layer 3: Multi-Repo                                            │
│  Global config, cross-repo deps, repo: prefix                   │
├─────────────────────────────────────────────────────────────────┤
│  Layer 2: MCP + Providers                                       │
│  MCP server, Kiro provider                                      │
├─────────────────────────────────────────────────────────────────┤
│  Layer 1: Git+                                                  │
│  Branches, PRs, isolation, hooks                                │
├─────────────────────────────────────────────────────────────────┤
│  Layer 0: CORE                                                  │
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

| Layer | Requires | Adds |
|-------|----------|------|
| 0: Core | Nothing | Basic spec execution |
| 1: Git+ | Core | Branches, PRs, isolation |
| 2: MCP | Core | MCP server, Kiro provider |
| 3: Multi-Repo | Core | Global config, cross-repo deps |
| 4: Structure | Core | Dependencies, hierarchy, labels |
| 5: Observability | Core | Logging, costs, metrics |
| 6: Scale | Core + Observability | Daemon, queue, Tantivy search |
| 7: Autonomy | Core + Structure | Drift, replay, coaching |

Layers 1-4 are independent of each other. You can have Multi-Repo without MCP, or Structure without Git+.

## Version Roadmap

| Version | Phase | Description |
|---------|-------|-------------|
| v0.1.0 | Phase 0 | Bootstrap Core - minimal working CLI |
| v0.2.0 | Phase 1 | Git+ - branches, PRs, isolation |
| v0.3.0 | Phase 2 | MCP + Providers |
| v0.4.0 | Phase 3 | Multi-Repo support |
| v0.5.0 | Phase 4 | Structure - deps, groups, labels |
| v0.6.0 | Phase 5 | Observability - logging, costs, metrics |
| v0.7.0 | Phase 6 | Scale - daemon, queue, search |
| v0.8.0 | Phase 7 | Autonomy - drift, replay, verification |
| v0.9.0 | Phase 8 | Polish - ecosystem, templates, approvals |

Phases 1-4 are independent and can be released in any order. Versions continue incrementing as features are refined.

## Implementation Phases

### Phase 0: Bootstrap Core (Manual) → v0.1.0

Build Core without chant (using AI agent directly). Tasks within each group run in parallel:

```
Group A (parallel):
  - Spec parser (read/write markdown + YAML)
  - State machine
  - CLI skeleton (init, add, list, show)

Group B (parallel, after A):
  - Prompt assembler
  - Agent invoker (default provider)
  - Git commit

Group C (after B):
  - chant work (execute spec) - ties it all together

Deliverable: Working chant that can execute specs
```

**After Phase 0, chant builds chant.**

### Phases 1-4: Independent Layers

These phases only depend on Core (not each other), but **each feature within a phase must be done sequentially via specs**.

```
Phase 1 → Phase 2 → Phase 3 → Phase 4
  │          │          │          │
  └──────────┴──────────┴──────────┘
     (order flexible, but ONE SPEC AT A TIME)
```

#### Phase 1: Git+ → v0.2.0

**Execute each spec one at a time, verify completion before next:**

```bash
chant add "Add --branch flag to create feature branches"
chant work <id>
# TEST: chant work --branch <spec> creates a branch

chant add "Add --pr flag to create pull requests via gh CLI"
chant work <id>
# TEST: chant work --pr <spec> creates a PR

chant add "Add branch and pr settings to config.md"
chant work <id>
# TEST: config defaults work
```

See [git.md](../reference/git.md) and [isolation.md](../scale/isolation.md) for design.

**After Phase 1**: Chant's own development switches to `branch: true` - all subsequent work uses task branches and PRs.

#### Phase 2: MCP + Providers → v0.3.0

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

See [mcp.md](../reference/mcp.md) and [protocol.md](../architecture/protocol.md) for design.

#### Phase 3: Multi-Repo → v0.4.0

```bash
chant add "Add global config support at ~/.config/chant/"
chant work <id>

chant add "Add repo: prefix parsing for cross-repo specs"
chant work <id>
```

See [multi-project.md](../scale/multi-project.md) for design.

#### Phase 4: Structure → v0.5.0

```bash
chant add "Add depends_on field with dependency checking"
chant work <id>

chant add "Add --label filter to list command"
chant work <id>

chant add "Add spec groups via .N filename suffix"
chant work <id>
```

See [deps.md](../concepts/deps.md), [groups.md](../concepts/groups.md), [triggers.md](../concepts/triggers.md), [spec-types.md](../concepts/spec-types.md).

### Phase 5: Observability → v0.6.0

Depends on: Core

```bash
chant add "Add chant lint command for spec validation"
chant work <id>

chant add "Add chant status command for project overview"
chant work <id>

chant add "Add exit codes and error recovery hints"
chant work <id>
```

See [observability.md](../scale/observability.md), [costs.md](../reference/costs.md), [schema.md](../reference/schema.md), [errors.md](../reference/errors.md), [reports.md](../reference/reports.md).

### Phase 6: Scale → v0.7.0

Depends on: Core + Observability

```bash
chant add "Add PID-based locking to prevent concurrent work"
chant work <id>

chant add "Add chant lock list command"
chant work <id>
```

See [scale.md](../scale/scale.md), [daemon.md](../scale/daemon.md), [locks.md](../scale/locks.md), [queue.md](../scale/queue.md).

### Phase 6.5: Semantic Search (Optional) → v0.7.x

Optional enhancement for research workflows. Adds vector-based similarity search.

```bash
chant add "Add fastembed-rs integration"
chant add "Add arroy vector store"
chant add "Add semantic search CLI (--semantic flag)"
chant add "Add hybrid search merging"
chant add "Add chant similar command"
chant work --parallel
```

**Tech stack:**
- `fastembed-rs` — Rust-native embeddings, runs locally
- `arroy` — Pure Rust ANN, LMDB storage (same patterns as Tantivy)
- `BGE-small-en` — Default model (384 dims, 50MB)

**Why optional:** Code specs work fine with keyword search. Research specs benefit from semantic similarity. Opt-in via config.

### Phase 7: Autonomy → v0.8.0

Depends on: Core + Structure (Phase 4)

```bash
chant add "Add chant verify command for drift detection"
chant work <id>
```

See [autonomy.md](../concepts/autonomy.md).

### Phase 8: Polish → v0.9.0

```bash
chant add "Add chant ready shortcut command"
chant work <id>
```

See [ecosystem.md](../guides/ecosystem.md), [approvals.md](../guides/approvals.md), [notifications.md](../reference/notifications.md), [templates.md](../reference/templates.md), [git-hooks.md](../reference/git-hooks.md).

Note: Claude provider is Phase 0. Phase 8 adds additional providers and ecosystem features.

## Phase Validation

Each phase must include integration tests that validate its features before moving to the next phase. See [Testing Strategy](../reference/testing.md) for comprehensive test specifications.

### Testing Requirements

| Phase | Validation |
|-------|------------|
| 0: Core | Spec create → work → complete cycle works end-to-end |
| 1: Git+ | Branch creation, PR creation, worktree isolation work |
| 2: MCP | MCP server responds, provider plugins load |
| 3: Multi-Repo | Cross-repo deps resolve, global list shows all repos |
| 4: Structure | Dependencies block correctly, groups split/combine |
| 5: Observability | Logs written, costs tracked, linter catches errors |
| 6: Scale | Locks prevent conflicts, daemon serves queries, queue orders correctly |
| 7: Autonomy | Drift detected on file change, replay restores state |
| 8: Polish | Notifications fire, templates expand, approvals block |

### Test Structure

```
tests/
├── integration/
│   ├── phase0_core_test.rs
│   ├── phase1_git_test.rs
│   ├── phase2_mcp_test.rs
│   ├── phase3_multirepo_test.rs
│   ├── phase4_structure_test.rs
│   ├── phase5_observability_test.rs
│   ├── phase6_scale_test.rs
│   ├── phase7_autonomy_test.rs
│   └── phase8_polish_test.rs
└── fixtures/
    └── sample_specs/
```

### Phase Gate

A phase is complete when:
1. All feature specs are `status: completed`
2. Integration tests pass
3. Documentation updated

No phase proceeds without passing its integration tests.

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

# Verify
chant verify 001

# Detect drift in chant itself
chant drift

# Replay if needed
chant replay 023
```

Chant maintains chant.

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
