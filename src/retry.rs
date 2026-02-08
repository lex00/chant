//! Retry logic with exponential backoff for failed specs.
//!
//! Provides retry state tracking and decision logic for determining whether
//! a failed spec should be retried or marked as permanently failed.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::FailureConfig;

/// Maximum retry delay capped at 1 hour to prevent overflow
const MAX_RETRY_DELAY_MS: u64 = 3_600_000;

/// Retry state for tracking retry attempts and timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryState {
    /// Number of retry attempts made so far
    pub attempts: usize,
    /// Timestamp of last retry attempt (milliseconds since epoch)
    pub last_retry_time: u64,
    /// Timestamp when next retry should occur (milliseconds since epoch)
    pub next_retry_time: u64,
}

impl RetryState {
    /// Create a new retry state with no attempts
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            attempts: 0,
            last_retry_time: now,
            next_retry_time: now,
        }
    }

    /// Update retry state after a failed attempt
    pub fn record_attempt(&mut self, next_delay_ms: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.attempts += 1;
        self.last_retry_time = now;
        self.next_retry_time = now + next_delay_ms;
    }
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Decision on whether to retry a failed spec
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDecision {
    /// Retry after the specified delay
    Retry(Duration),
    /// Permanent failure with reason
    PermanentFailure(String),
}

/// Determine whether a failed spec should be retried based on retry state, error log and config.
///
/// # Arguments
/// * `spec_id` - The spec ID (for error messages, currently unused)
/// * `retry_state` - Current retry state with attempt count
/// * `error_log` - The error log content to scan for retryable patterns
/// * `config` - Failure configuration with retry settings and patterns
///
/// # Returns
/// * `Ok(RetryDecision::Retry(delay))` - Should retry after the delay
/// * `Ok(RetryDecision::PermanentFailure(reason))` - Permanent failure, don't retry
/// * `Err(_)` - Configuration error
///
/// # Edge Cases
/// * Empty or missing error log → PermanentFailure
/// * max_retries = 0 → First failure is permanent
/// * No pattern match → PermanentFailure
/// * Backoff overflow → Capped at 1 hour
/// * Multiple pattern matches → Still retryable (OR logic)
/// * Exceeded max_retries → PermanentFailure
pub fn should_retry(
    _spec_id: &str,
    retry_state: &RetryState,
    error_log: &str,
    config: &FailureConfig,
) -> Result<RetryDecision> {
    // Validate config
    config.validate()?;

    // Delegate to decide_retry which has the full implementation
    Ok(decide_retry(retry_state, error_log, config))
}

/// Calculate exponential backoff delay for a given attempt number.
///
/// Formula: delay = base_delay * (backoff_multiplier ^ attempt)
/// Capped at MAX_RETRY_DELAY_MS (1 hour) to prevent overflow.
///
/// # Arguments
/// * `attempt` - The current attempt number (0-indexed)
/// * `base_delay_ms` - Base delay in milliseconds
/// * `backoff_multiplier` - Multiplier for exponential backoff (must be >= 1.0)
///
/// # Returns
/// Delay in milliseconds, capped at 1 hour
pub fn calculate_backoff_delay(attempt: usize, base_delay_ms: u64, backoff_multiplier: f64) -> u64 {
    // Calculate delay with overflow protection
    let delay = (base_delay_ms as f64) * backoff_multiplier.powi(attempt as i32);

    // Cap at maximum delay
    if delay > MAX_RETRY_DELAY_MS as f64 {
        MAX_RETRY_DELAY_MS
    } else {
        delay as u64
    }
}

/// Determine retry decision based on retry state and config.
///
/// # Arguments
/// * `state` - Current retry state with attempt count
/// * `error_log` - Error log to check for retryable patterns
/// * `config` - Failure configuration
///
/// # Returns
/// * `RetryDecision::Retry(delay)` if should retry
/// * `RetryDecision::PermanentFailure(reason)` if should not retry
pub fn decide_retry(state: &RetryState, error_log: &str, config: &FailureConfig) -> RetryDecision {
    // Edge case: Empty or missing error log
    if error_log.trim().is_empty() {
        return RetryDecision::PermanentFailure("Empty error log (no pattern match)".to_string());
    }

    // Edge case: max_retries = 0 means first failure is permanent
    if config.max_retries == 0 {
        return RetryDecision::PermanentFailure("max_retries is 0".to_string());
    }

    // Check if we've exceeded max retries
    if state.attempts >= config.max_retries {
        return RetryDecision::PermanentFailure(format!(
            "Exceeded max retries ({}/{})",
            state.attempts, config.max_retries
        ));
    }

    // Check if error log contains any retryable pattern (OR logic)
    let has_retryable_pattern = config
        .retryable_patterns
        .iter()
        .any(|pattern| error_log.contains(pattern));

    if !has_retryable_pattern {
        return RetryDecision::PermanentFailure(
            "No retryable pattern found in error log".to_string(),
        );
    }

    // Calculate exponential backoff delay
    let delay_ms = calculate_backoff_delay(
        state.attempts,
        config.retry_delay_ms,
        config.backoff_multiplier,
    );

    RetryDecision::Retry(Duration::from_millis(delay_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FailureConfig {
        FailureConfig {
            max_retries: 3,
            retry_delay_ms: 60_000, // 60 seconds
            backoff_multiplier: 2.0,
            retryable_patterns: vec!["rate_limit".to_string()],
            on_permanent_failure: crate::config::OnPermanentFailure::Skip,
        }
    }

    #[test]
    fn test_retry_state_new() {
        let state = RetryState::new();
        assert_eq!(state.attempts, 0);
        assert!(state.last_retry_time > 0);
        assert_eq!(state.last_retry_time, state.next_retry_time);
    }

    #[test]
    fn test_retry_state_record_attempt() {
        let mut state = RetryState::new();
        let initial_time = state.last_retry_time;

        state.record_attempt(5000);

        assert_eq!(state.attempts, 1);
        assert!(state.last_retry_time >= initial_time);
        assert_eq!(state.next_retry_time, state.last_retry_time + 5000);
    }

    #[test]
    fn test_calculate_backoff_delay() {
        // Base case: attempt 0
        assert_eq!(calculate_backoff_delay(0, 60_000, 2.0), 60_000);

        // Attempt 1: 60s * 2^1 = 120s
        assert_eq!(calculate_backoff_delay(1, 60_000, 2.0), 120_000);

        // Attempt 2: 60s * 2^2 = 240s
        assert_eq!(calculate_backoff_delay(2, 60_000, 2.0), 240_000);

        // Attempt 3: 60s * 2^3 = 480s
        assert_eq!(calculate_backoff_delay(3, 60_000, 2.0), 480_000);
    }

    #[test]
    fn test_calculate_backoff_delay_with_different_multiplier() {
        // Multiplier 1.5
        assert_eq!(calculate_backoff_delay(0, 60_000, 1.5), 60_000);
        assert_eq!(calculate_backoff_delay(1, 60_000, 1.5), 90_000);
        assert_eq!(calculate_backoff_delay(2, 60_000, 1.5), 135_000);
    }

    #[test]
    fn test_calculate_backoff_delay_overflow_cap() {
        // Large attempt number should be capped at 1 hour
        let delay = calculate_backoff_delay(100, 60_000, 2.0);
        assert_eq!(delay, MAX_RETRY_DELAY_MS);
    }

    #[test]
    fn test_decide_retry_with_retryable_error() {
        let mut state = RetryState::new();
        let config = test_config();
        let error_log = "Error: API rate_limit exceeded";

        // First attempt (state.attempts = 0)
        let decision = decide_retry(&state, error_log, &config);
        assert!(matches!(decision, RetryDecision::Retry(_)));
        if let RetryDecision::Retry(delay) = decision {
            assert_eq!(delay.as_millis(), 60_000); // 60s
        }

        // Second attempt
        state.record_attempt(60_000);
        let decision = decide_retry(&state, error_log, &config);
        assert!(matches!(decision, RetryDecision::Retry(_)));
        if let RetryDecision::Retry(delay) = decision {
            assert_eq!(delay.as_millis(), 120_000); // 120s
        }

        // Third attempt
        state.record_attempt(120_000);
        let decision = decide_retry(&state, error_log, &config);
        assert!(matches!(decision, RetryDecision::Retry(_)));
        if let RetryDecision::Retry(delay) = decision {
            assert_eq!(delay.as_millis(), 240_000); // 240s
        }

        // Fourth attempt - exceeds max_retries
        state.record_attempt(240_000);
        let decision = decide_retry(&state, error_log, &config);
        assert!(matches!(decision, RetryDecision::PermanentFailure(_)));
    }

    #[test]
    fn test_decide_retry_with_non_retryable_error() {
        let state = RetryState::new();
        let config = test_config();
        let error_log = "Error: syntax error in code";

        let decision = decide_retry(&state, error_log, &config);
        assert!(matches!(decision, RetryDecision::PermanentFailure(_)));
    }

    #[test]
    fn test_decide_retry_empty_error_log() {
        let state = RetryState::new();
        let config = test_config();

        let decision = decide_retry(&state, "", &config);
        assert!(matches!(decision, RetryDecision::PermanentFailure(_)));

        let decision = decide_retry(&state, "   ", &config);
        assert!(matches!(decision, RetryDecision::PermanentFailure(_)));
    }

    #[test]
    fn test_decide_retry_max_retries_zero() {
        let state = RetryState::new();
        let mut config = test_config();
        config.max_retries = 0;

        let error_log = "Error: rate_limit exceeded";
        let decision = decide_retry(&state, error_log, &config);
        assert!(matches!(decision, RetryDecision::PermanentFailure(_)));
    }

    #[test]
    fn test_decide_retry_multiple_patterns() {
        let state = RetryState::new();
        let mut config = test_config();
        config.retryable_patterns = vec![
            "rate_limit".to_string(),
            "timeout".to_string(),
            "connection_refused".to_string(),
        ];

        // Test each pattern matches (OR logic)
        let error_log1 = "Error: rate_limit exceeded";
        assert!(matches!(
            decide_retry(&state, error_log1, &config),
            RetryDecision::Retry(_)
        ));

        let error_log2 = "Error: timeout occurred";
        assert!(matches!(
            decide_retry(&state, error_log2, &config),
            RetryDecision::Retry(_)
        ));

        let error_log3 = "Error: connection_refused";
        assert!(matches!(
            decide_retry(&state, error_log3, &config),
            RetryDecision::Retry(_)
        ));
    }

    #[test]
    fn test_decide_retry_backoff_calculation() {
        let mut state = RetryState::new();
        let config = test_config();
        let error_log = "Error: rate_limit exceeded";

        // Attempt 0: 60s * 2^0 = 60s
        let decision = decide_retry(&state, error_log, &config);
        if let RetryDecision::Retry(delay) = decision {
            assert_eq!(delay.as_secs(), 60);
        }

        // Attempt 1: 60s * 2^1 = 120s
        state.record_attempt(60_000);
        let decision = decide_retry(&state, error_log, &config);
        if let RetryDecision::Retry(delay) = decision {
            assert_eq!(delay.as_secs(), 120);
        }

        // Attempt 2: 60s * 2^2 = 240s
        state.record_attempt(120_000);
        let decision = decide_retry(&state, error_log, &config);
        if let RetryDecision::Retry(delay) = decision {
            assert_eq!(delay.as_secs(), 240);
        }
    }
}
