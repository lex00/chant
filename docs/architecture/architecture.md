# Architecture Overview

## Living Intent Infrastructure

The composable primitives that make [self-driving specs](../concepts/autonomy.md) possible:

| Primitive | Purpose | Self-Driving Role |
|-----------|---------|-------------------|
| **Specs** | Executable specifications | The intent to execute |
| **Prompts** | Agent behavior definitions | How agents interpret intent |
| **Triggers** | Event-based activation | When specs activate |
| **Dependencies** | Execution ordering | Sequencing of intent |
| **Verification** | Ongoing truth-checking | Detecting drift |
| **Replay** | Re-execution mechanism | Restoring intent |

## The Big Picture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Human / CI                                  │
│                              │                                      │
│                         chant CLI                                   │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐ │
│  │   Prompts   │    │    Specs    │    │        Config           │ │
│  │             │    │             │    │                         │ │
│  │ .chant/     │    │ .chant/     │    │ .chant/config.md        │ │
│  │ prompts/    │    │ specs/      │    │                         │ │
│  │ *.md        │    │ *.md        │    │ defaults, providers     │ │
│  │             │    │             │    │                         │ │
│  └─────────────┘    └─────────────┘    └─────────────────────────┘ │
│                                                                     │
│                    MARKDOWN IS THE UI                               │
│                    (git-tracked, human-readable)                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Execution                                   │
│                                                                     │
│  1. Load spec      ─→  .chant/specs/001.md                         │
│  2. Load prompt    ─→  .chant/prompts/standard.md                  │
│  3. Acquire lock   ─→  .chant/.locks/001.pid                       │
│  4. Create branch  ─→  git checkout -b chant/001                   │
│  5. Invoke agent   ─→  configured provider                         │
│  6. Agent works    ─→  reads, edits, tests, commits                │
│  7. Update spec    ─→  status: completed, commit: abc123           │
│  8. Release lock   ─→  remove .locks/001.pid                       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Interface Layer                              │
│   CLI ◄──────────────────────────────────────────────► Daemon API   │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Core Layer                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐            │
│  │  Parser  │  │  Queue   │  │  Locks   │  │ Executor │            │
│  │ YAML +   │  │ Ready    │  │ PID      │  │ Agent    │            │
│  │ Markdown │  │ specs    │  │ files    │  │ invoke   │            │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘            │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Storage Layer                                │
│  .chant/                                                            │
│    config.md          (project config)        git-tracked          │
│    prompts/*.md       (agent behavior)        git-tracked          │
│    specs/*.md         (work items)            git-tracked          │
│    .locks/*.pid       (who's working)         gitignored           │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Provider Layer                               │
│  AI providers (pluggable) · SCM adapters (pluggable)               │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

**Implementation: Rust** - Single native binary. No runtime dependencies.

### Core Dependencies

| Category | Crate | Purpose |
|----------|-------|---------|
| Parsing | `pulldown-cmark` | Markdown parsing |
| | `serde`, `serde_yaml` | YAML frontmatter |
| CLI | `clap` | Argument parsing |
| Async | `rayon` | Parallel parsing |

### External Dependencies

| Dependency | Integration |
|------------|-------------|
| Git | Shell out (uses user's config/auth) |
| AI Agent | Shell out (provider CLI invocation) |

### Delivery

- `brew install chant`
- `cargo install chant`
- GitHub releases (cargo-dist)

---

## Agent Invocation

### The Spec IS the Prompt

Specs are granular, have acceptance criteria, and are linted. The agent prompt is simple:

```
Implement this spec.

{spec file contents}
```

The spec file contains everything the agent needs:
- Title (what to do)
- Description (context)
- Acceptance criteria (definition of done)
- Target files (where to look)

### Default Prompt

```markdown
# .chant/prompts/standard.md
---
name: standard
---

Implement this spec. Follow the acceptance criteria exactly.

When complete, commit with message: `chant({{spec.id}}): <description>`

---

{{spec}}
```

### Output Capture

Agent output goes to:
1. Terminal (streamed)
2. Spec file `progress` field (appended)

---

## Storage & Indexing

### Directory Structure

```
.chant/
├── config.md             # Project config (git-tracked)
├── prompts/              # Prompt files (git-tracked)
│   ├── standard.md
│   └── tdd.md
├── specs/                # All specs (git-tracked)
│   ├── 2026-01-22-001-x7m.md
│   └── 2026-01-22-002-q2n.md
├── .locks/               # PID files (gitignored)
└── .store/               # Index cache (gitignored)
```

### No Archive Folder

Specs stay in `specs/` forever. Completed specs have `status: completed`.

Why:
- Git history preserves everything
- Moving files changes IDs (breaks references)
- Simpler mental model

Filter by status instead:
```bash
chant list                  # Active (pending, in_progress, failed)
chant list --all            # Everything
chant list --completed      # Just completed
```

### Active Specs: In-Memory

For <50 active specs, parse files in parallel on each CLI invocation (~50-100ms).

---

## Deployment Modes

### Mode 1: Solo (No Daemon)

```
Developer Machine
  CLI → .chant/ → Agent
```

No daemon. CLI reads files directly. Simple. Works offline.

### Mode 2: Team (No Daemon)

```
Alice           Bob
  ↓               ↓
  └─────┬─────────┘
        ↓
    Git Remote (.chant/)
```

No daemon. Git coordinates. PID locks prevent double-work.

### Mode 3: Team (With Daemon)

```
Shared Server
  Daemon (Index + Queue)
     ↓
  Worker 1, Worker 2, Worker 3
```

Daemon provides fast queries, coordination. Workers poll queue, execute specs.

### Mode 4: Scale (K8s)

```
Kubernetes Cluster
  Daemon Pod (Index, Queue, API, Metrics)
     ↓
  Worker Pods (auto-scaled)
     ↓
  Prometheus → Grafana
```

---

## Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| **CLI** | User interface, command dispatch |
| **Parser** | Read/write markdown + YAML frontmatter |
| **Queue** | Track ready specs, priority ordering |
| **Locks** | Prevent double-work, crash recovery |
| **Executor** | Invoke agent, manage lifecycle |
| **Daemon** | Persistent services (optional) |
| **Index** | Fast search via Tantivy |
| **Providers** | Agent adapters (pluggable) |

## File Ownership

```
.chant/
├── config.md             # Human + Chant
├── prompts/*.md          # Human (or community)
├── specs/*.md            # Human creates, Chant updates
├── .locks/               # Chant only (gitignored)
└── .store/               # Chant only (gitignored)
```

---

For detailed agent protocol specification, see [protocol.md](protocol.md).
