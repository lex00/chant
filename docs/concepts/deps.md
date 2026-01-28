# Dependencies

## Basics

Specs can depend on other specs:

```yaml
---
status: pending
depends_on:
  - 2026-01-22-001-x7m
  - 2026-01-22-002-q2n
---
```

A spec is **blocked** if any dependency is not `completed`.

## Dependency Graph

```
    ┌─────┐
    │  A  │ (no deps, ready)
    └──┬──┘
       │
    ┌──▼──┐
    │  B  │ (depends on A, blocked)
    └──┬──┘
       │
    ┌──▼──┐
    │  C  │ (depends on B, blocked)
    └─────┘
```

When A completes → B becomes ready → when B completes → C becomes ready.

## Cycle Detection

Cycles are invalid:

```
A depends on B
B depends on C
C depends on A  ← cycle!
```

Detected at:
1. `chant lint` - Validates all specs
2. `chant add --depends-on` - Checks before adding
3. Spec file save - Editor integration (future)

```rust
fn detect_cycles(specs: &[Spec]) -> Vec<Cycle> {
    let graph = build_graph(specs);
    petgraph::algo::tarjan_scc(&graph)
        .into_iter()
        .filter(|scc| scc.len() > 1)
        .collect()
}
```

## Parallel Execution

Independent specs can run in parallel:

```
    ┌─────┐     ┌─────┐
    │  A  │     │  B  │  ← Both ready, can run in parallel
    └──┬──┘     └──┬──┘
       │           │
       └─────┬─────┘
             │
          ┌──▼──┐
          │  C  │  ← Blocked until both A and B complete
          └─────┘
```

```bash
chant work --parallel          # Run all ready specs in parallel
chant work --parallel --max 3  # Limit concurrent agents
```

## Split and Continue

For epic decomposition with parallel execution:

```bash
chant split 2026-01-22-001-x7m --then work --parallel
```

1. Split creates members (.1, .2, .3, ...)
2. Members without deps become ready
3. Ready members execute in parallel
4. As members complete, blocked ones become ready
5. Continue until all members done

## Ready Calculation

```rust
fn is_ready(spec: &Spec, all_specs: &[Spec]) -> bool {
    // Must be pending
    if spec.status != "pending" {
        return false;
    }

    // All dependencies must be completed
    for dep_id in &spec.depends_on {
        let dep = find_spec(all_specs, dep_id);
        if dep.status != "completed" {
            return false;
        }
    }

    // If has group members, all must be completed
    let members = find_group_members(all_specs, &spec.id);
    if !members.is_empty() {
        for member in members {
            if member.status != "completed" {
                return false;
            }
        }
    }

    true
}
```

## Blocked Specs

Attempting to work on a blocked spec:

```bash
$ chant work 2026-01-22-003-abc
✗ Spec has unsatisfied dependencies.
Blocked by: 2026-01-22-001-x7m (in_progress), 2026-01-22-002-q2n (pending)
Use --force to bypass dependency checks.
```

### Forcing Work on Blocked Specs

Use `--force` to bypass dependency checks when you know dependencies are satisfied despite their status:

```bash
$ chant work 2026-01-22-003-abc --force
⚠ Warning: Forcing work on spec (skipping dependency checks)
  Skipping dependencies: 2026-01-22-001-x7m (in_progress), 2026-01-22-002-q2n (pending)
→ Working 2026-01-22-003-abc with prompt 'standard'
...
```

**Use Cases for `--force`:**
1. **Dependency tracking bug** - Spec shows as blocked but dependencies are actually completed
2. **Manual override** - You verified dependencies are satisfied despite their status
3. **Emergency work** - Need to proceed despite dependency chain issues
4. **Testing** - Want to work on spec to test dependency resolution

**Warning:** Forcing work on a truly blocked spec may lead to conflicts or incomplete work if dependencies aren't actually satisfied.

## Dependency Visualization

```bash
$ chant deps 2026-01-22-003-abc
2026-01-22-003-abc
├── 2026-01-22-001-x7m [completed]
└── 2026-01-22-002-q2n [pending]
    └── 2026-01-22-004-def [in_progress]
```

## Cross-Spec References

Specs can reference each other in body text:

```markdown
See also: [[2026-01-22-001-x7m]]

This continues work from [[2026-01-22-002-q2n]].
```

`[[id]]` syntax is for documentation only. Not enforced as dependencies.

Use `depends_on` for actual blocking relationships.
