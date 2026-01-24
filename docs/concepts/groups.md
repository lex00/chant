# Spec Groups

Specs can be split into groups. One spec **drives** the group - it completes when all members complete.

## The Model

```
Driver spec
├── Member 1: completed  ─┐
├── Member 2: completed   ├─→ Driver auto-completes
└── Member 3: completed  ─┘
```

This is **status aggregation**, not hierarchy:
- Members report status back
- Members can depend on each other
- Members don't inherit driver properties
- Driver completes when all members complete

## Group Structure from Filenames

Group membership is determined by filename suffix:

```
2026-01-22-001-x7m.md       ← Driver
2026-01-22-001-x7m.1.md     ← Member 1
2026-01-22-001-x7m.2.md     ← Member 2
2026-01-22-001-x7m.2.1.md   ← Subgroup (member 2 is also a driver)
```

The `.N` suffix establishes group membership. No `group` field needed.

## Creating Groups

### Via Split

```bash
chant split 2026-01-22-001-x7m
```

Agent analyzes the driver spec and creates members:

```
2026-01-22-001-x7m.1.md
2026-01-22-001-x7m.2.md
2026-01-22-001-x7m.3.md
```

### Manually

```bash
chant add "Subspec description" --group 2026-01-22-001-x7m
# Creates: 2026-01-22-001-x7m.4.md
```

## Driver Auto-Completion

When all members are `completed`, the driver automatically completes:

```
Driver: pending
├── Member 1: completed
├── Member 2: completed
└── Member 3: completed
              ↓
Driver: completed (auto)
```

```rust
fn check_group_completion(driver_id: &str, specs: &[Spec]) -> bool {
    let members = find_group_members(specs, driver_id);

    if members.is_empty() {
        return false;  // No members, no auto-complete
    }

    members.iter().all(|m| m.status == "completed")
}

fn maybe_complete_driver(driver_id: &str, specs: &mut [Spec]) {
    if check_group_completion(driver_id, specs) {
        let driver = find_spec_mut(specs, driver_id);
        driver.status = "completed";
        driver.completed_at = Some(Utc::now());
        driver.auto_completed = true;  // Flag for audit
    }
}
```

## Working on Groups

### Driver with no members

Works like any spec:

```bash
chant work 2026-01-22-001-x7m  # Executes directly
```

### Driver with members

Working a driver automatically works its members:

```bash
chant work 2026-01-22-001-x7m             # Sequential (one at a time)
chant work 2026-01-22-001-x7m --parallel   # Parallel execution
chant work 2026-01-22-001-x7m --max 3      # Parallel with concurrency limit
```

Or work a specific member:

```bash
chant work 2026-01-22-001-x7m.2   # Just this member
```

## Member Dependencies

Members can depend on other members:

```yaml
# 2026-01-22-001-x7m.2.md
---
status: pending
depends_on:
  - 2026-01-22-001-x7m.1   # Must complete first
---
```

Execution order respects dependencies:

```bash
chant work 2026-01-22-001-x7m --parallel
# Runs .1 first, then .2 after .1 completes (respects depends_on)
```

## Nested Groups

A member can also be a driver for its own subgroup:

```
2026-01-22-001-x7m.md           ← Driver (top)
2026-01-22-001-x7m.2.md         ← Member of top, driver of subgroup
2026-01-22-001-x7m.2.1.md       ← Member of subgroup
2026-01-22-001-x7m.2.2.md       ← Member of subgroup
```

When .2.1 and .2.2 complete → .2 auto-completes → when all .N complete → top driver auto-completes.

## Listing Group Members

```bash
$ chant group 2026-01-22-001-x7m
2026-01-22-001-x7m.1  [completed]  Setup database schema
2026-01-22-001-x7m.2  [pending]    Implement API endpoints
2026-01-22-001-x7m.3  [pending]    Add authentication
```

## Orphan Detection

Members without a valid driver are flagged:

```bash
$ chant lint
Warning: 2026-01-22-001-x7m.5.md references non-existent driver
```

## What Groups Are NOT

Groups are not:
- **Inheritance** - Members don't inherit driver properties (labels, prompt, priority)
- **Hierarchy** - It's a flat group with a designated driver
- **Ownership** - Members are independent specs that happen to report status

If you need shared properties, set them explicitly on each member or use a project-level prompt.
