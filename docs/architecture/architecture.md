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

These primitives compose. A spec with triggers waits for conditions,
executes via prompts, verifies over time, and replays when drift occurs.

The infrastructure is "living" because it's not static configuration—
it actively maintains intent over time.

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
│  │ *.md        │    │ *.md        │    │ defaults, providers,    │ │
│  │             │    │             │    │ hooks, notifications    │ │
│  │ Agent       │    │ What to     │    │                         │ │
│  │ behavior    │    │ build       │    │                         │ │
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
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    chant work <id>                            │  │
│  │                                                               │  │
│  │  1. Load spec      ─→  .chant/specs/001.md                   │  │
│  │  2. Load prompt    ─→  .chant/prompts/standard.md            │  │
│  │  3. Acquire lock   ─→  .chant/.locks/001.pid                 │  │
│  │  4. Create branch  ─→  git checkout -b chant/001             │  │
│  │  5. Invoke agent   ─→  configured provider                   │  │
│  │  6. Agent works    ─→  reads, edits, tests, commits          │  │
│  │  7. Update spec    ─→  status: completed, commit: abc123     │  │
│  │  8. Release lock   ─→  remove .locks/001.pid                 │  │
│  │  9. Notify         ─→  webhook/slack (optional)              │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Layer Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Interface Layer                              │
│                                                                     │
│   CLI ◄──────────────────────────────────────────────► Daemon API   │
│    │                                                        │       │
│    │   chant add/work/list/show                  REST/Socket│       │
│    │                                                        │       │
└────┴────────────────────────────────────────────────────────┴───────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Core Layer                                   │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐            │
│  │  Parser  │  │  Queue   │  │  Locks   │  │ Executor │            │
│  │          │  │          │  │          │  │          │            │
│  │ YAML +   │  │ Ready    │  │ PID      │  │ Agent    │            │
│  │ Markdown │  │ specs    │  │ files    │  │ invoke   │            │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Storage Layer                                │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    .chant/ directory                          │  │
│  │                                                               │  │
│  │  config.md          (project config)        git-tracked      │  │
│  │  prompts/*.md       (agent behavior)        git-tracked      │  │
│  │  specs/*.md         (work items)            git-tracked      │  │
│  │  .locks/*.pid       (who's working)         gitignored       │  │
│  │  .store/            (tantivy index)         gitignored       │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Provider Layer                               │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  AI providers (pluggable)                                    │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  SCM adapters (pluggable)                                    │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Deployment Modes

### Mode 1: Solo (No Daemon)

```
┌─────────────────────────────────────┐
│           Developer Machine         │
│                                     │
│   ┌─────────┐      ┌─────────┐     │
│   │  CLI    │ ───► │ .chant/ │     │
│   └─────────┘      └─────────┘     │
│        │                            │
│        ▼                            │
│   ┌─────────┐                       │
│   │  Agent  │                       │
│   └─────────┘                       │
│                                     │
└─────────────────────────────────────┘

No daemon. CLI reads files directly.
Simple. Works offline.
```

### Mode 2: Team (No Daemon)

```
┌──────────────┐         ┌──────────────┐
│    Alice     │         │     Bob      │
│              │         │              │
│  ┌───────┐   │         │   ┌───────┐  │
│  │  CLI  │   │         │   │  CLI  │  │
│  └───┬───┘   │         │   └───┬───┘  │
│      │       │         │       │      │
└──────┼───────┘         └───────┼──────┘
       │                         │
       └────────────┬────────────┘
                    │
                    ▼
            ┌──────────────┐
            │  Git Remote  │
            │              │
            │   .chant/    │
            │   specs/     │
            │   prompts/   │
            └──────────────┘

No daemon. Git coordinates.
PID locks prevent double-work.
```

### Mode 3: Team (With Daemon)

```
┌─────────────────────────────────────────┐
│              Shared Server              │
│                                         │
│   ┌──────────────────────────────────┐  │
│   │            Daemon                 │  │
│   │                                   │  │
│   │  ┌─────────┐  ┌─────────┐       │  │
│   │  │  Index  │  │  Queue  │       │  │
│   │  │(Tantivy)│  │         │       │  │
│   │  └─────────┘  └─────────┘       │  │
│   │                                   │  │
│   └───────────────┬───────────────────┘  │
│                   │                      │
│     ┌─────────────┼─────────────┐       │
│     │             │             │       │
│     ▼             ▼             ▼       │
│  ┌──────┐    ┌──────┐    ┌──────┐      │
│  │Worker│    │Worker│    │Worker│      │
│  │  1   │    │  2   │    │  3   │      │
│  └──────┘    └──────┘    └──────┘      │
│                                         │
└─────────────────────────────────────────┘

Daemon provides fast queries, coordination.
Workers poll queue, execute specs.
```

### Mode 4: Scale (K8s)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                        │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Daemon Pod                               │ │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐           │ │
│  │  │ Index  │  │ Queue  │  │  API   │  │Metrics │           │ │
│  │  └────────┘  └────────┘  └────────┘  └────────┘           │ │
│  └────────────────────────────┬───────────────────────────────┘ │
│                               │                                  │
│         ┌─────────────────────┼─────────────────────┐           │
│         │                     │                     │           │
│         ▼                     ▼                     ▼           │
│  ┌────────────┐       ┌────────────┐       ┌────────────┐      │
│  │ Worker Pod │       │ Worker Pod │       │ Worker Pod │      │
│  │            │       │            │       │            │      │
│  │  chant     │       │  chant     │       │  chant     │      │
│  │  agent     │       │  agent     │       │  agent     │      │
│  │  worker    │       │  worker    │       │  worker    │      │
│  └────────────┘       └────────────┘       └────────────┘      │
│                                                                  │
│  ┌────────────┐       ┌────────────┐                            │
│  │  Grafana   │◄──────│Prometheus  │                            │
│  └────────────┘       └────────────┘                            │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘

Daemon exposes metrics.
K8s scales workers.
Grafana for visibility.
```

## Data Flow: Spec Execution

```
                    chant work 001
                         │
                         ▼
              ┌──────────────────────┐
              │    Load Spec File    │
              │  .chant/specs/001.md │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │    Check Ready       │
              │  status=pending?     │
              │  deps met?           │
              │  not locked?         │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │    Acquire Lock      │
              │  .chant/.locks/      │
              │  001.pid             │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │    Load Prompt       │
              │  .chant/prompts/     │
              │  standard.md         │
              │  + extensions        │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │   Assemble Context   │
              │                      │
              │  prompt + spec +     │
              │  project context     │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │    Create Branch     │
              │  (if branch: true)   │
              │  chant/001           │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │    Invoke Agent      │
              │                      │
              │  Provider: Claude    │
              │  Model: opus         │
              │                      │
              │  Agent reads code,   │
              │  makes changes,      │
              │  runs tests,         │
              │  commits             │
              └──────────┬───────────┘
                         │
              ┌──────────┴───────────┐
              │                      │
         Success                  Failure
              │                      │
              ▼                      ▼
    ┌─────────────────┐    ┌─────────────────┐
    │  Update Spec    │    │  Update Spec    │
    │                 │    │                 │
    │ status:completed│    │ status: failed  │
    │ commit: abc123  │    │ error: ...      │
    │ branch: chant/  │    │                 │
    └────────┬────────┘    └────────┬────────┘
             │                      │
             └──────────┬───────────┘
                        │
                        ▼
              ┌──────────────────────┐
              │    Release Lock      │
              │    Notify            │
              │    Create PR (opt)   │
              └──────────────────────┘
```

## Prompt Assembly

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Final Agent Prompt                             │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │  Base Prompt (.chant/prompts/standard.md)                     │ │
│  │                                                                │ │
│  │  # Execute Spec                                                │ │
│  │  You are implementing a spec for {{project.name}}...          │ │
│  │                                                                │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                              │                                      │
│                              ▼                                      │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │  Extension (if extends: other-prompt)                         │ │
│  │                                                                │ │
│  │  {{> base-prompt}}                                            │ │
│  │  ## Additional team rules...                                  │ │
│  │                                                                │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                              │                                      │
│                              ▼                                      │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │  Spec Content (.chant/specs/001.md body)                      │ │
│  │                                                                │ │
│  │  # Add authentication                                         │ │
│  │  ## Acceptance Criteria                                       │ │
│  │  - [ ] JWT tokens work                                        │ │
│  │                                                                │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                              │                                      │
│                              ▼                                      │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │  Project Context (from config, auto-detected)                 │ │
│  │                                                                │ │
│  │  Project: my-app                                              │ │
│  │  Language: rust                                               │ │
│  │  Test command: cargo test                                     │ │
│  │                                                                │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Prompt extension: base is rendered first, then extensions add to it.
Variables like {{project.name}} are filled from config and spec.
```

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
| **SCM** | External system sync (GitHub, Jira, etc.) |

## File Ownership

```
.chant/
├── config.md             # Human + Chant
├── prompts/
│   ├── standard.md       # Human (or community)
│   └── custom.md         # Human
├── specs/
│   ├── 001.md            # Human creates, Chant updates
│   └── 002.md            # Chant creates (via split)
├── .locks/               # Chant only (gitignored)
│   └── 001.pid
└── .store/               # Chant only (gitignored)
    └── tantivy/
```

## What Runs Where

| Component | Solo | Team (no daemon) | Team (daemon) | K8s |
|-----------|------|------------------|---------------|-----|
| CLI | Local | Local | Local | Local/CI |
| Spec files | Local + git | Local + git | Local + git | Shared volume |
| Locks | Local files | Local files | Daemon | Daemon |
| Index | On-demand | On-demand | Daemon | Daemon |
| Queue | Files | Files | Daemon | Daemon + Redis/PG |
| Agent | Local | Local | Workers | Worker pods |
| Metrics | None | None | Daemon | Prometheus |
