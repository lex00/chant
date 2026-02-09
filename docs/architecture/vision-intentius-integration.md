# Vision: Chant as an Intentius Domain

## Context

Chant is an AI development orchestrator. Developers write markdown specs, agents execute them, chant manages the lifecycle — dependencies, worktrees, verification, finalization.

Intentius is a declarative IaC framework. Developers write TypeScript declarations (`new Bucket({...})`), intentius builds, lints, and synthesizes them to cloud templates.

Today these are separate tools in separate languages (Rust and TypeScript). This document describes how they converge.

## The Insight

Chant specs and intentius declarations are the same pattern:

```
Declarative description  →  validated  →  synthesized  →  executed by engine
```

For AWS: `new Bucket({...})` → lint → CloudFormation JSON → AWS API
For chant: markdown spec → lint → typed spec object → agent execution

Chant is an intentius domain. Its "cloud provider" is the agent execution engine. Its "infrastructure" is development work.

## Architecture

```
                    ┌─────────────────────────────────────────────┐
                    │              intentius                       │
                    │                                             │
                    │  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
                    │  │  @core   │  │  @aws     │  │  @chant   │  │
                    │  │          │  │          │  │          │  │
                    │  │Declarable│  │ Bucket   │  │ Spec     │  │
                    │  │ Domain   │  │ Function │  │ Group    │  │
                    │  │ Lint     │  │ Role     │  │ Workflow │  │
                    │  │ Build    │  │ ...      │  │ ...      │  │
                    │  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
                    │       │             │             │         │
                    │       └──────┬──────┘       ┌────┘         │
                    │              │              │              │
                    │         ┌────▼──────────────▼────┐         │
                    │         │     intentius build     │         │
                    │         │     intentius lint      │         │
                    │         └────┬──────────────┬────┘         │
                    │              │              │              │
                    └──────────────┼──────────────┼──────────────┘
                                   │              │
                    ┌──────────────▼──┐  ┌────────▼──────────────┐
                    │  CloudFormation  │  │    Chant Execution    │
                    │  JSON template   │  │    Engine             │
                    │                  │  │                       │
                    │  Consumed by     │  │  Spawns agents        │
                    │  AWS API         │  │  Manages worktrees    │
                    │                  │  │  Verifies criteria    │
                    └──────────────────┘  └───────────────────────┘
```

## The User Experience

### Nothing changes for most users

Most people interact with chant exactly as they do today:

```markdown
---
status: pending
depends_on: [db-setup]
labels: [auth]
---

# Implement OAuth login

Build the OAuth flow using Google OAuth 2.0.

## Acceptance Criteria

- [ ] OAuth redirect works
- [ ] Token stored in DynamoDB
- [ ] Logout clears session
```

They write markdown. Agents write markdown. Chant parses it.

The difference is under the hood: chant parses that markdown into typed `@intentius/chant` objects. The frontmatter becomes typed fields on a `Spec` declarable. The acceptance criteria become a typed array. The dependency references become real references, not string IDs.

### What chant does with the parsed types

```
Markdown spec file
        │
        ▼
┌──────────────────┐
│  Chant parser     │
│                   │
│  YAML frontmatter │──► Spec.status, Spec.dependsOn, Spec.labels, ...
│  Heading          │──► Spec.title
│  Body             │──► Spec.description
│  Checkboxes       │──► Spec.criteria[]
│                   │
└────────┬──────────┘
         │
         ▼
  Spec (TypeScript object)
  implements Declarable
         │
         ├──► intentius lint  (validate against WCH rules)
         ├──► intentius build (resolve cross-refs, dependency DAG)
         └──► chant execute   (spawn agent, manage lifecycle)
```

### Power user path: TypeScript specs

For programmatic spec generation, teams can write TypeScript directly:

```typescript
import * as chant from "@intentius/chant";
import * as _ from "./_";

export const dbSetup = new chant.Spec({
  title: "Set up auth database schema",
  criteria: [
    "Migrations run successfully",
    "Seed data loads",
  ],
});

export const authFeature = new chant.Spec({
  title: "Implement OAuth login flow",
  dependsOn: [$.dbSetup],                // typed reference
  target: [$.authBucket, $.handler],      // cross-domain ref to AWS
  criteria: [
    "OAuth redirect works",
    "Token stored in DynamoDB",
  ],
});
```

This is the same thing as the markdown, but with type checking, IDE autocomplete, and composability. Both paths produce the same `Spec` objects in memory.

The TypeScript path is not the default. It exists for:
- Generating specs programmatically (e.g., "create a spec for every AWS service in this stack")
- Complex dependency graphs that benefit from type checking
- Specs that reference infrastructure resources across domains

## Chant Rewrites in TypeScript

Chant moves from Rust to TypeScript (Bun). Reasons:

1. **Same language as intentius** — one codebase, one type system, one contributor pool
2. **Direct access to `@intentius/core`** — no subprocess boundary, import the types
3. **Everything is subsecond** — speed difference is immaterial
4. **`bun build --compile`** — single binary distribution without Node.js dependency
5. **The integration is deep** — chant uses intentius declarables, lint rules, discovery, the barrel pattern. Crossing a language boundary for every interaction is friction without benefit.

Chant's Rust-specific dependencies are all replaceable:

| Rust (current) | TypeScript (replacement) |
|----------------|--------------------------|
| clap | commander / oclif |
| serde_yaml | js-yaml |
| pulldown-cmark | remark / markdown-it |
| tera | handlebars / built-in template literals |
| ureq | fetch (built into Bun) |
| nix (signals) | process.on('SIGINT') |
| colored | chalk / kleur |

No exotic dependencies. No tokio, no tantivy, no tree-sitter.

## Lint Rules for Specs

`@intentius/chant` ships lint rules under the `WCH` prefix. They validate specs the same way `WAW` rules validate AWS infrastructure:

| Rule | What it catches |
|------|----------------|
| WCH001 | Spec without acceptance criteria |
| WCH002 | Circular dependencies in `depends_on` |
| WCH003 | Dependency references nonexistent spec |
| WCH004 | Missing required frontmatter fields |
| WCH005 | Spec body too large (should be split) |
| WCH006 | Acceptance criterion not actionable (no verb) |
| WCH007 | Duplicate spec titles |
| WCH008 | Spec depends on failed/cancelled spec |

These run as part of `intentius lint`, alongside core and AWS rules. Same engine, same config, same output format.

Chant also does runtime validation (is this spec ready to execute? are worktrees clean?). The split:
- **Intentius lint** — spec quality and style (runs before work starts)
- **Chant runtime** — execution readiness (runs at work time)

## Cross-Domain References

The intentius type system enables specs that reference infrastructure:

```typescript
export const deployStorage = new chant.Spec({
  title: "Deploy storage infrastructure",
  target: [$.dataBucket, $.backupBucket],   // AWS Declarables
  criteria: [
    "CloudFormation stack deploys",
    "Buckets accessible from Lambda",
  ],
});
```

The `target` field uses intentius cross-domain references (`DomainOutput`). When chant executes this spec, it knows which infrastructure resources the spec affects. This enables:
- **Impact analysis** — "which specs touch this bucket?"
- **Ordered deployment** — infra specs before application specs
- **Drift detection** — spec says it deployed X, but X changed since

## The Domain Implementation

```typescript
// @intentius/chant/src/domain.ts
import { Domain, Declarable, DomainOutput } from "@intentius/core";

export const chantDomain: Domain = {
  name: "chant",
  rulePrefix: "WCH",

  serialize(entities: Map<string, Declarable>): string {
    // Serialize Spec declarables to chant's markdown format
    // This is the "CloudFormation template" equivalent
    return specs.map(specToMarkdown).join("\n---\n");
  },
};
```

The domain serializer can emit markdown (for chant's file-based workflow) or JSON (for API-driven workflows). The serialization format is an implementation detail — the typed `Spec` objects are the source of truth.

## What Stays the Same

- Markdown specs still work. Write them by hand, have agents write them.
- `chant work` still spawns agents in worktrees.
- `.chant/specs/` directory still holds spec files.
- The MCP server still exposes `chant_*` tools.
- The watch process still monitors for completion.
- Git worktree isolation still applies.

## What Changes

- Chant is TypeScript, not Rust.
- Specs parse into typed `@intentius/chant` Declarables.
- `intentius lint` validates specs alongside infrastructure.
- Cross-domain references let specs target infrastructure resources.
- TypeScript specs are an alternative authoring path for power users.
- One type system across infrastructure and orchestration.

## Implementation Sequence

1. **`@intentius/chant` domain package** — Define `Spec`, `Group`, `Workflow` as Declarables. Implement the markdown parser that produces these types. Write the domain serializer.
2. **Lint rules** — WCH001-WCH008, validating spec structure and dependencies.
3. **Chant core port** — Rewrite chant's execution engine in TypeScript. Process management, worktree handling, agent spawning, MCP server.
4. **Cross-domain references** — Enable specs to reference infrastructure Declarables.
5. **TypeScript spec authoring** — Support `.ts` spec files alongside markdown.
