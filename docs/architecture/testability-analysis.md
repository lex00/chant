# Testability Analysis

**Date:** 2026-02-04
**Coverage:** ~50% (467 unit tests, 59 integration tests)
**Total Source:** ~52k lines across 104+ Rust files

## Executive Summary

Chant's test suite is **quantity-focused** rather than **quality-focused**. While there are 467 unit tests achieving 50% coverage, the codebase suffers from three critical architectural barriers:

1. **Hard coupling to filesystem/git operations** - Core logic is entangled with I/O
2. **No abstraction boundaries** - Direct `Command::new("git")` calls throughout
3. **Business logic in command handlers** - 900+ line functions mixing concerns

The tests validate behavior but are brittle, slow, and difficult to maintain. Most test failures would be from infrastructure (git, filesystem) rather than logic bugs.

## Current Test Quality

### Unit vs Integration Ratio
- **Unit tests:** 467 (in `src/` with `#[cfg(test)]`)
- **Integration tests:** 59 tests across 21 files (~7,184 lines)
- **Ratio:** 8:1 unit to integration

**Analysis:** Ratio appears healthy, but unit tests often behave like integration tests due to filesystem/git dependencies.

### Test Effectiveness

**Good:**
- Extensive coverage of edge cases (checkbox counting, frontmatter parsing, dependency resolution)
- Tests verify actual behavior, not just implementation details
- Integration tests properly test worktree isolation and concurrent operations

**Problematic:**
- Many "unit" tests require real filesystem operations
- Integration tests average 120+ lines each (setup-heavy)
- No mocking framework - tests rely on actual git repositories
- Slow test suite (26.5s for unit tests alone)
- Tests repeat similar setup code (see `tests/common.rs`)

### Test Patterns

Example from `tests/integration/worktree_test.rs` (lines 85-125):
```rust
#[test]
fn test_worktree_creation_basic() {
    let repo_dir = PathBuf::from("/tmp/test-chant-wt-basic");
    common::setup_test_repo(&repo_dir)?;  // Real git init

    Command::new("git")
        .args(["worktree", "add", "-b", &branch, &wt_path_str])
        .output()?;

    assert!(worktree_exists(&worktree_path));
}
```

**Issue:** Tests require actual git state, making them slow and environment-dependent.

## Architectural Barriers

### 1. Mixed IO and Logic

**Example:** `src/cmd/work/single.rs` (lines 117-874)

The `cmd_work` function is 758 lines mixing:
- CLI argument parsing
- Spec validation logic
- Git operations
- Agent invocation
- Status file writes
- Error formatting
- User prompts

**Testability Impact:**
- Cannot test approval logic without filesystem
- Cannot test quality scoring without real specs
- Cannot test finalization without git operations

**Evidence:**
```rust
pub fn cmd_work(
    ids: &[String],
    prompt_name: Option<&str>,
    skip_deps: bool,
    skip_criteria: bool,
    parallel: bool,
    // ... 13 more parameters
) -> Result<()> {
    let specs_dir = crate::cmd::ensure_initialized()?;  // Filesystem check

    if !no_watch {
        super::ensure_watch_running()?;  // Process spawning
    }

    let mut spec = spec::resolve_spec(&specs_dir, id)?;  // File I/O
    let spec_path = specs_dir.join(format!("{}.md", spec.id));

    // ... 740 more lines of mixed concerns
}
```

### 2. No Dependency Injection

**Example:** `src/git.rs` (lines 39-51)

Git operations are hardcoded:
```rust
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to run git rev-parse")?;

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch)
}
```

**Problem:** No way to test logic that depends on git without actual git repository.

**Missing:** Trait abstraction for git operations:
```rust
// Does not exist currently
trait GitOperations {
    fn get_current_branch(&self) -> Result<String>;
    fn branch_exists(&self, branch: &str) -> Result<bool>;
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()>;
}
```

### 3. Business Logic in Command Handlers

**Example:** `src/cmd/work/single.rs` (lines 414-543)

Quality scoring logic is embedded in command handler:
```rust
if !skip_criteria {
    let quality_score = chant::scoring::calculate_spec_score(&spec, &all_specs, &config);

    match quality_score.traffic_light {
        TrafficLight::Refine => {
            eprintln!("Warning: Spec has quality issues");
            // ... 70 lines of display logic

            if atty::is(atty::Stream::Stdin) {
                print!("Continue anyway? [y/N] ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                // ... decision logic
            }
        }
        // ... more cases
    }
}
```

**Testability Issues:**
- Cannot test quality gate logic without TTY handling
- Display logic mixed with decision logic
- Hard to test different user responses

### 4. Concrete Types Everywhere

**Missing Traits:** Only 8 trait definitions found:
- `src/provider.rs`: Provider trait (for model providers)
- `src/cmd/dispatch.rs`: Execute trait (for commands)
- `src/cmd/lifecycle/split.rs`: 1 trait
- `src/cmd/validate.rs`: 1 trait

**Impact:** Core functionality like spec operations, git operations, and file I/O cannot be swapped with test doubles.

## Specific Areas Analysis

### `src/cmd/work/` - Work Execution Logic

**Testability Grade: D**

Problems:
- `single.rs`: 904 lines, 17 parameters, no separation of concerns
- Direct filesystem access throughout
- Worktree creation tightly coupled to execution
- Cannot test workflow without spawning processes

Example needed change:
```rust
// Current: Impossible to test without real worktree
let worktree_path = worktree::create_worktree(&spec.id, &branch_name)?;

// Refactored: Inject worktree manager
struct WorkContext<W: WorktreeManager> {
    worktree_mgr: W,
    spec_loader: Box<dyn SpecLoader>,
    git: Box<dyn GitOps>,
}

fn execute_work<W: WorktreeManager>(ctx: &WorkContext<W>, spec_id: &str) -> Result<()> {
    let worktree = ctx.worktree_mgr.create(&spec_id)?;
    // ... testable logic
}
```

### `src/mcp/` - MCP Handlers

**Testability Grade: C+**

Better than average:
- Handlers are relatively pure (lines 17-24 in `handlers.rs`)
- JSON in/out makes testing easier
- Less filesystem dependency

Issues:
- Still calls through to commands that touch filesystem
- No mock layer for spec operations
- Integration tests required for handler coverage

### `src/git.rs` - Git Logic

**Testability Grade: F**

Critical issues:
- Zero abstraction - all functions call `Command::new("git")`
- Cannot test branch logic without git repository
- No trait boundary for testing

**Current:** 100+ lines of direct command execution
**Tests:** Rely on real git repositories (see `tests/common.rs` lines 7-41)

### `src/spec.rs` - Spec Operations

**Testability Grade: B**

Relatively good:
- Pure parsing logic (lines 66-183 show good test coverage)
- Frontmatter handling is testable
- Clear separation in parse module

Issues still present:
- `load_all_specs` requires real filesystem
- `resolve_spec` does I/O directly
- Lifecycle operations mixed with storage

### `src/main.rs` - CLI Entry

**Testability Grade: C**

Structure is better than expected:
- Uses `Execute` trait for commands (line 22 in main.rs)
- Command dispatch is abstracted
- CLI parsing separated from execution

Issue:
- Little unit test coverage on main (only 3 test modules per grep)
- Most commands still do I/O directly

## Root Causes

### Why is testability poor?

1. **Evolved organically** - Started as CLI tool, not designed for testing
2. **No upfront abstraction** - Direct implementation over interfaces
3. **Rust's ownership** - Made developers prefer concrete types over trait objects
4. **Time pressure** - 52k lines built quickly prioritized features over architecture
5. **Integration tests as crutch** - "It works in CI" satisfied the bar

## Refactoring Proposals

### Priority 1: Extract Core Domain Logic

**Goal:** Separate business rules from I/O

**Changes:**
- Create `src/domain/` module with pure logic
- Move spec validation, dependency checking, quality scoring into domain
- Domain functions take data, not paths

**Example:**
```rust
// New: src/domain/spec_validation.rs
pub fn is_spec_ready(spec: &SpecData, all_specs: &[SpecData]) -> ReadinessCheck {
    // Pure logic, no I/O
    if spec.status != SpecStatus::Pending {
        return ReadinessCheck::NotReady(Reason::WrongStatus);
    }
    // ... more pure checks
}

// Command layer becomes thin
pub fn cmd_work_on_spec(spec_id: &str) -> Result<()> {
    let spec = load_spec(spec_id)?;  // I/O
    let all_specs = load_all_specs()?;  // I/O

    match domain::is_spec_ready(&spec, &all_specs) {  // Pure
        ReadinessCheck::Ready => execute_work(spec),
        ReadinessCheck::NotReady(reason) => display_error(reason),
    }
}
```

**Benefit:** Can test all validation logic with in-memory data

### Priority 2: Introduce Repository Pattern

**Goal:** Abstract filesystem and git operations

**Changes:**
```rust
// src/repository/spec_repository.rs
pub trait SpecRepository {
    fn load(&self, id: &str) -> Result<Spec>;
    fn save(&self, spec: &Spec) -> Result<()>;
    fn list_all(&self) -> Result<Vec<Spec>>;
}

pub struct FileSpecRepository {
    base_path: PathBuf,
}

pub struct InMemorySpecRepository {
    specs: HashMap<String, Spec>,
}

// src/repository/git_repository.rs
pub trait GitRepository {
    fn get_current_branch(&self) -> Result<String>;
    fn create_branch(&self, name: &str) -> Result<()>;
    fn create_worktree(&self, branch: &str, path: &Path) -> Result<()>;
}
```

**Benefit:** Tests can use `InMemorySpecRepository`, production uses `FileSpecRepository`

### Priority 3: Break Up Large Functions

**Goal:** Make functions testable in isolation

**Target:** Functions over 100 lines (especially `cmd_work`)

**Strategy:**
```rust
// Current: 758 lines, untestable
pub fn cmd_work(/* 17 params */) -> Result<()> { /* everything */ }

// Refactored: Multiple testable functions
fn validate_spec_ready(spec: &Spec, all_specs: &[Spec], options: &WorkOptions) -> Result<()>
fn assess_quality_score(spec: &Spec, all_specs: &[Spec], config: &Config) -> Result<QualityDecision>
fn prepare_worktree(spec_id: &str, git: &dyn GitRepository) -> Result<WorktreePath>
fn execute_agent(spec: &Spec, prompt: &str, agent: &dyn AgentRunner) -> Result<Output>
fn finalize_work(spec: &Spec, output: &Output, git: &dyn GitRepository) -> Result<()>

pub fn cmd_work(/* 17 params */) -> Result<()> {
    validate_spec_ready(&spec, &all_specs, &options)?;
    let decision = assess_quality_score(&spec, &all_specs, &config)?;
    let worktree = prepare_worktree(&spec.id, &git)?;
    let output = execute_agent(&spec, &prompt, &agent)?;
    finalize_work(&spec, &output, &git)?;
    Ok(())
}
```

**Benefit:** Each function can be unit tested with mocks

### Priority 4: Add Test Utilities

**Goal:** Make writing tests easier

**Changes:**
```rust
// tests/support/builders.rs
pub struct SpecBuilder {
    id: String,
    status: SpecStatus,
    dependencies: Vec<String>,
}

impl SpecBuilder {
    pub fn new(id: &str) -> Self { /* ... */ }
    pub fn with_status(mut self, status: SpecStatus) -> Self { /* ... */ }
    pub fn with_dependency(mut self, dep_id: &str) -> Self { /* ... */ }
    pub fn build(self) -> Spec { /* ... */ }
}

// Usage in tests
let spec = SpecBuilder::new("001")
    .with_status(SpecStatus::Pending)
    .with_dependency("002")
    .build();
```

**Benefit:** Reduce test boilerplate, focus on what's being tested

### Priority 5: Introduce Feature Facades

**Goal:** Higher-level APIs that hide complexity

**Example:**
```rust
// src/services/work_service.rs
pub struct WorkService {
    spec_repo: Box<dyn SpecRepository>,
    git_repo: Box<dyn GitRepository>,
    agent: Box<dyn AgentRunner>,
}

impl WorkService {
    pub fn execute_spec(&self, spec_id: &str, options: WorkOptions) -> Result<WorkResult> {
        // Orchestrates repositories and domain logic
        // Command handlers become one-liners
    }
}

// In cmd/work/single.rs
pub fn cmd_work(/* params */) -> Result<()> {
    let service = WorkService::from_config(&config)?;
    service.execute_spec(spec_id, options.into())?;
    Ok(())
}
```

**Benefit:** Services can be tested with mock repositories, commands become trivial

## Implementation Strategy

### Phase 1: Add Repository Traits (2-3 weeks)
1. Define `SpecRepository` and `GitRepository` traits
2. Implement `FileSpecRepository` and `CommandGitRepository` (current behavior)
3. Implement `InMemorySpecRepository` and `MockGitRepository` for tests
4. Refactor one module (`src/spec.rs`) to use repositories

### Phase 2: Extract Domain Logic (3-4 weeks)
1. Create `src/domain/` module
2. Move validation logic from commands to domain
3. Move quality scoring logic to domain
4. Move dependency resolution to domain
5. Add unit tests for all domain functions (targeting 90%+ coverage)

### Phase 3: Refactor Commands (4-6 weeks)
1. Break up large command functions (start with `cmd_work`)
2. Extract sub-functions with clear responsibilities
3. Make commands use repositories and domain logic
4. Replace integration tests with fast unit tests where possible

### Phase 4: Add Test Utilities (1-2 weeks)
1. Create builder pattern helpers
2. Add fixture generation utilities
3. Document testing patterns
4. Refactor existing tests to use utilities

## Success Metrics

**Before:**
- 467 unit tests (50% coverage)
- 26.5s unit test runtime
- Tests require filesystem/git
- Adding tests is painful

**After (6-month target):**
- 800+ unit tests (75%+ coverage)
- <5s unit test runtime for fast tests
- 90% of tests use in-memory implementations
- New tests are easy to write (builders + mocks)

## Risks

1. **Scope creep** - Refactoring can consume months if not bounded
2. **Breaking changes** - Public API changes may break downstream users
3. **Test confidence** - Mocks might not match real behavior

**Mitigations:**
- Use strangler fig pattern: New code uses abstractions, old code unchanged
- Keep both real and mock implementations passing same test suite
- Refactor incrementally, maintain passing tests throughout

## Recommendations

1. **Immediate:** Add repository traits for most-changed code (specs, git)
2. **Next sprint:** Extract domain logic from one command as proof-of-concept
3. **Long-term:** Adopt test-first development for new features
4. **Culture:** Make testability a PR review requirement

## Appendix: Test Coverage by Module

| Module | Unit Tests | Coverage | Testability Grade |
|--------|-----------|----------|-------------------|
| `src/spec.rs` | 40+ | 70%+ | B |
| `src/git.rs` | 10 | 30% | F |
| `src/cmd/work/` | 20 | 20% | D |
| `src/mcp/` | 15 | 50% | C+ |
| `src/worktree/` | 25 | 60% | C |
| `src/scoring.rs` | 80+ | 80% | A- |
| `src/config/` | 60+ | 75% | B+ |

**Note:** Coverage estimated from test counts and grep analysis. Run `cargo tarpaulin` for exact numbers.
