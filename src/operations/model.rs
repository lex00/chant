//! Model selection and detection logic.
//!
//! Handles model name resolution with the following priority:
//! 1. CHANT_MODEL env var (explicit override)
//! 2. ANTHROPIC_MODEL env var (Claude CLI default)
//! 3. defaults.model in config
//! 4. Parse from `claude --version` output (last resort)
//!
//! All model names are normalized to shorthand (e.g., "claude-sonnet-4-20250514" -> "sonnet").

use crate::config::Config;
use crate::spec::normalize_model_name;

/// Get the model name using the following priority:
/// 1. CHANT_MODEL env var (explicit override)
/// 2. ANTHROPIC_MODEL env var (Claude CLI default)
/// 3. defaults.model in config
/// 4. Parse from `claude --version` output (last resort)
///
/// Model names are normalized to shorthand (e.g., "claude-sonnet-4-20250514" -> "sonnet").
pub fn get_model_name(config: Option<&Config>) -> Option<String> {
    get_model_name_with_default(config.and_then(|c| c.defaults.model.as_deref()))
}

/// Get the model name with an optional default from config.
/// Used by parallel execution where full Config isn't available.
///
/// Model names are normalized to shorthand (e.g., "claude-sonnet-4-20250514" -> "sonnet").
pub fn get_model_name_with_default(config_model: Option<&str>) -> Option<String> {
    // 1. CHANT_MODEL env var
    if let Ok(model) = std::env::var("CHANT_MODEL") {
        if !model.is_empty() {
            return Some(normalize_model_name(&model));
        }
    }

    // 2. ANTHROPIC_MODEL env var
    if let Ok(model) = std::env::var("ANTHROPIC_MODEL") {
        if !model.is_empty() {
            return Some(normalize_model_name(&model));
        }
    }

    // 3. defaults.model from config
    if let Some(model) = config_model {
        if !model.is_empty() {
            return Some(normalize_model_name(model));
        }
    }

    // 4. Parse from claude --version output
    parse_model_from_claude_version().map(|m| normalize_model_name(&m))
}

/// Parse model name from `claude --version` output.
/// Expected format: "X.Y.Z (model-name)" or similar patterns.
fn parse_model_from_claude_version() -> Option<String> {
    use std::process::Command;

    let output = Command::new("claude").arg("--version").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);

    // Try to extract model from parentheses, e.g., "1.0.0 (claude-sonnet-4)"
    if let Some(start) = version_str.find('(') {
        if let Some(end) = version_str.find(')') {
            if start < end {
                let model = version_str[start + 1..end].trim();
                // Check if it looks like a model name (contains "claude" or common model patterns)
                if model.contains("claude")
                    || model.contains("sonnet")
                    || model.contains("opus")
                    || model.contains("haiku")
                {
                    return Some(model.to_string());
                }
            }
        }
    }

    None
}
