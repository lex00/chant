//! Structured output abstraction for chant.
//!
//! Provides a unified interface for outputting messages in different modes:
//! - Human: Colored emoji-prefixed output for terminal display
//! - Json: Structured JSON events for programmatic consumption
//! - Quiet: Only errors are emitted
//!
//! The Output struct auto-detects TTY for color support and can be injected
//! with a custom writer for test capture.

use colored::{Color, Colorize};
use serde_json::json;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Output mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable colored output with emoji prefixes
    Human,
    /// JSON-formatted structured output
    Json,
    /// Silent mode - only errors
    Quiet,
}

/// Output abstraction with mode-aware formatting
#[derive(Clone)]
pub struct Output {
    mode: OutputMode,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    is_tty: bool,
}

impl Output {
    /// Create a new Output writing to stdout
    pub fn new(mode: OutputMode) -> Self {
        let is_tty = atty::is(atty::Stream::Stdout);
        Self {
            mode,
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
            is_tty,
        }
    }

    /// Create an Output with a custom writer (for testing)
    pub fn with_writer(mode: OutputMode, writer: Box<dyn Write + Send>) -> Self {
        Self {
            mode,
            writer: Arc::new(Mutex::new(writer)),
            is_tty: false, // Assume non-TTY for custom writers
        }
    }

    /// Output a step message: "→ {msg}" in cyan
    pub fn step(&self, msg: &str) {
        match self.mode {
            OutputMode::Human => {
                let prefix = if self.is_tty {
                    "→".cyan().to_string()
                } else {
                    "→".to_string()
                };
                self.write_line(&format!("{} {}", prefix, msg));
            }
            OutputMode::Json => {
                self.write_json("step", msg, None);
            }
            OutputMode::Quiet => {}
        }
    }

    /// Output a success message: "✓ {msg}" in green
    pub fn success(&self, msg: &str) {
        match self.mode {
            OutputMode::Human => {
                let prefix = if self.is_tty {
                    "✓".green().to_string()
                } else {
                    "✓".to_string()
                };
                self.write_line(&format!("{} {}", prefix, msg));
            }
            OutputMode::Json => {
                self.write_json("success", msg, None);
            }
            OutputMode::Quiet => {}
        }
    }

    /// Output a warning message: "⚠ {msg}" in yellow
    pub fn warn(&self, msg: &str) {
        match self.mode {
            OutputMode::Human => {
                let prefix = if self.is_tty {
                    "⚠".yellow().to_string()
                } else {
                    "⚠".to_string()
                };
                self.write_line(&format!("{} {}", prefix, msg));
            }
            OutputMode::Json => {
                self.write_json("warning", msg, None);
            }
            OutputMode::Quiet => {}
        }
    }

    /// Output an error message: "✗ {msg}" in red
    pub fn error(&self, msg: &str) {
        match self.mode {
            OutputMode::Human => {
                let prefix = if self.is_tty {
                    "✗".red().to_string()
                } else {
                    "✗".to_string()
                };
                self.write_line(&format!("{} {}", prefix, msg));
            }
            OutputMode::Json => {
                self.write_json("error", msg, None);
            }
            OutputMode::Quiet => {
                // Errors always output, even in quiet mode
                self.write_line(&format!("✗ {}", msg));
            }
        }
    }

    /// Output plain info text (no prefix)
    pub fn info(&self, msg: &str) {
        match self.mode {
            OutputMode::Human => {
                self.write_line(msg);
            }
            OutputMode::Json => {
                self.write_json("info", msg, None);
            }
            OutputMode::Quiet => {}
        }
    }

    /// Output detail text (indented, for subordinate info)
    pub fn detail(&self, msg: &str) {
        match self.mode {
            OutputMode::Human => {
                self.write_line(&format!("  {}", msg));
            }
            OutputMode::Json => {
                self.write_json("detail", msg, None);
            }
            OutputMode::Quiet => {}
        }
    }

    /// Output a colored message with a custom prefix and color
    pub fn colored(&self, prefix: &str, msg: &str, color: Color) {
        match self.mode {
            OutputMode::Human => {
                let formatted_prefix = if self.is_tty {
                    prefix.color(color).to_string()
                } else {
                    prefix.to_string()
                };
                self.write_line(&format!("{} {}", formatted_prefix, msg));
            }
            OutputMode::Json => {
                self.write_json("message", msg, Some(("prefix", prefix)));
            }
            OutputMode::Quiet => {}
        }
    }

    /// Output a structured JSON event
    pub fn json(&self, value: &serde_json::Value) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{}", value);
        }
    }

    /// Write a line to the output
    fn write_line(&self, line: &str) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{}", line);
        }
    }

    /// Write a JSON-formatted log line
    fn write_json(&self, level: &str, msg: &str, extra: Option<(&str, &str)>) {
        if let Ok(mut writer) = self.writer.lock() {
            let mut obj = json!({
                "level": level,
                "msg": msg,
            });

            if let Some((key, value)) = extra {
                obj[key] = json!(value);
            }

            let _ = writeln!(writer, "{}", obj);
        }
    }

    /// Get the current output mode
    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Check if running in TTY
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // Test-specific writer that wraps Arc<Mutex<Vec<u8>>>
    struct TestWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl TestWriter {
        fn new() -> (Self, Arc<Mutex<Vec<u8>>>) {
            let buffer = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    buffer: buffer.clone(),
                },
                buffer,
            )
        }
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buffer.lock().unwrap().flush()
        }
    }

    #[test]
    fn test_human_mode_output() {
        let (writer, buffer) = TestWriter::new();
        let output = Output::with_writer(OutputMode::Human, Box::new(writer));

        output.step("Starting");
        output.success("Done");
        output.warn("Warning");
        output.error("Error");
        output.info("Info");
        output.detail("Detail");

        let data = buffer.lock().unwrap();
        let result = String::from_utf8(data.clone()).unwrap();
        assert!(result.contains("→ Starting"));
        assert!(result.contains("✓ Done"));
        assert!(result.contains("⚠ Warning"));
        assert!(result.contains("✗ Error"));
        assert!(result.contains("Info"));
        assert!(result.contains("  Detail"));
    }

    #[test]
    fn test_json_mode_output() {
        let (writer, buffer) = TestWriter::new();
        let output = Output::with_writer(OutputMode::Json, Box::new(writer));

        output.step("Starting");
        output.success("Done");

        let data = buffer.lock().unwrap();
        let result = String::from_utf8(data.clone()).unwrap();
        assert!(result.contains(r#""level":"step""#));
        assert!(result.contains(r#""msg":"Starting""#));
        assert!(result.contains(r#""level":"success""#));
        assert!(result.contains(r#""msg":"Done""#));
    }

    #[test]
    fn test_quiet_mode_only_errors() {
        let (writer, buffer) = TestWriter::new();
        let output = Output::with_writer(OutputMode::Quiet, Box::new(writer));

        output.step("Starting");
        output.success("Done");
        output.warn("Warning");
        output.error("Error");
        output.info("Info");

        let data = buffer.lock().unwrap();
        let result = String::from_utf8(data.clone()).unwrap();
        // Only error should be present
        assert!(result.contains("✗ Error"));
        assert!(!result.contains("Starting"));
        assert!(!result.contains("Done"));
        assert!(!result.contains("Warning"));
        assert!(!result.contains("Info"));
    }

    #[test]
    fn test_mode_getter() {
        let output = Output::new(OutputMode::Json);
        assert_eq!(output.mode(), OutputMode::Json);
    }
}
