//! Model provider abstraction for invoking AI agents.
//!
//! Supports multiple providers (Claude, Ollama, OpenAI).
//!
//! # Doc Audit
//! - audited: (pending)
//! - docs: architecture/invoke.md
//! - ignore: false

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::io::BufRead;
use std::process::{Command, Stdio};
use ureq::Agent;

/// Model provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    #[default]
    Claude,
    Ollama,
    Openai,
}

/// Provider configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,
    #[serde(default)]
    pub openai: Option<OpenaiConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
}

fn default_ollama_endpoint() -> String {
    "http://localhost:11434/v1".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenaiConfig {
    #[serde(default = "default_openai_endpoint")]
    pub endpoint: String,
}

fn default_openai_endpoint() -> String {
    "https://api.openai.com/v1".to_string()
}

/// Trait for model providers
pub trait ModelProvider {
    fn invoke(
        &self,
        message: &str,
        model: &str,
        callback: &mut dyn FnMut(&str) -> Result<()>,
    ) -> Result<String>;

    #[allow(dead_code)]
    fn name(&self) -> &'static str;
}

/// Claude CLI provider (existing behavior)
pub struct ClaudeCliProvider;

impl ModelProvider for ClaudeCliProvider {
    fn invoke(
        &self,
        message: &str,
        model: &str,
        callback: &mut dyn FnMut(&str) -> Result<()>,
    ) -> Result<String> {
        let mut cmd = Command::new("claude");
        cmd.arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--model")
            .arg(model)
            .arg("--dangerously-skip-permissions")
            .arg(message)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .context("Failed to invoke claude CLI. Is it installed and in PATH?")?;

        let mut captured_output = String::new();
        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                for text in extract_text_from_stream_json(&line) {
                    for text_line in text.lines() {
                        callback(text_line)?;
                        captured_output.push_str(text_line);
                        captured_output.push('\n');
                    }
                }
            }
        }

        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Agent exited with status: {}", status);
        }

        Ok(captured_output)
    }

    fn name(&self) -> &'static str {
        "claude"
    }
}

/// Ollama provider (OpenAI-compatible API with agent runtime)
pub struct OllamaProvider {
    pub endpoint: String,
}

impl ModelProvider for OllamaProvider {
    fn invoke(
        &self,
        message: &str,
        model: &str,
        callback: &mut dyn FnMut(&str) -> Result<()>,
    ) -> Result<String> {
        // Validate endpoint URL
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(anyhow!("Invalid endpoint URL: {}", self.endpoint));
        }

        // Use tokio runtime for async ollama-rs agent
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(crate::agent::run_agent(
            &self.endpoint,
            model,
            "",
            message,
            callback,
        ))
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Connection") || err_str.contains("connect") {
                anyhow!("Failed to connect to Ollama at {}\n\nOllama does not appear to be running. To fix:\n\n  1. Install Ollama: https://ollama.ai/download\n  2. Start Ollama: ollama serve\n  3. Pull a model: ollama pull {}\n\nOr switch to Claude CLI by removing 'provider: ollama' from .chant/config.md", self.endpoint, model)
            } else {
                e
            }
        })
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}

/// OpenAI provider
pub struct OpenaiProvider {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl ModelProvider for OpenaiProvider {
    fn invoke(
        &self,
        message: &str,
        model: &str,
        callback: &mut dyn FnMut(&str) -> Result<()>,
    ) -> Result<String> {
        let api_key = self
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow!("OPENAI_API_KEY environment variable not set"))?;

        let url = format!("{}/chat/completions", self.endpoint);

        // Validate endpoint URL
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(anyhow!("Invalid endpoint URL: {}", self.endpoint));
        }

        let request_body = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": message
                }
            ],
            "stream": true,
        });

        // Create HTTP agent and send request
        let agent = Agent::new();
        let response = agent
            .post(&url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", api_key))
            .send_json(&request_body)
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        // Check response status
        if response.status() == 401 {
            return Err(anyhow!(
                "Authentication failed. Check OPENAI_API_KEY env var"
            ));
        }

        if response.status() != 200 {
            return Err(anyhow!(
                "HTTP {}: {}",
                response.status(),
                response.status_text()
            ));
        }

        // Stream response body - buffer tokens until we have complete lines
        let reader = std::io::BufReader::new(response.into_reader());
        let mut captured_output = String::new();
        let mut line_buffer = String::new();

        for line in reader.lines().map_while(Result::ok) {
            if let Some(json_str) = line.strip_prefix("data: ") {
                if json_str == "[DONE]" {
                    break;
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                        for choice in choices {
                            if let Some(delta) = choice.get("delta") {
                                if let Some(content) = delta.get("content").and_then(|c| c.as_str())
                                {
                                    line_buffer.push_str(content);

                                    // Only callback when we have complete lines
                                    while let Some(newline_pos) = line_buffer.find('\n') {
                                        let complete_line = &line_buffer[..newline_pos];
                                        callback(complete_line)?;
                                        captured_output.push_str(complete_line);
                                        captured_output.push('\n');
                                        line_buffer = line_buffer[newline_pos + 1..].to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Flush any remaining buffered content
        if !line_buffer.is_empty() {
            callback(&line_buffer)?;
            captured_output.push_str(&line_buffer);
            captured_output.push('\n');
        }

        if captured_output.is_empty() {
            return Err(anyhow!("Empty response from OpenAI API"));
        }

        Ok(captured_output)
    }

    fn name(&self) -> &'static str {
        "openai"
    }
}

/// Helper function to extract text from Claude CLI stream-json format
fn extract_text_from_stream_json(line: &str) -> Vec<String> {
    let mut texts = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some("assistant") = json.get("type").and_then(|t| t.as_str()) {
            if let Some(content) = json
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            {
                for item in content {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        texts.push(text.to_string());
                    }
                }
            }
        }
    }

    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ollama_endpoint() {
        assert_eq!(
            default_ollama_endpoint(),
            "http://localhost:11434/v1".to_string()
        );
    }

    #[test]
    fn test_default_openai_endpoint() {
        assert_eq!(
            default_openai_endpoint(),
            "https://api.openai.com/v1".to_string()
        );
    }

    #[test]
    fn test_claude_provider_name() {
        let provider = ClaudeCliProvider;
        assert_eq!(provider.name(), "claude");
    }

    #[test]
    fn test_ollama_provider_name() {
        let provider = OllamaProvider {
            endpoint: "http://localhost:11434/v1".to_string(),
        };
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_openai_provider_name() {
        let provider = OpenaiProvider {
            endpoint: "https://api.openai.com/v1".to_string(),
            api_key: None,
        };
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_provider_type_default() {
        let provider_type: ProviderType = Default::default();
        assert_eq!(provider_type, ProviderType::Claude);
    }
}
