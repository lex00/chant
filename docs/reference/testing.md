# Testing Strategy

## Philosophy

Design integration tests first. Unit tests emerge from implementation. Tests exercise the complete system.

## Bootstrap Testing

Chant builds itself after Phase 0. Each phase has gate tests.

### Phase 0 Gate: Self-Hosting

Phase 0 is complete when chant can execute specs on itself:

```bash
# Bootstrap validation
$ chant add "Add --verbose flag to list command"
Created: 2026-01-22-001-x7m

$ chant work 001
[Agent executes spec]
[Makes changes to cmd/list.go]
[Commits changes]

$ chant show 001
status: completed
commit: abc123

# The test: did it actually work?
$ chant list --verbose
[Shows verbose output]
```

```rust
#[test]
fn test_phase0_self_hosting() {
    // Use real chant repo (not temp)
    let repo = ChantRepo::open(".");

    // Create a spec that modifies chant itself
    let spec_id = repo.add_spec("Add test comment to main.go");

    // Work on it with mock agent that adds comment
    repo.work_with_mock(&spec_id, |files| {
        files.append("cmd/main.go", "// test comment");
    });

    // Verify committed
    let spec = repo.read_spec(&spec_id);
    assert_eq!(spec.status, "completed");

    // Verify change exists
    assert!(repo.file_contains("cmd/main.go", "// test comment"));

    // Clean up
    repo.revert_last_commit();
}
```

### Phase 1 Gate: Git+

```rust
#[test]
fn test_phase1_branching() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        git:
          branch: true
    "#);

    let spec = repo.add_spec("Test branching");
    repo.work(&spec);

    // Verify branch created
    let branches = repo.git_branches();
    assert!(branches.iter().any(|b| b.contains(&spec)));
}

#[test]
fn test_phase1_pr_creation() {
    let repo = TempRepo::new_with_remote(); // Has GitHub remote
    repo.init_with_config(r#"
        git:
          branch: true
          pr: true
    "#);

    let spec = repo.add_spec("Test PR");
    repo.work(&spec);

    // Verify PR created (mock GitHub API)
    assert!(repo.mock_github.pr_created());
}

#[test]
fn test_phase1_isolation() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        isolation: clone
    "#);

    let spec = repo.add_spec("Test isolation");
    let handle = repo.work_async(&spec);

    // Verify clone exists during work
    assert!(repo.clone_exists(&spec));

    handle.wait();

    // Clone cleaned up after
    assert!(!repo.clone_exists(&spec));
}

#[test]
fn test_phase1_post_work_hooks() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        hooks:
          post_work:
            - "echo 'done' >> .chant/hook.log"
    "#);

    let spec = repo.add_spec("Test hooks");
    repo.work(&spec);

    // Verify hook ran
    assert!(repo.exists(".chant/hook.log"));
    assert!(repo.file_contains(".chant/hook.log", "done"));
}
```

### Phase 2 Gate: MCP + Kiro

```rust
#[test]
fn test_phase2_mcp_server() {
    // Start MCP server
    let mcp = ChantMcpServer::start();

    // List tools
    let tools = mcp.list_tools();
    assert!(tools.contains("chant_spec_get"));
    assert!(tools.contains("chant_spec_list"));

    // Call a tool
    let result = mcp.call_tool("chant_spec_list", json!({}));
    assert!(result.is_ok());
}

#[test]
fn test_phase2_kiro_provider() {
    // Skip if kiro-cli not available
    if !kiro_available() {
        return;
    }

    let repo = TempRepo::new();
    repo.init_with_config(r#"
        agent:
          provider: kiro
    "#);

    // Verify MCP config generated
    let spec = repo.add_spec("Test spec");
    repo.work(&spec);

    assert!(repo.exists(".kiro/mcp.json"));
}
```

### Phase 3 Gate: Multi-Repo

```rust
#[test]
fn test_phase3_multi_repo() {
    let repo1 = TempRepo::new();
    let repo2 = TempRepo::new();
    repo1.init();
    repo2.init();

    // Configure global
    let global = GlobalConfig::temp();
    global.add_repo("repo1", &repo1.path);
    global.add_repo("repo2", &repo2.path);

    // Create specs in both
    let spec1 = repo1.add_spec("Spec in repo1");
    let spec2 = repo2.add_spec_with_dep(
        "Spec in repo2",
        &format!("repo1:{}", spec1)  // Cross-repo dep
    );

    // Verify blocked
    assert!(repo2.is_blocked(&spec2));

    // Complete spec1
    repo1.work(&spec1);

    // Now spec2 is ready
    assert!(repo2.is_ready(&spec2));
}
```

### Phase 4 Gate: Structure

```rust
#[test]
fn test_phase4_labels() {
    let repo = TempRepo::new();
    repo.init();

    repo.add_spec_with_labels("Bug fix", &["bug", "urgent"]);
    repo.add_spec_with_labels("Feature", &["feature"]);
    repo.add_spec_with_labels("Another bug", &["bug"]);

    // Filter by label
    let bugs = repo.list_specs_with_label("bug");
    assert_eq!(bugs.len(), 2);

    let urgent = repo.list_specs_with_label("urgent");
    assert_eq!(urgent.len(), 1);
}

#[test]
fn test_phase4_circular_dependency() {
    let repo = TempRepo::new();
    repo.init();

    let spec_a = repo.add_spec("Spec A");
    let spec_b = repo.add_spec_with_dep("Spec B", &spec_a);

    // Try to make A depend on B (circular)
    let result = repo.add_dependency(&spec_a, &spec_b);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("circular"));
}

#[test]
fn test_phase4_triggers() {
    let repo = TempRepo::new();
    repo.init();

    // Create spec with trigger
    let trigger_spec = repo.add_spec_with_frontmatter(r#"---
status: pending
triggers:
  on_complete:
    - "chant add 'Follow-up task'"
---

# Trigger test
"#);

    repo.work(&trigger_spec);

    // Verify triggered spec was created
    let specs = repo.list_specs();
    assert!(specs.iter().any(|s| s.title.contains("Follow-up")));
}

#[test]
fn test_phase4_spec_types() {
    let repo = TempRepo::new();
    repo.init();

    // Create specs of different types
    repo.add_spec_with_type("Implementation task", "implementation");
    repo.add_spec_with_type("API docs", "documentation");
    repo.add_spec_with_type("Survey analysis", "research");

    // Filter by type
    let docs = repo.list_specs_with_type("documentation");
    assert_eq!(docs.len(), 1);

    let research = repo.list_specs_with_type("research");
    assert_eq!(research.len(), 1);
}

#[test]
fn test_phase4_groups() {
    let repo = TempRepo::new();
    repo.init();

    // Create driver spec
    let driver = repo.add_spec("Driver spec");

    // Split into members
    repo.split(&driver);

    // Verify group structure
    let group = repo.get_group(&driver);
    assert!(group.members.len() > 0);

    // Work members in parallel
    repo.run(&format!("chant work {} --parallel", driver)).success();

    // All members complete → driver auto-completes
    let driver_spec = repo.read_spec(&driver);
    assert_eq!(driver_spec.status, "completed");
}
```

### Phase 5 Gate: Observability

```rust
#[test]
fn test_phase5_cost_tracking() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Cost test");
    repo.work(&spec);

    // Verify cost recorded
    let spec = repo.read_spec(&spec);
    assert!(spec.cost.is_some());
    assert!(spec.cost.unwrap().tokens > 0);
}

#[test]
fn test_phase5_output_logging() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Logging test");
    repo.work(&spec);

    // Verify log file exists
    let log_path = format!(".chant/logs/{}.log", spec);
    assert!(repo.exists(&log_path));

    // Verify log contains output
    let log = repo.read(&log_path);
    assert!(log.len() > 0);
}

#[test]
fn test_phase5_linting() {
    let repo = TempRepo::new();
    repo.init();

    // Create spec missing acceptance criteria
    let spec = repo.add_spec_raw(r#"---
status: pending
---

# Vague spec

Do something.
"#);

    // Lint should warn
    let result = repo.run(&format!("chant lint {}", spec));
    assert!(result.stdout.contains("missing acceptance criteria"));
}

#[test]
fn test_phase5_error_catalog() {
    let repo = TempRepo::new();
    repo.init();

    // Trigger a known error
    let result = repo.run("chant work nonexistent-spec");

    // Error should have catalog code
    assert!(result.stderr.contains("CHANT-E001")); // Spec not found
    assert!(result.exit_code != 0);
}

#[test]
fn test_phase5_reports() {
    let repo = TempRepo::new();
    repo.init();

    // Create and complete some specs
    for i in 0..3 {
        let spec = repo.add_spec(&format!("Spec {}", i));
        repo.work(&spec);
    }

    // Generate report
    let result = repo.run("chant report --format json").success();
    let report: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();

    assert_eq!(report["total_specs"], 3);
    assert_eq!(report["completed"], 3);
    assert!(report["total_cost"].as_f64().unwrap() > 0.0);
}
```

### Phase 6 Gate: Scale

```rust
#[test]
fn test_phase6_tantivy_search() {
    let repo = TempRepo::new();
    repo.init();

    repo.add_spec("Fix authentication bug in login");
    repo.add_spec("Add payment processing");
    repo.add_spec("Update auth tokens");

    // Full-text search
    let results = repo.search("authentication");
    assert_eq!(results.len(), 1);

    // Partial match
    let results = repo.search("auth");
    assert_eq!(results.len(), 2);

    // Field search
    let results = repo.search("status:pending");
    assert_eq!(results.len(), 3);
}

#[test]
fn test_phase6_queue() {
    let repo = TempRepo::new();
    repo.init();

    // Add multiple specs
    for i in 0..5 {
        repo.add_spec(&format!("Spec {}", i));
    }

    // Start queue with max 2 concurrent
    let queue = repo.start_queue(2);

    // Should process all
    queue.wait_all();

    // All completed
    let specs = repo.list_specs();
    assert!(specs.iter().all(|s| s.status == "completed"));
}

#[test]
fn test_phase6_daemon_global() {
    let repo1 = TempRepo::new();
    let repo2 = TempRepo::new();
    repo1.init();
    repo2.init();

    let global = GlobalConfig::temp();
    global.add_repo("repo1", &repo1.path);
    global.add_repo("repo2", &repo2.path);

    // Start global daemon
    let daemon = global.start_daemon_global();

    // Add specs to both repos
    repo1.add_spec("Spec in repo1");
    repo2.add_spec("Spec in repo2");

    // Daemon sees both
    let status = daemon.status();
    assert_eq!(status.total_specs, 2);
    assert!(status.repos.contains("repo1"));
    assert!(status.repos.contains("repo2"));

    daemon.stop();
}

#[test]
fn test_phase6_locks() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Lock test");

    // Acquire lock
    let handle = repo.work_async(&spec);
    thread::sleep(Duration::from_millis(100));

    // Verify locked
    assert!(repo.is_locked(&spec));

    // Second attempt fails
    let result = repo.run(&format!("chant work {}", spec));
    assert!(result.stderr.contains("locked"));

    handle.wait();

    // Lock released
    assert!(!repo.is_locked(&spec));
}

#[test]
fn test_phase6_pools() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        pools:
          fast:
            max_concurrent: 5
            priority: high
          slow:
            max_concurrent: 2
            priority: low
    "#);

    // Create specs in different pools
    repo.add_spec_with_pool("Fast task 1", "fast");
    repo.add_spec_with_pool("Fast task 2", "fast");
    repo.add_spec_with_pool("Slow task", "slow");

    // Start queue respects pools
    let queue = repo.start_queue_with_pools();
    queue.wait_all();

    // All completed
    let specs = repo.list_specs();
    assert!(specs.iter().all(|s| s.status == "completed"));
}
```

### Phase 6.5 Gate: Semantic Search (Optional)

```rust
#[test]
#[ignore] // Requires fastembed-rs models downloaded
fn test_phase6_5_semantic_search() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        search:
          semantic: true
    "#);

    repo.add_spec("Fix authentication bug in login flow");
    repo.add_spec("Add payment processing for checkout");
    repo.add_spec("Update user credentials validation");

    // Semantic search finds related concepts
    let results = repo.search("--semantic 'security credentials'");
    assert!(results.len() >= 2); // Should find auth and credentials specs
}

#[test]
#[ignore] // Requires fastembed-rs models downloaded
fn test_phase6_5_hybrid_search() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        search:
          semantic: true
          hybrid: true
    "#);

    repo.add_spec("Implement OAuth2 authentication");
    repo.add_spec("Add SSO login support");

    // Hybrid combines keyword + semantic
    let results = repo.search("--hybrid 'user login'");
    assert!(results.len() == 2);
}

#[test]
#[ignore] // Requires fastembed-rs models downloaded
fn test_phase6_5_similar() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        search:
          semantic: true
    "#);

    let spec1 = repo.add_spec("Add rate limiting to API endpoints");
    repo.add_spec("Implement request throttling");
    repo.add_spec("Fix database connection pool");

    // Find similar specs
    let similar = repo.run(&format!("chant similar {}", spec1)).success();
    assert!(similar.stdout.contains("throttling")); // Semantically similar
    assert!(!similar.stdout.contains("database")); // Not related
}
```

### Phase 7 Gate: Autonomy

```rust
#[test]
fn test_phase7_drift_detection() {
    let repo = TempRepo::new();
    repo.init();

    // Create and complete a spec
    let spec = repo.add_spec("Add config.json with version=1.0");
    repo.work(&spec);

    // Manually modify the file (simulate drift)
    repo.write("config.json", r#"{"version": "2.0"}"#);

    // Detect drift
    let result = repo.run(&format!("chant drift {}", spec));
    assert!(result.stdout.contains("drift detected"));
    assert!(result.stdout.contains("config.json"));
}

#[test]
fn test_phase7_replay() {
    let repo = TempRepo::new();
    repo.init();

    // Create and complete a spec
    let spec = repo.add_spec("Add hello.txt with 'Hello'");
    repo.work(&spec);
    assert!(repo.file_contains("hello.txt", "Hello"));

    // Delete the file (simulate drift)
    repo.delete("hello.txt");
    assert!(!repo.exists("hello.txt"));

    // Replay should restore
    repo.run(&format!("chant replay {}", spec)).success();
    assert!(repo.file_contains("hello.txt", "Hello"));
}

#[test]
fn test_phase7_verification() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        autonomy:
          verify_schedule: "0 * * * *"  # Every hour
    "#);

    let spec = repo.add_spec("Add validated.txt");
    repo.work(&spec);

    // Run verification
    let result = repo.run(&format!("chant verify {}", spec));
    assert!(result.stdout.contains("verified"));
}
```

### Phase 8 Gate: Polish

```rust
#[test]
fn test_phase8_alternate_provider() {
    // Skip if no API key
    if std::env::var("PROVIDER_API_KEY").is_err() {
        return;
    }

    let repo = TempRepo::new();
    repo.init_with_config(r#"
        agent:
          provider: alternate
          model: model-name
    "#);

    let spec = repo.add_spec("Add test.txt with 'provider test'");
    repo.work(&spec);

    assert!(repo.exists("test.txt"));
}

#[test]
fn test_phase8_prompt_install() {
    let repo = TempRepo::new();
    repo.init();

    // Install prompt from URL
    repo.run("chant prompt install https://example.com/prompts/tdd.md")
        .success();

    // Verify installed
    assert!(repo.exists(".chant/prompts/tdd.md"));

    // Can use it
    let spec = repo.add_spec("TDD spec");
    repo.run(&format!("chant work {} --prompt tdd", spec)).success();
}

#[test]
fn test_phase8_scm_github() {
    let repo = TempRepo::new_with_remote();
    repo.init_with_config(r#"
        scm:
          provider: github
          repo: owner/repo
    "#);

    let spec = repo.add_spec("GitHub integration test");

    // Sync to GitHub
    repo.run(&format!("chant sync {}", spec)).success();

    // Verify issue created (mock)
    assert!(repo.mock_github.issue_created());
}

#[test]
fn test_phase8_approval_workflow() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        approval:
          required: true
          approvers: ["alice", "bob"]
    "#);

    let spec = repo.add_spec("Needs approval");
    repo.work(&spec);

    // Spec is pending_approval, not completed
    let spec = repo.read_spec(&spec);
    assert_eq!(spec.status, "pending_approval");

    // Approve
    repo.run(&format!("chant approve {} --as alice", spec)).success();

    // Now completed
    let spec = repo.read_spec(&spec);
    assert_eq!(spec.status, "completed");
}

#[test]
fn test_phase8_notifications() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        notifications:
          webhook:
            url: "http://localhost:9999/webhook"
            events: [completed, failed]
    "#);

    // Start mock webhook server
    let server = MockWebhookServer::start(9999);

    let spec = repo.add_spec("Notification test");
    repo.work(&spec);

    // Webhook received notification
    let events = server.received_events();
    assert!(events.iter().any(|e| e.event_type == "completed"));
    assert!(events.iter().any(|e| e.spec_id == spec));
}

#[test]
fn test_phase8_templates() {
    let repo = TempRepo::new();
    repo.init();

    // Install template
    repo.write(".chant/templates/bug.md", r#"---
labels: [bug]
---

# Bug: {{title}}

## Steps to Reproduce
1.

## Expected Behavior

## Actual Behavior
"#);

    // Create from template
    let result = repo.run("chant add --template bug 'Login fails'").success();
    let spec_id = result.spec_id();

    // Verify template applied
    let spec = repo.read_spec(&spec_id);
    assert!(spec.labels.contains(&"bug".to_string()));
    assert!(spec.body.contains("Steps to Reproduce"));
}

#[test]
fn test_phase8_git_hooks() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        hooks:
          manager: native
          pre_commit:
            enabled: true
            lint: true
    "#);

    // Generate hooks
    repo.run("chant hooks generate --manager native").success();

    // Verify hook installed
    assert!(repo.exists(".git/hooks/pre-commit"));

    // Create invalid spec
    repo.add_spec_raw(r#"---
status: pending
---

# Vague spec

No criteria.
"#);

    // Commit should warn (pre-commit lint)
    let result = repo.run("git add . && git commit -m 'test'");
    assert!(result.stdout.contains("missing acceptance criteria"));
}
```

### Phase Gates Summary

| Phase | Gate Test | Must Pass |
|-------|-----------|-----------|
| 0 → 1 | `test_phase0_self_hosting` | Chant executes spec on itself |
| 1 → 2 | `test_phase1_branching` | Branches created, PRs opened |
| 1 → 2 | `test_phase1_isolation` | Clone isolation works |
| 1 → 2 | `test_phase1_post_work_hooks` | Hooks execute after work |
| 2 → 3 | `test_phase2_mcp_server` | MCP tools work |
| 2 → 3 | `test_phase2_kiro_provider` | Kiro config generated |
| 3 → 4 | `test_phase3_multi_repo` | Cross-repo deps resolve |
| 4 → 5 | `test_phase4_labels` | Label filtering works |
| 4 → 5 | `test_phase4_triggers` | Triggers fire on events |
| 4 → 5 | `test_phase4_spec_types` | Type filtering works |
| 4 → 5 | `test_phase4_groups` | Groups split/combine |
| 5 → 6 | `test_phase5_cost_tracking` | Costs recorded per spec |
| 5 → 6 | `test_phase5_linting` | Linter warns on issues |
| 5 → 6 | `test_phase5_error_catalog` | Errors have catalog codes |
| 5 → 6 | `test_phase5_reports` | Reports generated correctly |
| 6 → 7 | `test_phase6_tantivy_search` | Search returns results |
| 6 → 7 | `test_phase6_locks` | Locks prevent conflicts |
| 6 → 7 | `test_phase6_queue` | Queue processes specs |
| 6 → 7 | `test_phase6_pools` | Pools respect limits |
| 6 → 7 | `test_daemon_mode` | Daemon indexes and serves |
| 6.5   | `test_phase6_5_semantic_search` | Semantic search works (optional) |
| 7 → 8 | `test_phase7_drift_detection` | Drift detected |
| 7 → 8 | `test_phase7_replay` | Replay restores state |
| 8 → ✓ | `test_phase8_approval_workflow` | Approvals block completion |
| 8 → ✓ | `test_phase8_notifications` | Webhooks fire on events |
| 8 → ✓ | `test_phase8_templates` | Templates expand correctly |
| 8 → ✓ | `test_phase8_git_hooks` | Git hooks integrate |

## Test Hierarchy

```
Integration Tests (high-level, design first)
    ↓
Component Tests (boundaries)
    ↓
Unit Tests (implementation details)
```

## Integration Tests

End-to-end tests that exercise complete workflows.

### Core Workflow

```rust
#[test]
fn test_basic_workflow() {
    let repo = TempRepo::new();

    // Init
    repo.run("chant init").success();
    assert!(repo.exists(".chant/config.md"));

    // Add spec
    let output = repo.run("chant add 'Fix bug'").success();
    let spec_id = output.spec_id();

    // Verify spec created
    repo.run(&format!("chant show {}", spec_id)).success();

    // Execute (with mock agent)
    repo.run(&format!("chant work {} --agent mock", spec_id))
        .success();

    // Verify completed
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

    // Create driver with group members
    let driver = repo.add_spec("Driver spec");
    repo.split(&driver); // Creates members

    // Execute in parallel
    repo.run(&format!(
        "chant work {} --parallel --max 3",
        driver
    )).success();

    // All members completed
    for member in repo.members(&driver) {
        assert_eq!(member.status, "completed");
    }

    // Driver auto-completed
    let driver = repo.read_spec(&driver);
    assert_eq!(driver.status, "completed");
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

    // C is blocked
    assert!(repo.is_blocked(&spec_c));

    // Work on A
    repo.work(&spec_a);

    // B now ready, C still blocked
    assert!(repo.is_ready(&spec_b));
    assert!(repo.is_blocked(&spec_c));

    // Complete chain
    repo.work(&spec_b);
    repo.work(&spec_c);

    // All done
    assert_eq!(repo.read_spec(&spec_c).status, "completed");
}
```

### Worktree Isolation

```rust
#[test]
fn test_worktree_isolation() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Isolated work");

    // Start work (creates worktree)
    let handle = repo.work_async(&spec);

    // Verify worktree exists
    assert!(repo.exists(&format!(".chant/.worktrees/{}", spec)));

    // Complete
    handle.wait();

    // Worktree cleaned up
    assert!(!repo.exists(&format!(".chant/.worktrees/{}", spec)));
}
```

### Lock Contention

```rust
#[test]
fn test_lock_contention() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Contested spec");

    // First agent acquires lock
    let handle1 = repo.work_async(&spec);
    thread::sleep(Duration::from_millis(100));

    // Second agent blocked
    let result = repo.run(&format!("chant work {}", spec));
    assert!(result.stderr.contains("locked"));

    handle1.wait();
}
```

### Daemon Mode

```rust
#[test]
fn test_daemon_mode() {
    let repo = TempRepo::new();
    repo.init_with_config(r#"
        scale:
          daemon:
            enabled: true
    "#);

    // Start daemon
    let daemon = repo.start_daemon();

    // Operations use daemon
    let spec = repo.add_spec("Daemon spec");
    repo.work(&spec);

    // Check metrics
    let metrics = daemon.metrics();
    assert_eq!(metrics.specs_completed, 1);

    daemon.stop();
}
```

### Error Recovery

```rust
#[test]
fn test_crash_recovery() {
    let repo = TempRepo::new();
    repo.init();

    let spec = repo.add_spec("Crash test");

    // Simulate crash (create stale lock)
    repo.create_stale_lock(&spec, 99999);

    // Verify locked
    assert!(repo.is_locked(&spec));

    // Force unlock
    repo.run(&format!("chant unlock {}", spec)).success();

    // Can work now
    repo.work(&spec);
}
```

### Multi-Repo Workflows

```rust
#[test]
fn test_multi_repo_list() {
    let repo1 = TempRepo::new();
    let repo2 = TempRepo::new();
    repo1.init();
    repo2.init();

    let global = GlobalConfig::temp();
    global.add_repo("frontend", &repo1.path);
    global.add_repo("backend", &repo2.path);

    repo1.add_spec("Frontend spec");
    repo2.add_spec("Backend spec");

    // List all
    let output = global.run("chant list --global").success();
    assert!(output.contains("frontend:"));
    assert!(output.contains("backend:"));
}

#[test]
fn test_cross_repo_dependency() {
    let shared = TempRepo::new();
    let app = TempRepo::new();
    shared.init();
    app.init();

    let global = GlobalConfig::temp();
    global.add_repo("shared", &shared.path);
    global.add_repo("app", &app.path);

    // Shared types spec
    let types_spec = shared.add_spec("Add User type");

    // App depends on shared
    let app_spec = app.add_spec_with_frontmatter(&format!(r#"
---
status: pending
depends_on:
  - shared:{}
---

# Use User type

Import and use User type from shared.
"#, types_spec));

    // App is blocked
    assert!(app.is_blocked(&app_spec));

    // Complete shared
    shared.work(&types_spec);

    // App now ready
    assert!(app.is_ready(&app_spec));
}
```

### MCP Server

```rust
#[test]
fn test_mcp_spec_get() {
    let repo = TempRepo::new();
    repo.init();

    let spec_id = repo.add_spec("Test spec");

    let mcp = ChantMcpServer::start_in(&repo.path);

    let result = mcp.call_tool("chant_spec_get", json!({
        "id": spec_id
    }));

    assert!(result.is_ok());
    let spec: Spec = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(spec.status, "pending");
}

#[test]
fn test_mcp_spec_update() {
    let repo = TempRepo::new();
    repo.init();

    let spec_id = repo.add_spec("Test spec");

    let mcp = ChantMcpServer::start_in(&repo.path);

    // Update via MCP
    mcp.call_tool("chant_spec_update", json!({
        "id": spec_id,
        "status": "in_progress"
    })).unwrap();

    // Verify
    let spec = repo.read_spec(&spec_id);
    assert_eq!(spec.status, "in_progress");
}
```

### Kiro Integration

```rust
#[test]
#[ignore] // Requires kiro-cli installed
fn test_kiro_config_generation() {
    let repo = TempRepo::new();
    repo.init();

    let spec_id = repo.add_spec("Kiro test");

    // Prepare for Kiro (generates config files)
    repo.run(&format!(
        "chant work {} --provider kiro --dry-run",
        spec_id
    )).success();

    // Verify config files
    assert!(repo.exists(".kiro/mcp.json"));

    let mcp_config: serde_json::Value =
        serde_json::from_str(&repo.read(".kiro/mcp.json")).unwrap();

    assert!(mcp_config["mcpServers"]["chant"].is_object());
}

#[test]
#[ignore] // Requires kiro-cli installed and authenticated
fn test_kiro_execution() {
    let repo = TempRepo::new();
    repo.init();

    let spec_id = repo.add_spec("Add hello.txt with 'Hello from Kiro'");

    repo.run(&format!(
        "chant work {} --provider kiro",
        spec_id
    )).success();

    // Verify work done
    assert!(repo.exists("hello.txt"));
    assert!(repo.file_contains("hello.txt", "Hello from Kiro"));
}
```

## Component Tests

Test boundaries between components.

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
    assert!(spec.body.contains("Safari"));
}
```

### Search Index Tests

```rust
#[test]
fn test_search_index() {
    let index = TantivyIndex::new_temp();

    index.add(Spec {
        id: "2026-01-22-001-x7m",
        status: "pending",
        body: "Fix authentication bug",
    });

    // Field search
    let results = index.search("status:pending");
    assert_eq!(results.len(), 1);

    // Full-text search
    let results = index.search("authentication");
    assert_eq!(results.len(), 1);

    // No match
    let results = index.search("payment");
    assert_eq!(results.len(), 0);
}
```

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
        if self.should_succeed {
            Ok(())
        } else {
            Err(Error::AgentFailed("Mock failure"))
        }
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

      # Unit tests
      - run: cargo test --lib

      # Integration tests
      - run: cargo test --test integration

      # Slow tests (parallel, daemon)
      - run: cargo test --test slow -- --test-threads=1
```

## Coverage Goals

| Component | Target |
|-----------|--------|
| Core workflow | 90% |
| Parser (spec + ID) | 95% |
| MCP server | 85% |
| Multi-repo | 80% |
| Search | 80% |
| CLI | 70% |
| Daemon | 75% |
| Providers | 70% |

## Test Categories

```bash
# All tests
cargo test

# Fast only (unit + component)
cargo test --lib

# Integration only
cargo test --test integration

# Phase gate tests
cargo test --test gates

# Multi-repo tests
cargo test --test multi_repo

# MCP server tests
cargo test --test mcp

# With real agent (slow, costs money)
cargo test --test real_agent --ignored

# With Kiro (requires kiro-cli)
cargo test --test kiro --ignored

# Bootstrap test (modifies real repo, careful!)
cargo test --test bootstrap --ignored
```
