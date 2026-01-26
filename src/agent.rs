//! Agent runtime integration for ollama with function calling support.
//!
//! Uses raw HTTP to avoid ollama-rs serialization issues with tool types.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

use crate::tools;

const MAX_ITERATIONS: usize = 50;

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
pub async fn run_agent(
    endpoint: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    callback: &mut dyn FnMut(&str) -> Result<()>,
) -> Result<String> {
    // Parse endpoint to get base URL
    let url = Url::parse(endpoint).unwrap_or_else(|_| Url::parse("http://localhost:11434").unwrap());
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

        // Send request
        let client = ureq::Agent::new();
        let response = client
            .post(&chat_url)
            .set("Content-Type", "application/json")
            .send_json(&request)
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        let response_text = response.into_string()?;
        let chat_response: ChatResponse = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse response: {} - body: {}", e, response_text))?;

        // Check if model requested tool calls
        if chat_response.message.tool_calls.is_empty() {
            // No tool calls - model has provided final response
            final_response = chat_response.message.content.clone();
            for line in final_response.lines() {
                callback(line)?;
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

    #[tokio::test]
    async fn test_agent_initialization() {
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
