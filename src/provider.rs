use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::io::BufRead;
use std::process::{Command, Stdio};

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

/// Ollama provider (OpenAI-compatible API)
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
        let url = format!("{}/chat/completions", self.endpoint);

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

        let request_body_str = serde_json::to_string(&request_body)?;

        // Validate endpoint URL
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(anyhow!("Invalid endpoint URL: {}", self.endpoint));
        }

        // Use curl command to make the request
        let mut cmd = Command::new("curl");
        cmd.arg("-s")
            .arg("-X")
            .arg("POST")
            .arg(&url)
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(&request_body_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to execute curl. Is it installed and in PATH? {}", e))?;

        let mut captured_output = String::new();

        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);

            for line in reader.lines().map_while(Result::ok) {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str == "[DONE]" {
                        break;
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) =
                                        delta.get("content").and_then(|c| c.as_str())
                                    {
                                        for text_line in content.lines() {
                                            callback(text_line)?;
                                            captured_output.push_str(text_line);
                                            captured_output.push('\n');
                                        }
                                        // Handle inline text without newline
                                        if !content.is_empty() && !content.ends_with('\n') {
                                            captured_output.push_str(content);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let status = child.wait()?;

        if !status.success() {
            // Check if it's a connection error
            if captured_output.is_empty() {
                return Err(anyhow!("Failed to connect to Ollama at {}\n\nOllama does not appear to be running. To fix:\n\n  1. Install Ollama: https://ollama.ai/download\n  2. Start Ollama: ollama serve\n  3. Pull a model: ollama pull {}\n\nOr switch to Claude CLI by removing 'provider: ollama' from .chant/config.md", self.endpoint, model));
            }
        }

        if captured_output.is_empty() {
            return Err(anyhow!(
                "Model '{}' not found. Run: ollama pull {}",
                model,
                model
            ));
        }

        Ok(captured_output)
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

        let request_body_str = serde_json::to_string(&request_body)?;

        // Validate endpoint URL
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            return Err(anyhow!("Invalid endpoint URL: {}", self.endpoint));
        }

        // Use curl command to make the request
        let mut cmd = Command::new("curl");
        cmd.arg("-s")
            .arg("-X")
            .arg("POST")
            .arg(&url)
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-H")
            .arg(format!("Authorization: Bearer {}", api_key))
            .arg("-d")
            .arg(&request_body_str)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to execute curl. Is it installed and in PATH? {}", e))?;

        let mut captured_output = String::new();

        if let Some(stdout) = child.stdout.take() {
            let reader = std::io::BufReader::new(stdout);

            for line in reader.lines().map_while(Result::ok) {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str == "[DONE]" {
                        break;
                    }

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) =
                                        delta.get("content").and_then(|c| c.as_str())
                                    {
                                        for text_line in content.lines() {
                                            callback(text_line)?;
                                            captured_output.push_str(text_line);
                                            captured_output.push('\n');
                                        }
                                        // Handle inline text without newline
                                        if !content.is_empty() && !content.ends_with('\n') {
                                            captured_output.push_str(content);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let status = child.wait()?;

        if !status.success() {
            // Check output for auth error
            if captured_output.contains("401") || captured_output.contains("Unauthorized") {
                return Err(anyhow!(
                    "Authentication failed. Check OPENAI_API_KEY env var"
                ));
            }
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
