# Testing Strategy

## Approach

Design integration tests first. Unit tests emerge from implementation. Tests exercise the complete system.

See [philosophy](../getting-started/philosophy.md) for chant's broader design principles.

## Test Hierarchy

```
Integration Tests (high-level, design first)
    ↓
Component Tests (boundaries)
    ↓
Unit Tests (implementation details)
```

## Bootstrap Testing

Chant builds itself. Each phase has gate tests.

### Phase Gate Example

```rust
#[test]
fn test_phase0_self_hosting() {
    let repo = ChantRepo::open(".");
    let spec_id = repo.add_spec("Add test comment to main.go");

    repo.work_with_mock(&spec_id, |files| {
        files.append("cmd/main.go", "// test comment");
    });

    let spec = repo.read_spec(&spec_id);
    assert_eq!(spec.status, "completed");
    assert!(repo.file_contains("cmd/main.go", "// test comment"));

    repo.revert_last_commit();
}
```

### Phase Gates Summary

| Phase | Key Tests |
|-------|-----------|
| 0 → 1 | Self-hosting: chant executes spec on itself |
| 1 → 2 | Branching, isolation, hooks |
| 2 → 3 | MCP server tools, provider config |
| 3 → 4 | Cross-repo dependencies |
| 4 → 5 | Labels, triggers, spec types, groups |
| 5 → 6 | Cost tracking, linting, error catalog, reports |
| 6 → 7 | Search, locks, queue, pools, daemon |
| 7 → 8 | Drift detection, replay, verification |
| 8 → ✓ | Approvals, notifications, templates, git hooks |

---

## Integration Tests

End-to-end tests that exercise complete workflows.

### Core Workflow

```rust
#[test]
fn test_basic_workflow() {
    let repo = TempRepo::new();
    repo.run("chant init").success();

    let output = repo.run("chant add 'Fix bug'").success();
    let spec_id = output.spec_id();

    repo.run(&format!("chant work {} --agent mock", spec_id)).success();

    let spec = repo.read_spec(&spec_id);
    assert_eq!(spec.status, "completed");
}
```

### Parallel Execution

```rust
#[test]
fn test_parallel_members() {
    let repo = TempRepo::new();
    repo.init();

    let driver = repo.add_spec("Driver spec");
    repo.split(&driver);

    repo.run(&format!("chant work {} --parallel --max 3", driver)).success();

    for member in repo.members(&driver) {
        assert_eq!(member.status, "completed");
    }
    assert_eq!(repo.read_spec(&driver).status, "completed");
}
```

### Dependency Chain

```rust
#[test]
fn test_dependency_chain() {
    let repo = TempRepo::new();
    repo.init();

    let spec_a = repo.add_spec("Spec A");
    let spec_b = repo.add_spec_with_dep("Spec B", &spec_a);
    let spec_c = repo.add_spec_with_dep("Spec C", &spec_b);

    assert!(repo.is_blocked(&spec_c));

    repo.work(&spec_a);
    assert!(repo.is_ready(&spec_b));
    assert!(repo.is_blocked(&spec_c));

    repo.work(&spec_b);
    repo.work(&spec_c);
    assert_eq!(repo.read_spec(&spec_c).status, "completed");
}
```

### Lock Contention

```rust
#[test]
fn test_lock_contention() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Contested spec");
    let handle1 = repo.work_async(&spec);
    thread::sleep(Duration::from_millis(100));

    let result = repo.run(&format!("chant work {}", spec));
    assert!(result.stderr.contains("locked"));

    handle1.wait();
}
```

### Error Recovery

```rust
#[test]
fn test_crash_recovery() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Crash test");
    repo.create_stale_lock(&spec, 99999);

    assert!(repo.is_locked(&spec));
    repo.run(&format!("chant unlock {}", spec)).success();
    repo.work(&spec);
}
```

---

## Component Tests

### Parser Tests

```rust
#[test]
fn test_spec_parser() {
    let content = r#"---
status: pending
labels: [urgent, bug]
---

# Fix authentication

Login fails on Safari.
"#;

    let spec = parse_spec(content).unwrap();
    assert_eq!(spec.status, "pending");
    assert_eq!(spec.labels, vec!["urgent", "bug"]);
}
```

### Search Index Tests

```rust
#[test]
fn test_search_index() {
    let index = TantivyIndex::new_temp();

    index.add(Spec { id: "001", status: "pending", body: "Fix authentication bug" });

    assert_eq!(index.search("status:pending").len(), 1);
    assert_eq!(index.search("authentication").len(), 1);
    assert_eq!(index.search("payment").len(), 0);
}
```

---

## Test Fixtures

### Mock Agent

```rust
struct MockAgent {
    should_succeed: bool,
    delay: Duration,
}

impl Agent for MockAgent {
    fn execute(&self, spec: &Spec) -> Result<()> {
        thread::sleep(self.delay);
        if self.should_succeed { Ok(()) }
        else { Err(Error::AgentFailed("Mock failure")) }
    }
}
```

### Temp Repository

```rust
struct TempRepo {
    dir: TempDir,
}

impl TempRepo {
    fn new() -> Self { ... }
    fn init(&self) { ... }
    fn add_spec(&self, desc: &str) -> String { ... }
    fn work(&self, spec_id: &str) { ... }
    fn read_spec(&self, spec_id: &str) -> Spec { ... }
}
```

---

## CI Configuration

```yaml
# .github/workflows/test.yml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - run: cargo test --lib           # Unit tests
      - run: cargo test --test integration
      - run: cargo test --test slow -- --test-threads=1
```

## Coverage Goals

| Component | Target |
|-----------|--------|
| Core workflow | 90% |
| Parser | 95% |
| MCP server | 85% |
| Search | 80% |
| CLI | 70% |

## Test Categories

```bash
cargo test                           # All tests
cargo test --lib                     # Fast (unit + component)
cargo test --test integration        # Integration only
cargo test --test gates              # Phase gate tests
cargo test --test real_agent --ignored  # With real agent (slow)
```
