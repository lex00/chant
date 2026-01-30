//! Agent runtime for ollama with function calling support.
//!
//! Uses ureq HTTP client for direct API communication.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::tools;

const MAX_ITERATIONS: usize = 50;
const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";

/// Calculate exponential backoff delay with jitter
fn calculate_backoff(attempt: u32, base_delay_ms: u64) -> u64 {
    let exponential = 2u64.saturating_pow(attempt - 1);
    let delay = base_delay_ms.saturating_mul(exponential);
    // Add jitter: ±10% of delay to avoid thundering herd
    let jitter = (delay / 10).saturating_mul(
        ((attempt as u64).wrapping_mul(7)) % 21 / 10, // Deterministic pseudo-random jitter
    );
    if attempt.is_multiple_of(2) {
        delay.saturating_add(jitter)
    } else {
        delay.saturating_sub(jitter)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Value>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCall {
    id: String,
    function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCallFunction {
    name: String,
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[allow(dead_code)] // Required for serde deserialization completeness
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
}

/// Run an agent loop using direct HTTP with function calling.
///
/// This creates an agent loop where:
/// 1. Model receives a prompt and thinks about what to do
/// 2. Model requests tool execution if needed
/// 3. Runtime executes the tool
/// 4. Runtime feeds result back to model
/// 5. Loop continues until task is complete
pub fn run_agent(
    endpoint: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    callback: &mut dyn FnMut(&str) -> Result<()>,
) -> Result<String> {
    run_agent_with_retries(
        endpoint,
        model,
        system_prompt,
        user_message,
        callback,
        3,
        1000,
    )
}

/// Run an agent loop with configurable retry policy
pub fn run_agent_with_retries(
    endpoint: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    callback: &mut dyn FnMut(&str) -> Result<()>,
    max_retries: u32,
    retry_delay_ms: u64,
) -> Result<String> {
    // Parse endpoint to get base URL
    // Use const for fallback to avoid nested unwrap
    let url = Url::parse(endpoint).unwrap_or_else(|_| {
        Url::parse(DEFAULT_OLLAMA_ENDPOINT).expect("DEFAULT_OLLAMA_ENDPOINT is valid")
    });
    let base_url = format!(
        "{}://{}:{}",
        url.scheme(),
        url.host_str().unwrap_or("localhost"),
        url.port().unwrap_or(11434)
    );
    let chat_url = format!("{}/api/chat", base_url);

    // Build messages
    let mut messages: Vec<Message> = vec![];
    if !system_prompt.is_empty() {
        messages.push(Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });
    }
    messages.push(Message {
        role: "user".to_string(),
        content: user_message.to_string(),
        tool_calls: None,
        tool_call_id: None,
    });

    // Get tool definitions (already in correct format with lowercase "function")
    let tool_defs = tools::get_tool_definitions();

    // Tool calling loop
    let mut iteration = 0;
    let mut final_response = String::new();

    loop {
        iteration += 1;
        if iteration > MAX_ITERATIONS {
            callback(&format!(
                "Warning: Reached max iterations ({})",
                MAX_ITERATIONS
            ))?;
            break;
        }

        // Build request
        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.clone(),
            tools: tool_defs.clone(),
            stream: false,
        };

        // Send request with retry logic
        let mut attempt = 0;
        let chat_response = loop {
            attempt += 1;

            let client = ureq::Agent::new();
            let response = client
                .post(&chat_url)
                .set("Content-Type", "application/json")
                .send_json(&request);

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    // Check for retryable HTTP errors
                    let is_retryable = status == 429
                        || status == 500
                        || status == 502
                        || status == 503
                        || status == 504;

                    if status == 200 {
                        // Success
                        let response_text = resp.into_string()?;
                        match serde_json::from_str::<ChatResponse>(&response_text) {
                            Ok(parsed) => break parsed,
                            Err(e) => {
                                return Err(anyhow!(
                                    "Failed to parse response: {} - body: {}",
                                    e,
                                    response_text
                                ))
                            }
                        }
                    } else if is_retryable && attempt <= max_retries {
                        // Retryable error - wait and retry
                        let delay_ms = calculate_backoff(attempt, retry_delay_ms);
                        callback(&format!(
                            "[Retry {}] HTTP {} - waiting {}ms before retry",
                            attempt, status, delay_ms
                        ))?;
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        continue;
                    } else {
                        // Non-retryable error or max retries exceeded
                        return Err(anyhow!(
                            "HTTP request failed with status {}: {} (after {} attempt{})",
                            status,
                            resp.status_text(),
                            attempt,
                            if attempt == 1 { "" } else { "s" }
                        ));
                    }
                }
                Err(e) => {
                    // Network error - check if retryable
                    let error_str = e.to_string();
                    let is_retryable = error_str.contains("Connection")
                        || error_str.contains("timeout")
                        || error_str.contains("reset");

                    if is_retryable && attempt <= max_retries {
                        let delay_ms = calculate_backoff(attempt, retry_delay_ms);
                        callback(&format!(
                            "[Retry {}] Network error - waiting {}ms before retry: {}",
                            attempt, delay_ms, error_str
                        ))?;
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        continue;
                    } else {
                        return Err(anyhow!("HTTP request failed: {}", e));
                    }
                }
            }
        };

        // Check if model requested tool calls
        if chat_response.message.tool_calls.is_empty() {
            // No tool calls - model has provided final response
            final_response = chat_response.message.content.clone();

            // Buffer content and only call callback when we have complete lines
            let mut line_buffer = String::new();
            for ch in final_response.chars() {
                line_buffer.push(ch);
                if ch == '\n' {
                    let line = line_buffer.trim_end_matches('\n');
                    callback(line)?;
                    line_buffer.clear();
                }
            }

            // Flush any remaining buffered content
            if !line_buffer.is_empty() {
                callback(&line_buffer)?;
            }
            break;
        }

        // Add assistant message with tool calls to history
        messages.push(Message {
            role: "assistant".to_string(),
            content: chat_response.message.content.clone(),
            tool_calls: Some(chat_response.message.tool_calls.clone()),
            tool_call_id: None,
        });

        // Process each tool call from the model
        for tool_call in &chat_response.message.tool_calls {
            let tool_name = &tool_call.function.name;
            let tool_args = &tool_call.function.arguments;

            // Log the tool call
            callback(&format!("[Tool: {}] {}", tool_name, tool_args))?;

            // Execute the tool
            let result = match tools::execute_tool(tool_name, tool_args) {
                Ok(output) => output,
                Err(error) => format!("Error: {}", error),
            };

            // Log abbreviated result
            let result_preview = if result.len() > 200 {
                format!("{}... ({} bytes)", &result[..200], result.len())
            } else {
                result.clone()
            };
            callback(&format!("[Result] {}", result_preview))?;

            // Add tool response to messages
            messages.push(Message {
                role: "tool".to_string(),
                content: result,
                tool_calls: None,
                tool_call_id: Some(tool_call.id.clone()),
            });
        }
    }

    Ok(final_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_initialization() {
        // Test that agent can be created with proper parameters
        let endpoint = "http://localhost:11434";
        let model = "qwen2.5:7b";
        let system_prompt = "You are a helpful assistant.";
        let user_message = "Hello, who are you?";

        let mut callback = |_line: &str| -> Result<()> { Ok(()) };

        // Verify types are correct
        let _endpoint = endpoint;
        let _model = model;
        let _system_prompt = system_prompt;
        let _user_message = user_message;
        let _ = &mut callback;
    }
}
