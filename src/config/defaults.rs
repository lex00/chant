//! Default values and configuration structs with default implementations.

use serde::{Deserialize, Serialize};

use crate::provider::ProviderType;

/// Macro to generate default functions for serde attributes
macro_rules! default_fn {
    ($name:ident, $type:ty, $value:expr) => {
        pub(crate) fn $name() -> $type {
            $value
        }
    };
}

// =========================================================================
// DEFAULT VALUE FUNCTIONS
// =========================================================================

default_fn!(default_complexity_criteria, usize, 10);
default_fn!(default_complexity_files, usize, 5);
default_fn!(default_complexity_words, usize, 150);
default_fn!(default_simple_criteria, usize, 1);
default_fn!(default_simple_files, usize, 1);
default_fn!(default_simple_words, usize, 3);
default_fn!(default_max_retries, usize, 3);
default_fn!(default_retry_delay_ms, u64, 60_000); // 60 seconds
default_fn!(default_backoff_multiplier, f64, 2.0);
default_fn!(default_poll_interval_ms, u64, 5000); // 5 seconds
default_fn!(default_site_output_dir, String, "./public/".to_string());
default_fn!(default_site_base_url, String, "/".to_string());
default_fn!(default_site_title, String, "Project Specs".to_string());
default_fn!(
    default_include_statuses,
    Vec<String>,
    vec![
        "completed".to_string(),
        "in_progress".to_string(),
        "pending".to_string(),
    ]
);
default_fn!(default_true, bool, true);
default_fn!(default_agent_weight, usize, 1);
default_fn!(default_agent_name, String, "main".to_string());
default_fn!(default_agent_command, String, "claude".to_string());
default_fn!(default_max_concurrent, usize, 2);
default_fn!(default_stagger_delay_ms, u64, 1000); // Default 1 second between agent spawns
default_fn!(default_stagger_jitter_ms, u64, 200); // Default 20% of stagger_delay_ms (200ms is 20% of 1000ms)
default_fn!(default_rotation_strategy, String, "none".to_string());
default_fn!(default_prompt, String, "bootstrap".to_string());
default_fn!(default_branch_prefix, String, "chant/".to_string());
default_fn!(default_main_branch, String, "main".to_string());

// =========================================================================
// CONFIG STRUCTS WITH DEFAULTS
// =========================================================================

/// Thresholds for linter complexity heuristics
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LintThresholds {
    /// Max acceptance criteria for complex specs (default: 10)
    #[serde(default = "default_complexity_criteria")]
    pub complexity_criteria: usize,
    /// Max target files for complex specs (default: 5)
    #[serde(default = "default_complexity_files")]
    pub complexity_files: usize,
    /// Max words in title for complex specs (default: 150)
    #[serde(default = "default_complexity_words")]
    pub complexity_words: usize,
    /// Min acceptance criteria for simple specs (default: 1)
    #[serde(default = "default_simple_criteria")]
    pub simple_criteria: usize,
    /// Min target files for simple specs (default: 1)
    #[serde(default = "default_simple_files")]
    pub simple_files: usize,
    /// Min words in title for simple specs (default: 3)
    #[serde(default = "default_simple_words")]
    pub simple_words: usize,
}

impl Default for LintThresholds {
    fn default() -> Self {
        Self {
            complexity_criteria: default_complexity_criteria(),
            complexity_files: default_complexity_files(),
            complexity_words: default_complexity_words(),
            simple_criteria: default_simple_criteria(),
            simple_files: default_simple_files(),
            simple_words: default_simple_words(),
        }
    }
}

/// Linter configuration for spec validation
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LintConfig {
    /// Thresholds for complexity heuristics
    #[serde(default)]
    pub thresholds: LintThresholds,
    /// List of rule names to disable (skip during linting)
    #[serde(default)]
    pub disable: Vec<String>,
}

/// Failure handling strategy for permanent failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnPermanentFailure {
    /// Skip the failed spec and continue watching others
    #[default]
    Skip,
    /// Stop the watch command entirely
    Stop,
}

/// Configuration for failure handling in watch command
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FailureConfig {
    /// Maximum number of retry attempts for transient errors
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
    /// Delay in milliseconds before first retry
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
    /// Multiplier for exponential backoff (must be >= 1.0)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// Regex patterns for errors that should be retried
    #[serde(default)]
    pub retryable_patterns: Vec<String>,
    /// Action to take on permanent failure
    #[serde(default)]
    pub on_permanent_failure: OnPermanentFailure,
}

impl Default for FailureConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
            retryable_patterns: vec![],
            on_permanent_failure: OnPermanentFailure::default(),
        }
    }
}

/// Configuration for watch command behavior
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WatchConfig {
    /// Poll interval in milliseconds
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Failure handling configuration
    #[serde(default)]
    pub failure: FailureConfig,
    /// Idle timeout in minutes (default: 5)
    #[serde(default = "default_idle_timeout_minutes")]
    pub idle_timeout_minutes: u64,
}

fn default_idle_timeout_minutes() -> u64 {
    5
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: default_poll_interval_ms(),
            failure: FailureConfig::default(),
            idle_timeout_minutes: default_idle_timeout_minutes(),
        }
    }
}

/// Configuration for what specs to include in the site
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteIncludeConfig {
    /// Statuses to include (default: completed, in_progress, pending)
    #[serde(default = "default_include_statuses")]
    pub statuses: Vec<String>,
    /// Labels to include (empty = all)
    #[serde(default)]
    pub labels: Vec<String>,
}

impl Default for SiteIncludeConfig {
    fn default() -> Self {
        Self {
            statuses: default_include_statuses(),
            labels: vec![],
        }
    }
}

/// Configuration for what to exclude from the site
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SiteExcludeConfig {
    /// Labels to exclude from output
    #[serde(default)]
    pub labels: Vec<String>,
    /// Fields to redact from output (e.g., cost_usd, tokens)
    #[serde(default)]
    pub fields: Vec<String>,
}

/// Feature toggles for site pages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteFeaturesConfig {
    /// Generate changelog page
    #[serde(default = "default_true")]
    pub changelog: bool,
    /// Generate dependency graph page
    #[serde(default = "default_true")]
    pub dependency_graph: bool,
    /// Generate timeline page
    #[serde(default = "default_true")]
    pub timeline: bool,
    /// Generate status index pages
    #[serde(default = "default_true")]
    pub status_indexes: bool,
    /// Generate label index pages
    #[serde(default = "default_true")]
    pub label_indexes: bool,
}

impl Default for SiteFeaturesConfig {
    fn default() -> Self {
        Self {
            changelog: true,
            dependency_graph: true,
            timeline: true,
            status_indexes: true,
            label_indexes: true,
        }
    }
}

/// Graph detail level for dependency visualization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphDetailLevel {
    /// Show only spec IDs
    Minimal,
    /// Show IDs and titles
    Titles,
    /// Show IDs, titles, status, and labels
    #[default]
    Full,
}

/// Configuration for dependency graph visualization
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteGraphConfig {
    /// Level of detail in the graph
    #[serde(default)]
    pub detail: GraphDetailLevel,
}

impl Default for SiteGraphConfig {
    fn default() -> Self {
        Self {
            detail: GraphDetailLevel::Full,
        }
    }
}

/// Timeline grouping option
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelineGroupBy {
    /// Group by day
    #[default]
    Day,
    /// Group by week
    Week,
    /// Group by month
    Month,
}

/// Configuration for timeline visualization
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteTimelineConfig {
    /// How to group timeline entries
    #[serde(default)]
    pub group_by: TimelineGroupBy,
    /// Whether to include pending specs in timeline
    #[serde(default)]
    pub include_pending: bool,
}

impl Default for SiteTimelineConfig {
    fn default() -> Self {
        Self {
            group_by: TimelineGroupBy::Day,
            include_pending: false,
        }
    }
}

/// Configuration for static site generation
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SiteConfig {
    /// Output directory for generated site (default: ./public/)
    #[serde(default = "default_site_output_dir")]
    pub output_dir: String,
    /// Base URL for the site (default: /)
    #[serde(default = "default_site_base_url")]
    pub base_url: String,
    /// Site title (default: "Project Specs")
    #[serde(default = "default_site_title")]
    pub title: String,
    /// Content filtering - what to include
    #[serde(default)]
    pub include: SiteIncludeConfig,
    /// Content filtering - what to exclude
    #[serde(default)]
    pub exclude: SiteExcludeConfig,
    /// Feature toggles for different page types
    #[serde(default)]
    pub features: SiteFeaturesConfig,
    /// Graph visualization options
    #[serde(default)]
    pub graph: SiteGraphConfig,
    /// Timeline visualization options
    #[serde(default)]
    pub timeline: SiteTimelineConfig,
}

/// Configuration for a single agent (Claude account/command)
#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    /// Name of the agent (for display and attribution)
    #[serde(default = "default_agent_name")]
    pub name: String,
    /// Shell command to invoke this agent (e.g., "claude", "claude-alt1")
    #[serde(default = "default_agent_command")]
    pub command: String,
    /// Maximum concurrent instances for this agent
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
    /// Weight for agent rotation selection (higher = more likely to be selected)
    #[serde(default = "default_agent_weight")]
    pub weight: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: default_agent_name(),
            command: default_agent_command(),
            max_concurrent: default_max_concurrent(),
            weight: default_agent_weight(),
        }
    }
}

/// Configuration for parallel execution with multiple agents
#[derive(Debug, Deserialize, Clone)]
pub struct ParallelConfig {
    /// List of available agents (Claude accounts/commands)
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    /// Delay in milliseconds between spawning each agent to avoid API rate limiting
    #[serde(default = "default_stagger_delay_ms")]
    pub stagger_delay_ms: u64,
    /// Jitter in milliseconds for spawn delays (default: 20% of stagger_delay_ms)
    #[serde(default = "default_stagger_jitter_ms")]
    pub stagger_jitter_ms: u64,
}

impl ParallelConfig {
    /// Calculate total capacity as sum of all agent max_concurrent values
    pub fn total_capacity(&self) -> usize {
        self.agents.iter().map(|a| a.max_concurrent).sum()
    }
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            agents: vec![AgentConfig::default()],
            stagger_delay_ms: default_stagger_delay_ms(),
            stagger_jitter_ms: default_stagger_jitter_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
    /// Default model name to use when env vars are not set
    #[serde(default)]
    pub model: Option<String>,
    /// Default model name for split operations (defaults to sonnet)
    #[serde(default)]
    pub split_model: Option<String>,
    /// Default main branch name for merges (defaults to "main")
    #[serde(default = "default_main_branch")]
    pub main_branch: String,
    /// Default provider (claude, ollama, openai)
    #[serde(default)]
    pub provider: ProviderType,
    /// Agent rotation strategy for single spec execution (none, random, round-robin)
    #[serde(default = "default_rotation_strategy")]
    pub rotation_strategy: String,
    /// List of prompt extensions to append to all prompts
    #[serde(default)]
    pub prompt_extensions: Vec<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            prompt: default_prompt(),
            branch_prefix: default_branch_prefix(),
            model: None,
            split_model: None,
            main_branch: default_main_branch(),
            provider: ProviderType::Claude,
            rotation_strategy: default_rotation_strategy(),
            prompt_extensions: vec![],
        }
    }
}
