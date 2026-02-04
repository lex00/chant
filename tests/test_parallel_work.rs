//! Tests for parallel work execution
//!
//! Tests the parallel worker pool lifecycle, spec distribution, and failure scenarios.

use chant::config::AgentConfig;
use chant::spec::{Spec, SpecStatus};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test environment with specs directory
fn setup_test_env() -> (TempDir, PathBuf, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let specs_dir = base_path.join(".chant/specs");
    let prompts_dir = base_path.join(".chant/prompts");
    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&prompts_dir).unwrap();

    // Create a standard prompt file
    let prompt_content = "You are implementing a task for chant.";
    fs::write(prompts_dir.join("standard.md"), prompt_content).unwrap();

    // Set cwd to temp directory
    std::env::set_current_dir(base_path).unwrap();

    (temp_dir, specs_dir, prompts_dir)
}

/// Helper to create a test spec file
fn create_spec(specs_dir: &std::path::Path, id: &str, status: &str, body: &str) {
    let content = format!(
        r#"---
type: code
status: {}
---

{}
"#,
        status, body
    );
    let spec_path = specs_dir.join(format!("{}.md", id));
    fs::write(&spec_path, &content).unwrap();

    // Load and re-save to ensure proper parsing and id field
    let mut spec = Spec::parse(id, &content).unwrap();
    spec.id = id.to_string();
    spec.save(&spec_path).unwrap();
}

/// Helper to create agent configs for testing
fn create_test_agents(num_agents: usize, max_per_agent: usize) -> Vec<AgentConfig> {
    if num_agents == 0 {
        vec![]
    } else {
        (0..num_agents)
            .map(|i| AgentConfig {
                name: format!("agent-{}", i),
                command: "echo".to_string(), // Use echo as a dummy command
                max_concurrent: max_per_agent,
                weight: 1,
            })
            .collect()
    }
}

/// Helper to calculate total capacity from agents
fn calculate_total_capacity(agents: &[AgentConfig]) -> usize {
    agents.iter().map(|a| a.max_concurrent).sum()
}

/// Simulate spec distribution logic for testing
fn distribute_specs_to_agents_simple(
    specs: &[Spec],
    agents: &[AgentConfig],
    max_override: Option<usize>,
) -> Vec<(String, String)> {
    let total_max = max_override.unwrap_or_else(|| calculate_total_capacity(agents));
    let mut agent_allocations: Vec<usize> = vec![0; agents.len()];
    let mut assignments = Vec::new();

    for spec in specs {
        if assignments.len() >= total_max {
            break;
        }

        // Find agent with most remaining capacity
        let mut best_agent_idx = None;
        let mut best_remaining_capacity = 0;

        for (idx, agent) in agents.iter().enumerate() {
            let remaining = agent.max_concurrent.saturating_sub(agent_allocations[idx]);
            if remaining > best_remaining_capacity {
                best_remaining_capacity = remaining;
                best_agent_idx = Some(idx);
            }
        }

        if let Some(idx) = best_agent_idx {
            agent_allocations[idx] += 1;
            assignments.push((spec.id.clone(), agents[idx].name.clone()));
        }
    }

    assignments
}

// ============================================================================
// Worker Pool Creation Tests
// ============================================================================

#[test]
#[serial]
fn test_worker_pool_single_agent() {
    let agents = create_test_agents(1, 1);
    assert_eq!(calculate_total_capacity(&agents), 1);
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "agent-0");
    assert_eq!(agents[0].max_concurrent, 1);
}

#[test]
#[serial]
fn test_worker_pool_two_agents() {
    let agents = create_test_agents(2, 1);
    assert_eq!(calculate_total_capacity(&agents), 2);
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].name, "agent-0");
    assert_eq!(agents[1].name, "agent-1");
}

#[test]
#[serial]
fn test_worker_pool_n_agents() {
    let agents = create_test_agents(5, 2);
    assert_eq!(calculate_total_capacity(&agents), 10);
    assert_eq!(agents.len(), 5);
}

#[test]
#[serial]
fn test_worker_pool_default_agent() {
    let agents = create_test_agents(0, 0);
    assert_eq!(agents.len(), 0);
}

// ============================================================================
// Spec Distribution Tests
// ============================================================================

#[test]
#[serial]
fn test_spec_distribution_single_worker() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-03-001-abc", "pending", "# Spec 1");
    create_spec(&specs_dir, "2026-02-03-002-def", "pending", "# Spec 2");

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(1, 2);

    let assignments = distribute_specs_to_agents_simple(&specs, &agents, None);

    // Should distribute both specs to single agent
    assert_eq!(assignments.len(), 2);
    assert_eq!(assignments[0].1, "agent-0");
    assert_eq!(assignments[1].1, "agent-0");
}

#[test]
#[serial]
fn test_spec_distribution_multiple_workers() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    for i in 1..=4 {
        create_spec(
            &specs_dir,
            &format!("2026-02-03-00{}-abc", i),
            "pending",
            &format!("# Spec {}", i),
        );
    }

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(2, 2);

    let assignments = distribute_specs_to_agents_simple(&specs, &agents, None);

    // Should distribute 4 specs across 2 agents
    assert_eq!(assignments.len(), 4);

    // Count specs per agent
    let agent0_count = assignments.iter().filter(|a| a.1 == "agent-0").count();
    let agent1_count = assignments.iter().filter(|a| a.1 == "agent-1").count();

    // Should be balanced (2 each)
    assert_eq!(agent0_count, 2);
    assert_eq!(agent1_count, 2);
}

#[test]
#[serial]
fn test_spec_distribution_respects_max_override() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    for i in 1..=5 {
        create_spec(
            &specs_dir,
            &format!("2026-02-03-00{}-abc", i),
            "pending",
            &format!("# Spec {}", i),
        );
    }

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(2, 3);

    // Override max to 3
    let assignments = distribute_specs_to_agents_simple(&specs, &agents, Some(3));

    // Should only distribute 3 specs despite having capacity for 6
    assert_eq!(assignments.len(), 3);
}

#[test]
#[serial]
fn test_spec_distribution_respects_per_agent_limit() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    for i in 1..=5 {
        create_spec(
            &specs_dir,
            &format!("2026-02-03-00{}-abc", i),
            "pending",
            &format!("# Spec {}", i),
        );
    }

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(2, 2);

    let assignments = distribute_specs_to_agents_simple(&specs, &agents, None);

    // Should only distribute 4 specs (2 per agent * 2 agents)
    assert_eq!(assignments.len(), 4);

    // Verify no agent exceeds max
    let agent0_count = assignments.iter().filter(|a| a.1 == "agent-0").count();
    let agent1_count = assignments.iter().filter(|a| a.1 == "agent-1").count();

    assert!(agent0_count <= 2);
    assert!(agent1_count <= 2);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
#[serial]
fn test_empty_spec_list() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(2, 2);

    let assignments = distribute_specs_to_agents_simple(&specs, &agents, None);

    assert_eq!(assignments.len(), 0);
}

#[test]
#[serial]
fn test_single_spec_parallel_mode() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-03-001-abc", "pending", "# Single spec");

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(3, 2);

    let assignments = distribute_specs_to_agents_simple(&specs, &agents, None);

    // Should distribute single spec to first agent
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].1, "agent-0");
}

#[test]
#[serial]
fn test_more_workers_than_specs() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-03-001-abc", "pending", "# Spec 1");
    create_spec(&specs_dir, "2026-02-03-002-def", "pending", "# Spec 2");

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let agents = create_test_agents(5, 1);

    let assignments = distribute_specs_to_agents_simple(&specs, &agents, None);

    // Should distribute 2 specs across available agents
    assert_eq!(assignments.len(), 2);

    // Each spec should go to different agents (least-loaded-first)
    assert_ne!(assignments[0].1, assignments[1].1);
}

// ============================================================================
// Spec Status Tests
// ============================================================================

#[test]
#[serial]
fn test_spec_status_transitions_to_in_progress() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-03-001-abc", "pending", "# Test spec");

    // Load and update status to in_progress
    let mut spec = chant::spec::resolve_spec(&specs_dir, "2026-02-03-001-abc").unwrap();
    spec.frontmatter.status = SpecStatus::InProgress;
    let spec_path = specs_dir.join("2026-02-03-001-abc.md");
    spec.save(&spec_path).unwrap();

    // Verify status was updated
    let reloaded = chant::spec::resolve_spec(&specs_dir, "2026-02-03-001-abc").unwrap();
    assert_eq!(reloaded.frontmatter.status, SpecStatus::InProgress);
}

#[test]
#[serial]
fn test_spec_status_transitions_to_completed() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-03-001-abc",
        "in_progress",
        "# Test spec",
    );

    // Update status to completed
    let mut spec = chant::spec::resolve_spec(&specs_dir, "2026-02-03-001-abc").unwrap();
    spec.frontmatter.status = SpecStatus::Completed;
    let spec_path = specs_dir.join("2026-02-03-001-abc.md");
    spec.save(&spec_path).unwrap();

    // Verify status was updated
    let reloaded = chant::spec::resolve_spec(&specs_dir, "2026-02-03-001-abc").unwrap();
    assert_eq!(reloaded.frontmatter.status, SpecStatus::Completed);
}

#[test]
#[serial]
fn test_spec_status_transitions_to_failed() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(
        &specs_dir,
        "2026-02-03-001-abc",
        "in_progress",
        "# Failed spec",
    );

    // Update status to failed
    let mut spec = chant::spec::resolve_spec(&specs_dir, "2026-02-03-001-abc").unwrap();
    spec.frontmatter.status = SpecStatus::Failed;
    let spec_path = specs_dir.join("2026-02-03-001-abc.md");
    spec.save(&spec_path).unwrap();

    // Verify status was updated
    let reloaded = chant::spec::resolve_spec(&specs_dir, "2026-02-03-001-abc").unwrap();
    assert_eq!(reloaded.frontmatter.status, SpecStatus::Failed);
}

// ============================================================================
// Agent Configuration Tests
// ============================================================================

#[test]
fn test_agent_config_creation() {
    let agent = AgentConfig {
        name: "test-agent".to_string(),
        command: "claude".to_string(),
        max_concurrent: 3,
        weight: 1,
    };

    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.command, "claude");
    assert_eq!(agent.max_concurrent, 3);
    assert_eq!(agent.weight, 1);
}

#[test]
fn test_agent_config_default() {
    let agent = AgentConfig::default();

    assert!(!agent.name.is_empty());
    assert!(!agent.command.is_empty());
    assert!(agent.max_concurrent > 0);
    assert!(agent.weight > 0);
}

// ============================================================================
// Multiple Spec Readiness Tests
// ============================================================================

#[test]
#[serial]
fn test_multiple_ready_specs() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-03-001-abc", "pending", "# Ready 1");
    create_spec(&specs_dir, "2026-02-03-002-def", "pending", "# Ready 2");
    create_spec(&specs_dir, "2026-02-03-003-ghi", "pending", "# Ready 3");

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let ready_specs: Vec<_> = specs.iter().filter(|s| s.is_ready(&specs)).collect();

    assert_eq!(ready_specs.len(), 3);
}

#[test]
#[serial]
fn test_filtering_completed_specs() {
    let (_temp, specs_dir, _prompts_dir) = setup_test_env();

    create_spec(&specs_dir, "2026-02-03-001-abc", "pending", "# Ready");
    create_spec(&specs_dir, "2026-02-03-002-def", "completed", "# Done");
    create_spec(&specs_dir, "2026-02-03-003-ghi", "in_progress", "# Working");

    let specs = chant::spec::load_all_specs(&specs_dir).unwrap();
    let ready_specs: Vec<_> = specs.iter().filter(|s| s.is_ready(&specs)).collect();

    // Only pending spec should be ready
    assert_eq!(ready_specs.len(), 1);
    assert_eq!(ready_specs[0].id, "2026-02-03-001-abc");
}
