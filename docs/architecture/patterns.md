# Architecture Patterns

This document maps chant's architecture to well-known software design patterns. Understanding these patterns helps contributors navigate the ~18K LOC codebase and see how different components fit together.

---

## 1. State Machine (TransitionBuilder)

**Pattern**: Builder + State Machine
**Location**: `src/spec/state_machine.rs`

The `TransitionBuilder` provides composable, validated state transitions for spec lifecycle management. It uses the Builder pattern to accumulate preconditions, then validates them before executing the state transition.

```
┌─────────────────────────────────────────────────┐
│  TransitionBuilder (Builder Pattern)           │
│                                                 │
│  .require_clean_tree()                          │
│  .require_dependencies_met()                    │
│  .require_all_criteria_checked()                │
│  .require_commits_exist()                       │
│  .to(SpecStatus::Completed)                    │
│                                                 │
│  ┌──────────────────────────────┐               │
│  │  Valid Transitions (FSM)     │               │
│  │                              │               │
│  │  Pending → InProgress        │               │
│  │  InProgress → Completed      │               │
│  │  InProgress → Failed         │               │
│  │  Failed → Pending            │               │
│  │  ...                         │               │
│  └──────────────────────────────┘               │
└─────────────────────────────────────────────────┘
```

**Key insight**: Preconditions are composable — you can require different combinations of checks depending on the transition context.

---

## 2. Execution Engine (Template Method + Strategy)

**Pattern**: Template Method + Strategy
**Location**: `src/cmd/work/executor.rs`, `src/cmd/work/single.rs`, `src/cmd/work/chain.rs`, `src/cmd/work/parallel.rs`

The execution engine uses Template Method to define the common validation/agent invocation/finalization flow, while Strategy pattern allows for three execution modes (single, chain, parallel).

```
┌──────────────────────────────────────────────────┐
│  Executor (Template Method)                      │
│                                                  │
│  1. validate_spec()                              │
│  2. invoke_agent()     ← Strategy implementation │
│  3. finalize_spec()                              │
│                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────┐ │
│  │   Single    │  │    Chain    │  │ Parallel │ │
│  │   (one)     │  │ (sequence)  │  │ (N-way)  │ │
│  └─────────────┘  └─────────────┘  └──────────┘ │
└──────────────────────────────────────────────────┘
```

**Key insight**: Common validation logic lives in `executor.rs`, while each mode implements its own scheduling strategy.

---

## 3. Worktree Isolation (Sandbox + Mutex)

**Pattern**: Sandbox + Mutex
**Location**: `src/worktree/mod.rs`, `src/worktree/git_ops.rs`

Git worktrees provide isolated execution environments for each spec. Mutex-protected creation prevents race conditions during parallel worktree creation.

```
┌──────────────────────────────────────────────────┐
│  Main Repository                                 │
│  (/path/to/project)                              │
│                                                  │
│  ┌──────────────────────────────────┐            │
│  │  Worktree Lock (Mutex)           │            │
│  │  Serializes create operations    │            │
│  └──────────────────────────────────┘            │
│                                                  │
│  ┌───────────────────┐  ┌───────────────────┐   │
│  │ Worktree A        │  │ Worktree B        │   │
│  │ /tmp/chant-001    │  │ /tmp/chant-002    │   │
│  │ branch: chant/001 │  │ branch: chant/002 │   │
│  │                   │  │                   │   │
│  │ .chant-status.json│  │ .chant-status.json│   │
│  └───────────────────┘  └───────────────────┘   │
└──────────────────────────────────────────────────┘
```

**Key insight**: Each agent works in an isolated filesystem and git branch, preventing conflicts between concurrent executions.

---

## 4. Dependency Graph (DAG + Topological Sort)

**Pattern**: Directed Acyclic Graph (DAG)
**Location**: `src/deps.rs`, `src/spec.rs`

Spec dependencies form a DAG with cycle detection and topological sort for determining execution order.

```
┌──────────────────────────────────────────────────┐
│  Dependency Resolution                           │
│                                                  │
│     spec-001                                     │
│       ↓                                          │
│     spec-002 ──→ spec-004                        │
│       ↓                                          │
│     spec-003                                     │
│                                                  │
│  Functions:                                      │
│  • resolve_dependency()                          │
│  • check_circular_dependencies()                 │
│  • is_blocked_by_dependencies()                  │
│  • is_ready()                                    │
└──────────────────────────────────────────────────┘
```

**Key insight**: Cross-repo dependencies supported via `repo:spec-id` syntax, resolved through config-defined repo paths.

---

## 5. Merge Driver (Rules Engine)

**Pattern**: Rules Engine (Declarative Conflict Resolution)
**Location**: `src/merge.rs`, `src/merge_driver.rs`

The merge driver uses a declarative rules table to map spec fields to merge strategies (ours, theirs, union, newest).

```
┌──────────────────────────────────────────────────┐
│  Merge Strategy Table                            │
│                                                  │
│  Field          Strategy      Reason             │
│  ─────────────  ────────────  ─────────────────  │
│  status         theirs        Agent's final      │
│  completed_at   theirs        Agent sets         │
│  model          theirs        Agent context      │
│  depends_on     union         Merge deps         │
│  target_files   union         Combine files      │
│  body           theirs        Agent output       │
│                                                  │
│  Strategies:                                     │
│  • ours: Keep base branch value                  │
│  • theirs: Take incoming branch value            │
│  • union: Merge both (deduplicate)               │
│  • newest: Pick most recent timestamp            │
└──────────────────────────────────────────────────┘
```

**Key insight**: Spec merges from work branches back to main are fully automated using field-specific merge strategies.

---

## 6. MCP Interface (Command Pattern)

**Pattern**: Command + Self-Describing Tools
**Location**: `src/mcp/server.rs`, `src/mcp/handlers.rs`, `src/mcp/tools/mod.rs`

The Model Context Protocol (MCP) interface exposes chant operations as self-describing tools with JSON Schema definitions.

```
┌──────────────────────────────────────────────────┐
│  MCP Server                                      │
│                                                  │
│  Tool Registry:                                  │
│  ┌────────────────────────────────────┐          │
│  │ chant_spec_list                    │          │
│  │ chant_spec_get                     │          │
│  │ chant_spec_update                  │          │
│  │ chant_work_start                   │          │
│  │ chant_finalize                     │          │
│  │ chant_watch_status                 │          │
│  │ ...                                │          │
│  └────────────────────────────────────┘          │
│                                                  │
│  Each tool:                                      │
│  • JSON Schema for parameters                    │
│  • Handler function in operations layer          │
│  • Structured JSON response                      │
└──────────────────────────────────────────────────┘
```

**Key insight**: Tools provide Claude-friendly interface to chant operations, enabling AI orchestration.

---

## 7. Provider Abstraction (Strategy Pattern)

**Pattern**: Strategy (Pluggable Backends)
**Location**: `src/provider.rs`

The `ModelProvider` trait defines a common interface for AI provider backends, enabling easy swapping between Claude, OpenAI, Ollama, etc.

```
┌──────────────────────────────────────────────────┐
│  ModelProvider Trait                             │
│                                                  │
│  trait ModelProvider {                           │
│    fn invoke_agent(...) -> Result<Output>        │
│  }                                               │
│                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌────────┐ │
│  │ ClaudeCliProvider                              │
│  │ └─→ claude code                  │            │
│  │                                  │            │
│  │ OpenaiProvider                   │            │
│  │ └─→ OpenAI API                   │            │
│  │                                  │            │
│  │ OllamaProvider                   │            │
│  │ └─→ Local Ollama                 │            │
│  └──────────────────────────────────┘            │
│                                                  │
│  Configured in: .chant/config.md                 │
│  providers.default_provider = "claude" | ...     │
└──────────────────────────────────────────────────┘
```

**Key insight**: Providers are configured declaratively and selected at runtime, no code changes needed.

---

## 8. Spec Groups (Composite Pattern)

**Pattern**: Composite (Driver/Member Hierarchy)
**Location**: `src/spec_group.rs`, `src/merge.rs`

Driver specs can have member specs in a hierarchical relationship. Members are identified by numeric suffixes (`.1`, `.2`, `.3`).

```
┌──────────────────────────────────────────────────┐
│  Spec Groups (Composite Pattern)                 │
│                                                  │
│  driver-spec                                     │
│    ├─→ driver-spec.1 (member)                    │
│    ├─→ driver-spec.2 (member)                    │
│    └─→ driver-spec.3 (member)                    │
│                                                  │
│  Functions:                                      │
│  • is_member_of()                                │
│  • get_members()                                 │
│  • all_members_completed()                       │
│  • auto_complete_driver_if_ready()               │
│                                                  │
│  Behavior:                                       │
│  • Driver marks in_progress when first member    │
│    starts                                        │
│  • Driver auto-completes when all members done   │
│  • Members can execute sequentially or in DAG    │
│    order via depends_on                          │
└──────────────────────────────────────────────────┘
```

**Key insight**: Drivers provide grouping/orchestration without executing themselves — they're auto-completed when members finish.

---

## 9. Watch Service (Observer Pattern)

**Pattern**: Observer (Polling-Based)
**Location**: `src/cmd/watch.rs`, `src/worktree/status.rs`

The watch service polls worktree status files and spec status, triggering lifecycle operations (finalize, merge, failure handling) when state changes are detected.

```
┌──────────────────────────────────────────────────┐
│  Watch Service (Observer via Polling)            │
│                                                  │
│  ┌─────────────────────────────────────┐         │
│  │  Poll Loop (configurable interval)  │         │
│  │                                     │         │
│  │  1. Load all specs                  │         │
│  │  2. Discover active worktrees       │         │
│  │  3. Read .chant-status.json files   │         │
│  │  4. Detect state changes            │         │
│  │  5. Trigger lifecycle handlers      │         │
│  │     • finalize + merge              │         │
│  │     • failure recovery              │         │
│  │  6. Sleep for poll_interval_ms      │         │
│  └─────────────────────────────────────┘         │
│                                                  │
│  Crash Recovery:                                 │
│  • Detects stale worktrees (>1hr working)        │
│  • Marks specs as failed                         │
│  • Cleans up orphaned worktrees                  │
└──────────────────────────────────────────────────┘
```

**Key insight**: Watch provides hands-off orchestration — start agents and let watch handle completion/merge/cleanup.

---

## 10. Operations Layer (Facade/Service Layer)

**Pattern**: Facade + Service Layer
**Location**: `src/operations/mod.rs`, `src/operations/create.rs`, `src/operations/finalize.rs`, etc.

The operations layer provides a stable API for spec operations, shared by both CLI commands and MCP handlers. This ensures consistency and simplifies testing.

```
┌──────────────────────────────────────────────────┐
│  Operations Layer (Facade)                       │
│                                                  │
│  ┌────────────────┐      ┌──────────────────┐   │
│  │  CLI Commands  │      │  MCP Handlers    │   │
│  └────────┬───────┘      └─────────┬────────┘   │
│           │                        │            │
│           └────────┬───────────────┘            │
│                    ↓                            │
│  ┌──────────────────────────────────────┐       │
│  │  Operations Layer                    │       │
│  │  • create_spec()                     │       │
│  │  • update_spec()                     │       │
│  │  • finalize_spec()                   │       │
│  │  • reset_spec()                      │       │
│  └──────────────────────────────────────┘       │
│                    ↓                            │
│  ┌──────────────────────────────────────┐       │
│  │  Domain Layer                        │       │
│  │  • Spec                              │       │
│  │  • SpecRepository                    │       │
│  │  • State Machine                     │       │
│  └──────────────────────────────────────┘       │
└──────────────────────────────────────────────────┘
```

**Key insight**: Operations layer provides canonical business logic, preventing drift between CLI and MCP implementations.

---

## 11. Config (Layered Configuration)

**Pattern**: Layered Configuration Merge
**Location**: `src/config/mod.rs`

Configuration is loaded from multiple sources with explicit merge order: global config, project config, and project-local agents config.

```
┌──────────────────────────────────────────────────┐
│  Configuration Merge (Layered Override)          │
│                                                  │
│  1. Global Config                                │
│     ~/.config/chant/config.md                    │
│     (user preferences, provider keys)            │
│                                                  │
│  2. Project Config  ← overrides                  │
│     .chant/config.md                             │
│     (git-tracked, team defaults)                 │
│                                                  │
│  3. Agents Config  ← overrides parallel.agents   │
│     .chant/agents.md                             │
│     (gitignored, local agent overrides)          │
│                                                  │
│  Result: Merged Config                           │
│  • Later layers override earlier                 │
│  • Allows user/project/local customization       │
└──────────────────────────────────────────────────┘
```

**Key insight**: Configuration merge allows personal defaults globally while projects define team standards and individuals override locally.

---

## 12. Output (Strategy + Adapter)

**Pattern**: Strategy + Adapter
**Location**: `src/ui.rs`, `src/formatters.rs`

The output system supports multiple output modes (human-readable colored output, JSON, quiet mode) via strategy pattern, with adapters for different contexts (CLI, MCP, test).

```
┌──────────────────────────────────────────────────┐
│  Output Strategies                               │
│                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────┐  │
│  │   Human     │  │    JSON     │  │  Quiet  │  │
│  │  (colored)  │  │ (machine)   │  │ (none)  │  │
│  └─────────────┘  └─────────────┘  └─────────┘  │
│                                                  │
│  Adapters:                                       │
│  • status_icon() - colored icons per status      │
│  • colors::success/warning/error                 │
│  • is_quiet() - check for silent mode            │
│  • Formatters for status displays                │
│                                                  │
│  Configuration:                                  │
│  • project.silent = true  ← suppresses output    │
│  • CHANT_QUIET env var                           │
└──────────────────────────────────────────────────┘
```

**Key insight**: Output strategies enable machine-readable JSON for CI/automation while preserving human-friendly terminal output.

---

## 13. Score/Lint (Composite Scorer)

**Pattern**: Composite (Aggregated Metrics)
**Location**: `src/scoring.rs`, `src/score/mod.rs`, `src/score/complexity.rs`, etc.

The quality scoring system composes multiple independent metrics (complexity, confidence, splittability, isolation, AC quality) into a unified score with traffic-light status.

```
┌──────────────────────────────────────────────────┐
│  Scoring System (Composite)                      │
│                                                  │
│  SpecScore {                                     │
│    complexity:      ComplexityGrade,             │
│    confidence:      ConfidenceGrade,             │
│    splittability:   SplittabilityGrade,          │
│    isolation:       Option<IsolationGrade>,      │
│    ac_quality:      ACQualityGrade,              │
│    traffic_light:   TrafficLight,                │
│  }                                               │
│                                                  │
│  Scorers (Independent):                          │
│  • Complexity (criteria count, file count, size) │
│  • Confidence (structure, vague language)        │
│  • Splittability (decomposability)               │
│  • Isolation (group member independence)         │
│  • AC Quality (acceptance criteria clarity)      │
│                                                  │
│  Traffic Light:                                  │
│  • Ready (green): Safe to execute                │
│  • Warning (yellow): Consider splitting          │
│  • Stop (red): Needs rework                      │
└──────────────────────────────────────────────────┘
```

**Key insight**: Each scorer is independent and tests are isolated. Traffic light provides quick go/no-go decision.

---

## Pattern Summary Table

| Pattern | Component | Location | Purpose |
|---------|-----------|----------|---------|
| **State Machine + Builder** | TransitionBuilder | `spec/state_machine.rs` | Composable preconditions for lifecycle transitions |
| **Template Method + Strategy** | Execution Engine | `cmd/work/executor.rs` | Common validation flow, pluggable execution modes |
| **Sandbox + Mutex** | Worktree Isolation | `worktree/mod.rs` | Isolated execution environments, serialized creation |
| **DAG** | Dependency Graph | `deps.rs` | Cross-repo dependencies, cycle detection, topological sort |
| **Rules Engine** | Merge Driver | `merge.rs` | Declarative conflict resolution by field |
| **Command** | MCP Interface | `mcp/server.rs` | Self-describing tools for AI orchestration |
| **Strategy** | Provider Abstraction | `provider.rs` | Pluggable AI backends |
| **Composite** | Spec Groups | `spec_group.rs` | Driver/member hierarchy with auto-completion |
| **Observer** | Watch Service | `cmd/watch.rs` | Polling-based lifecycle orchestration |
| **Facade/Service Layer** | Operations | `operations/mod.rs` | Shared business logic for CLI and MCP |
| **Layered Config** | Config Merge | `config/mod.rs` | Global → project → local override semantics |
| **Strategy + Adapter** | Output | `ui.rs`, `formatters.rs` | Human/JSON/Quiet output modes |
| **Composite** | Score/Lint | `scoring.rs`, `score/mod.rs` | Independent metric aggregation, traffic light |

---

## Navigation Tips

When exploring the codebase:

1. **Start with patterns, not files** — Identify which pattern you need to understand, then read the relevant module
2. **Follow the data** — Specs flow through: parse → validate → execute → finalize → merge
3. **Check tests** — Most patterns have comprehensive test coverage showing usage examples
4. **Read module docs** — Each module has header comments explaining its role and architectural decisions
5. **Use `docs/architecture/architecture.md`** — For high-level component overview and storage layout

---

## Additional Resources

- [Architecture Overview](architecture.md) - High-level system design
- [Agent Protocol](protocol.md) - How agents interact with chant
- [Concepts](../concepts/) - Core concepts like specs, dependencies, groups
- [CLI Reference](../reference/cli.md) - Command-line interface documentation
