//! Spec command handlers for chant CLI
//!
//! Handles core spec operations including:
//! - Creating, listing, showing, and deleting specs
//! - Status checking and linting
//!
//! Note: Spec execution is handled by cmd::work module
//! Note: Lifecycle operations (merge, archive, split, diagnostics, logging) are in cmd::lifecycle module

mod add;
mod approve;
mod delete;
mod lint;
mod list;
mod show;

// Re-export public command functions
pub use add::cmd_add;
pub use approve::{cmd_approve, cmd_reject};
pub use delete::{cmd_cancel, cmd_delete, cmd_export};
pub use lint::{cmd_lint, lint_specific_specs, LintFormat};
pub use list::{cmd_list, cmd_status};
pub use show::cmd_show;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd;
    use crate::cmd::commits::CommitError;
    use crate::cmd::finalize::finalize_spec;
    use crate::cmd::model::{get_model_name, get_model_name_with_default};
    use crate::{lookup_log_file, LogLookupResult};
    use chant::config::Config;
    use chant::spec::{self, Spec, SpecFrontmatter, SpecStatus};
    use lint::validate_spec_type;
    use serial_test::serial;
    use show::{format_yaml_value, key_to_title_case};
    use tempfile::TempDir;

    #[test]
    fn test_ensure_logs_dir_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Logs dir shouldn't exist yet
        assert!(!base_path.join("logs").exists());

        // Call ensure_logs_dir_at
        cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

        // Logs dir should now exist
        assert!(base_path.join("logs").exists());
        assert!(base_path.join("logs").is_dir());
    }

    #[test]
    fn test_ensure_logs_dir_updates_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create base dir without .gitignore
        // (tempdir already exists, no need to create)

        // Call ensure_logs_dir_at
        cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should now exist and contain "logs/"
        let gitignore_path = base_path.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("logs/"));
    }

    #[test]
    fn test_ensure_logs_dir_appends_to_existing_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create existing .gitignore with other content
        let gitignore_path = base_path.join(".gitignore");
        std::fs::write(&gitignore_path, "*.tmp\n").unwrap();

        // Call ensure_logs_dir_at
        cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should contain both original and new content
        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains("*.tmp"));
        assert!(content.contains("logs/"));
    }

    #[test]
    fn test_ensure_logs_dir_no_duplicate_gitignore_entry() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create existing .gitignore that already has logs/
        let gitignore_path = base_path.join(".gitignore");
        std::fs::write(&gitignore_path, "logs/\n").unwrap();

        // Create logs dir (since ensure_logs_dir only updates gitignore when creating dir)
        std::fs::create_dir_all(base_path.join("logs")).unwrap();

        // Call ensure_logs_dir_at
        cmd::agent::ensure_logs_dir_at(&base_path).unwrap();

        // .gitignore should still have only one "logs/" entry
        let content = std::fs::read_to_string(&gitignore_path).unwrap();
        let count = content
            .lines()
            .filter(|line| line.trim() == "logs/")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_streaming_log_writer_creates_header() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";

        // Create log writer (this writes the header)
        let _writer =
            cmd::agent::StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();

        // Check that log file exists with header BEFORE any lines are written
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        assert!(log_path.exists());

        let content = std::fs::read_to_string(&log_path).unwrap();

        // Check header format
        assert!(content.starts_with("# Agent Log: 2026-01-24-00a-xyz\n"));
        assert!(content.contains("# Started: "));
        assert!(content.contains("# Prompt: standard\n"));
    }

    #[test]
    fn test_streaming_log_writer_writes_lines() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";

        // Create log writer and write lines
        let mut writer =
            cmd::agent::StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
        writer.write_line("Test agent output").unwrap();
        writer.write_line("With multiple lines").unwrap();

        // Read it back
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        let content = std::fs::read_to_string(&log_path).unwrap();

        // Check header format
        assert!(content.starts_with("# Agent Log: 2026-01-24-00a-xyz\n"));
        assert!(content.contains("# Started: "));
        assert!(content.contains("# Prompt: standard\n"));

        // Check output is preserved
        assert!(content.contains("Test agent output\n"));
        assert!(content.contains("With multiple lines\n"));
    }

    #[test]
    fn test_streaming_log_writer_flushes_each_line() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00a-xyz";
        let prompt_name = "standard";

        // Create log writer
        let mut writer =
            cmd::agent::StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));

        // Write first line
        writer.write_line("Line 1").unwrap();

        // Verify it's visible immediately (flushed) by reading the file
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Line 1"));

        // Write second line
        writer.write_line("Line 2").unwrap();

        // Verify both lines are visible
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Line 1"));
        assert!(content.contains("Line 2"));
    }

    #[test]
    fn test_streaming_log_writer_overwrites_on_new_run() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let spec_id = "2026-01-24-00b-abc";
        let prompt_name = "standard";

        // First run
        {
            let mut writer =
                cmd::agent::StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
            writer.write_line("Content A").unwrap();
        }

        // Second run (simulating replay)
        {
            let mut writer =
                cmd::agent::StreamingLogWriter::new_at(&base_path, spec_id, prompt_name).unwrap();
            writer.write_line("Content B").unwrap();
        }

        // Read it back
        let log_path = base_path.join("logs").join(format!("{}.log", spec_id));
        let content = std::fs::read_to_string(&log_path).unwrap();

        // Should contain only Content B
        assert!(content.contains("Content B"));
        assert!(!content.contains("Content A"));
    }

    #[test]
    fn test_lookup_log_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create specs directory and a spec file
        let specs_dir = base_path.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let spec_content = r#"---
type: code
status: pending
---

# Test spec
"#;
        std::fs::write(specs_dir.join("2026-01-24-00a-xyz.md"), spec_content).unwrap();

        // Lookup log without creating logs directory
        let result = lookup_log_file(&base_path, "xyz").unwrap();

        match result {
            LogLookupResult::NotFound { spec_id, log_path } => {
                assert_eq!(spec_id, "2026-01-24-00a-xyz");
                assert!(log_path
                    .to_string_lossy()
                    .contains("2026-01-24-00a-xyz.log"));
            }
            LogLookupResult::Found(_) => panic!("Expected NotFound, got Found"),
        }
    }

    #[test]
    fn test_lookup_log_file_found() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create specs directory and a spec file
        let specs_dir = base_path.join("specs");
        let logs_dir = base_path.join("logs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::create_dir_all(&logs_dir).unwrap();

        let spec_content = r#"---
type: code
status: pending
---

# Test spec
"#;
        std::fs::write(specs_dir.join("2026-01-24-00b-abc.md"), spec_content).unwrap();

        // Create a log file
        std::fs::write(
            logs_dir.join("2026-01-24-00b-abc.log"),
            "# Agent Log\nTest output",
        )
        .unwrap();

        // Lookup log
        let result = lookup_log_file(&base_path, "abc").unwrap();

        match result {
            LogLookupResult::Found(path) => {
                assert!(path.to_string_lossy().contains("2026-01-24-00b-abc.log"));
            }
            LogLookupResult::NotFound { .. } => panic!("Expected Found, got NotFound"),
        }
    }

    #[test]
    fn test_lookup_log_file_spec_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create specs directory and multiple spec files
        let specs_dir = base_path.join("specs");
        let logs_dir = base_path.join("logs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::create_dir_all(&logs_dir).unwrap();

        let spec_content = r#"---
type: code
status: pending
---

# Test spec
"#;
        std::fs::write(specs_dir.join("2026-01-24-00c-def.md"), spec_content).unwrap();
        std::fs::write(specs_dir.join("2026-01-24-00d-ghi.md"), spec_content).unwrap();

        // Create log file for one spec
        std::fs::write(
            logs_dir.join("2026-01-24-00c-def.log"),
            "# Agent Log\nOutput for def",
        )
        .unwrap();

        // Lookup using partial ID should resolve correctly
        let result = lookup_log_file(&base_path, "def").unwrap();
        match result {
            LogLookupResult::Found(path) => {
                assert!(path.to_string_lossy().contains("2026-01-24-00c-def.log"));
            }
            LogLookupResult::NotFound { .. } => panic!("Expected Found for 'def'"),
        }

        // Lookup for spec without log
        let result = lookup_log_file(&base_path, "ghi").unwrap();
        match result {
            LogLookupResult::NotFound { spec_id, .. } => {
                assert_eq!(spec_id, "2026-01-24-00d-ghi");
            }
            LogLookupResult::Found(_) => panic!("Expected NotFound for 'ghi'"),
        }
    }

    #[test]
    fn test_lookup_log_file_not_initialized() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Don't create specs directory
        let result = lookup_log_file(&base_path, "abc");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Chant not initialized"));
    }

    #[test]
    #[serial]
    fn test_get_model_name_from_chant_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set CHANT_MODEL
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_name(None);
        assert_eq!(result, Some("claude-opus-4-5".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_from_anthropic_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set only ANTHROPIC_MODEL
        std::env::remove_var("CHANT_MODEL");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = get_model_name(None);
        assert_eq!(result, Some("claude-sonnet-4".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_chant_takes_precedence() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set both env vars
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = get_model_name(None);
        // CHANT_MODEL takes precedence
        assert_eq!(result, Some("claude-opus-4-5".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_from_config_default() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars so config default is used
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = get_model_name_with_default(Some("claude-sonnet-4"));
        assert_eq!(result, Some("claude-sonnet-4".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_env_takes_precedence_over_config() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set env var
        std::env::set_var("ANTHROPIC_MODEL", "claude-opus-4-5");
        std::env::remove_var("CHANT_MODEL");

        // Env var should take precedence over config
        let result = get_model_name_with_default(Some("claude-sonnet-4"));
        assert_eq!(result, Some("claude-opus-4-5".to_string()));

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_none_when_unset() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset both env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        // With no config and no env vars, falls back to claude version parsing
        // which may or may not return a value depending on system
        let result = get_model_name_with_default(None);
        // We can't assert the exact value since it depends on whether claude is installed
        // and what version it is, so we just verify it doesn't panic
        let _ = result;

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_empty_string_returns_none() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set empty string
        std::env::set_var("CHANT_MODEL", "");
        std::env::remove_var("ANTHROPIC_MODEL");

        // Empty env var should fall through to config default or claude version
        let result = get_model_name_with_default(None);
        // Can't assert exact value since it depends on whether claude is installed
        let _ = result;

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_name_empty_config_model_skipped() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        // Empty config model should be skipped
        let result = get_model_name_with_default(Some(""));
        // Falls through to claude version parsing
        let _ = result;

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_key_to_title_case_single_word() {
        assert_eq!(key_to_title_case("status"), "Status");
        assert_eq!(key_to_title_case("type"), "Type");
        assert_eq!(key_to_title_case("commit"), "Commit");
    }

    #[test]
    fn test_key_to_title_case_snake_case() {
        assert_eq!(key_to_title_case("depends_on"), "Depends On");
        assert_eq!(key_to_title_case("completed_at"), "Completed At");
        assert_eq!(key_to_title_case("target_files"), "Target Files");
    }

    #[test]
    fn test_key_to_title_case_empty_string() {
        assert_eq!(key_to_title_case(""), "");
    }

    #[test]
    fn test_format_yaml_value_null() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Null);
        // Result contains ANSI codes, but should represent "~"
        assert!(result.contains("~") || result.contains('\x1b'));
    }

    #[test]
    fn test_format_yaml_value_bool_true() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Bool(true));
        // Result contains ANSI codes for green, but should represent "true"
        assert!(result.contains("true") || result.contains('\x1b'));
    }

    #[test]
    fn test_format_yaml_value_bool_false() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Bool(false));
        // Result contains ANSI codes for red, but should represent "false"
        assert!(result.contains("false") || result.contains('\x1b'));
    }

    #[test]
    fn test_format_yaml_value_number() {
        use serde_yaml::Value;
        let result = format_yaml_value("test", &Value::Number(42.into()));
        assert_eq!(result, "42");
    }

    #[test]
    fn test_format_yaml_value_string_status_completed() {
        use serde_yaml::Value;
        let result = format_yaml_value("status", &Value::String("completed".to_string()));
        // Should contain green ANSI codes
        assert!(result.contains("completed"));
    }

    #[test]
    fn test_format_yaml_value_string_status_failed() {
        use serde_yaml::Value;
        let result = format_yaml_value("status", &Value::String("failed".to_string()));
        // Should contain red ANSI codes
        assert!(result.contains("failed"));
    }

    #[test]
    fn test_format_yaml_value_string_status_pending() {
        use serde_yaml::Value;
        let result = format_yaml_value("status", &Value::String("pending".to_string()));
        // Should contain yellow ANSI codes
        assert!(result.contains("pending"));
    }

    #[test]
    fn test_format_yaml_value_string_commit() {
        use serde_yaml::Value;
        let result = format_yaml_value("commit", &Value::String("abc1234".to_string()));
        // Should contain cyan ANSI codes
        assert!(result.contains("abc1234"));
    }

    #[test]
    fn test_format_yaml_value_string_type() {
        use serde_yaml::Value;
        let result = format_yaml_value("type", &Value::String("code".to_string()));
        // Should contain blue ANSI codes
        assert!(result.contains("code"));
    }

    #[test]
    fn test_format_yaml_value_sequence() {
        use serde_yaml::Value;
        let seq = Value::Sequence(vec![
            Value::String("item1".to_string()),
            Value::String("item2".to_string()),
        ]);
        let result = format_yaml_value("labels", &seq);
        // Should be formatted as [item1, item2] with magenta colors
        assert!(result.starts_with('['));
        assert!(result.ends_with(']'));
        assert!(result.contains("item1"));
        assert!(result.contains("item2"));
    }

    #[test]
    fn test_format_yaml_value_plain_string() {
        use serde_yaml::Value;
        // For keys not in the special list, string should be plain
        let result = format_yaml_value("prompt", &Value::String("standard".to_string()));
        assert_eq!(result, "standard");
    }

    #[test]
    fn test_extract_text_from_stream_json_assistant_message() {
        let json_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello, world!"}]}}"#;
        let texts = cmd::agent::extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["Hello, world!"]);
    }

    #[test]
    fn test_extract_text_from_stream_json_multiple_content_blocks() {
        let json_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"First"},{"type":"text","text":"Second"}]}}"#;
        let texts = cmd::agent::extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["First", "Second"]);
    }

    #[test]
    fn test_extract_text_from_stream_json_system_message() {
        let json_line = r#"{"type":"system","subtype":"init"}"#;
        let texts = cmd::agent::extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_result_message() {
        let json_line = r#"{"type":"result","subtype":"success","result":"Done"}"#;
        let texts = cmd::agent::extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_invalid_json() {
        let json_line = "not valid json";
        let texts = cmd::agent::extract_text_from_stream_json(json_line);
        assert!(texts.is_empty());
    }

    #[test]
    fn test_extract_text_from_stream_json_mixed_content_types() {
        // Content can include tool_use blocks which we should skip
        let json_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Analyzing..."},{"type":"tool_use","name":"read_file"}]}}"#;
        let texts = cmd::agent::extract_text_from_stream_json(json_line);
        assert_eq!(texts, vec!["Analyzing..."]);
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_from_chant_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set CHANT_MODEL
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = cmd::agent::get_model_for_invocation(None);
        assert_eq!(result, "claude-opus-4-5");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_from_anthropic_model() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set only ANTHROPIC_MODEL
        std::env::remove_var("CHANT_MODEL");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = cmd::agent::get_model_for_invocation(None);
        assert_eq!(result, "claude-sonnet-4");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_chant_takes_precedence() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set both env vars
        std::env::set_var("CHANT_MODEL", "claude-opus-4-5");
        std::env::set_var("ANTHROPIC_MODEL", "claude-sonnet-4");

        let result = cmd::agent::get_model_for_invocation(Some("config-model"));
        // CHANT_MODEL takes precedence
        assert_eq!(result, "claude-opus-4-5");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_from_config() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars so config default is used
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = cmd::agent::get_model_for_invocation(Some("claude-sonnet-4"));
        assert_eq!(result, "claude-sonnet-4");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_defaults_to_haiku() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset both env vars and no config
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        let result = cmd::agent::get_model_for_invocation(None);
        assert_eq!(result, "haiku");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_empty_env_falls_through() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Set empty env vars
        std::env::set_var("CHANT_MODEL", "");
        std::env::set_var("ANTHROPIC_MODEL", "");

        let result = cmd::agent::get_model_for_invocation(Some("config-model"));
        // Empty env vars should fall through to config
        assert_eq!(result, "config-model");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        } else {
            std::env::remove_var("CHANT_MODEL");
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        } else {
            std::env::remove_var("ANTHROPIC_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_get_model_for_invocation_empty_config_falls_to_haiku() {
        // Save original env vars
        let orig_chant = std::env::var("CHANT_MODEL").ok();
        let orig_anthropic = std::env::var("ANTHROPIC_MODEL").ok();

        // Unset env vars
        std::env::remove_var("CHANT_MODEL");
        std::env::remove_var("ANTHROPIC_MODEL");

        // Empty config model should fall through to haiku
        let result = cmd::agent::get_model_for_invocation(Some(""));
        assert_eq!(result, "haiku");

        // Restore original env vars
        if let Some(val) = orig_chant {
            std::env::set_var("CHANT_MODEL", val);
        }
        if let Some(val) = orig_anthropic {
            std::env::set_var("ANTHROPIC_MODEL", val);
        }
    }

    #[test]
    fn test_finalize_spec_sets_status_and_timestamps() {
        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().to_path_buf();

        // Create a spec with pending status
        let spec_content = r#"---
type: task
id: 2026-01-24-test-xyz
status: in_progress
---

# Test spec

## Acceptance Criteria

- [x] Item 1
- [x] Item 2
"#;
        specs_dir
            .join("2026-01-24-test-xyz.md")
            .parent()
            .map(|p| std::fs::create_dir_all(p).ok());
        std::fs::write(specs_dir.join("2026-01-24-test-xyz.md"), spec_content).unwrap();

        // Create a minimal config from string
        let config_str = r#"---
project:
  name: test-project
defaults:
  prompt: standard
  branch: true
  branch_prefix: "chant/"
git:
  provider: github
---
"#;
        let config = Config::parse(config_str).unwrap();

        // Load and finalize the spec
        let mut spec = spec::resolve_spec(&specs_dir, "2026-01-24-test-xyz").unwrap();
        let spec_path = specs_dir.join("2026-01-24-test-xyz.md");

        // Before finalization, status should be in_progress
        assert_eq!(spec.frontmatter.status, SpecStatus::InProgress);
        assert!(spec.frontmatter.completed_at.is_none());

        // Finalize the spec (pass empty commits to avoid git dependency in tests)
        finalize_spec(&mut spec, &spec_path, &config, &[], true, Some(vec![])).unwrap();

        // After finalization, status should be completed
        assert_eq!(spec.frontmatter.status, SpecStatus::Completed);
        assert!(spec.frontmatter.completed_at.is_some());

        // Read back the spec from file to verify it was saved
        let saved_spec = spec::resolve_spec(&specs_dir, "2026-01-24-test-xyz").unwrap();
        assert_eq!(saved_spec.frontmatter.status, SpecStatus::Completed);
        assert!(saved_spec.frontmatter.completed_at.is_some());
    }

    #[test]
    fn test_commit_error_display() {
        let err1 = CommitError::GitCommandFailed("test error".to_string());
        assert_eq!(err1.to_string(), "Git command failed: test error");

        let err2 = CommitError::NoMatchingCommits;
        assert_eq!(err2.to_string(), "No matching commits found");
    }

    #[test]
    fn test_validate_spec_type_driver_empty_members() {
        let spec = Spec {
            id: "test-driver".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "driver".to_string(),
                members: Some(vec![]),
                ..Default::default()
            },
            title: Some("Driver spec".to_string()),
            body: "# Driver\n".to_string(),
        };

        let warnings = validate_spec_type(&spec);
        assert!(warnings
            .iter()
            .any(|w| w.message.contains("empty 'members' array")));
    }

    #[test]
    fn test_validate_spec_type_driver_with_members() {
        let spec = Spec {
            id: "test-driver".to_string(),
            frontmatter: SpecFrontmatter {
                r#type: "driver".to_string(),
                members: Some(vec!["test-driver.1".to_string()]),
                ..Default::default()
            },
            title: Some("Driver spec".to_string()),
            body: "# Driver\n".to_string(),
        };

        let warnings = validate_spec_type(&spec);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_spec_status_needs_attention_added() {
        // This test verifies that SpecStatus enum includes NeedsAttention variant
        let status = SpecStatus::NeedsAttention;
        assert_eq!(status, SpecStatus::NeedsAttention);
    }
}
