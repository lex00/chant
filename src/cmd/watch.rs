//! Watch command logging infrastructure
//!
//! Provides structured logging with timestamps for the watch command,
//! writing to both stdout and a persistent log file.

use anyhow::{Context, Result};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Logger for watch command with structured output and file persistence
#[allow(dead_code)]
pub struct WatchLogger {
    log_file: Option<std::fs::File>,
    log_path: PathBuf,
    stdout_only: bool,
}

#[allow(dead_code)]
impl WatchLogger {
    /// Initialize the watch logger with log file at `.chant/logs/watch.log`
    pub fn init() -> Result<Self> {
        let log_dir = PathBuf::from(".chant/logs");
        let log_path = log_dir.join("watch.log");

        // Create log directory if it doesn't exist
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir).with_context(|| {
                format!("Failed to create log directory: {}", log_dir.display())
            })?;
        }

        // Try to open log file in append mode
        let (log_file, stdout_only) =
            match OpenOptions::new().create(true).append(true).open(&log_path) {
                Ok(file) => (Some(file), false),
                Err(e) => {
                    // Log file unwritable - fall back to stdout-only mode
                    eprintln!(
                        "Warning: Could not open log file at {}: {}",
                        log_path.display(),
                        e
                    );
                    eprintln!("Continuing with stdout-only logging");
                    (None, true)
                }
            };

        Ok(WatchLogger {
            log_file,
            log_path,
            stdout_only,
        })
    }

    /// Log an event with timestamp to both stdout and file
    pub fn log_event(&mut self, message: &str) -> Result<()> {
        let timestamp = Local::now().format("[%H:%M:%S]");
        let formatted = format!("{} {}", timestamp, message);

        // Write to stdout
        println!("{}", formatted);

        // Write to file if available
        if let Some(ref mut file) = self.log_file {
            writeln!(file, "{}", formatted).with_context(|| {
                format!("Failed to write to log file: {}", self.log_path.display())
            })?;

            // Flush to ensure visibility during long runs
            file.flush().with_context(|| {
                format!("Failed to flush log file: {}", self.log_path.display())
            })?;
        }

        Ok(())
    }

    /// Get the path to the log file
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Check if logger is in stdout-only mode (file logging failed)
    pub fn is_stdout_only(&self) -> bool {
        self.stdout_only
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_logger_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        // Create .chant directory (but not logs subdirectory)
        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let logger = WatchLogger::init().unwrap();
        assert!(PathBuf::from(".chant/logs").exists());
        assert!(!logger.is_stdout_only());

        std::env::set_current_dir(&original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_logger_writes_to_file() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("Test message").unwrap();

        // Read using relative path while still in tmp directory
        let contents = fs::read_to_string(".chant/logs/watch.log").unwrap();

        std::env::set_current_dir(&original_dir).unwrap();

        assert!(contents.contains("Test message"));
        assert!(contents.contains("["));
        assert!(contents.contains("]"));
    }

    #[test]
    #[serial]
    fn test_logger_multiple_events() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("Event 1").unwrap();
        logger.log_event("Event 2").unwrap();
        logger.log_event("Event 3").unwrap();

        // Read using relative path while still in tmp directory
        let contents = fs::read_to_string(".chant/logs/watch.log").unwrap();

        std::env::set_current_dir(&original_dir).unwrap();

        assert!(contents.contains("Event 1"));
        assert!(contents.contains("Event 2"));
        assert!(contents.contains("Event 3"));

        // Count lines
        let line_count = contents.lines().count();
        assert_eq!(line_count, 3);
    }

    #[test]
    #[serial]
    fn test_logger_appends_to_existing_file() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let logs_dir = tmp.path().join(".chant/logs");
        fs::create_dir_all(&logs_dir).unwrap();
        let log_path = logs_dir.join("watch.log");
        fs::write(&log_path, "Existing content\n").unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("New event").unwrap();

        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("Existing content"));
        assert!(contents.contains("New event"));

        std::env::set_current_dir(&original_dir).unwrap();
    }

    #[test]
    #[serial]
    fn test_timestamp_format() {
        let tmp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        let chant_dir = tmp.path().join(".chant");
        fs::create_dir(&chant_dir).unwrap();

        std::env::set_current_dir(tmp.path()).unwrap();

        let mut logger = WatchLogger::init().unwrap();
        logger.log_event("Test").unwrap();

        // Read using relative path while still in tmp directory
        let contents = fs::read_to_string(".chant/logs/watch.log").unwrap();

        std::env::set_current_dir(&original_dir).unwrap();

        // Should match [HH:MM:SS] format
        assert!(contents.starts_with("["));
        let parts: Vec<&str> = contents.split(']').collect();
        assert!(parts.len() >= 2);
        // Timestamp should be in format [HH:MM:SS]
        let timestamp = parts[0].trim_start_matches('[');
        let time_parts: Vec<&str> = timestamp.split(':').collect();
        assert_eq!(time_parts.len(), 3); // HH:MM:SS
    }
}
