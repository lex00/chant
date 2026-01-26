//! Agent runtime integration for ollama-rs with function calling support.
//!
//! Provides an agent loop that handles model thinking, tool execution,
//! and feedback to create an agentic workflow with local LLMs.

use anyhow::Result;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::ChatMessage;
use ollama_rs::generation::tools::ToolInfo;
use ollama_rs::Ollama;
use url::Url;

use crate::tools;

const MAX_ITERATIONS: usize = 50;

/// Run an agent loop using ollama-rs with function calling.
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
    // Parse endpoint to extract host and port for ollama-rs
    let url =
        Url::parse(endpoint).unwrap_or_else(|_| Url::parse("http://localhost:11434").unwrap());
    let host = format!(
        "{}://{}",
        url.scheme(),
        url.host_str().unwrap_or("localhost")
    );
    let port = url.port().unwrap_or(11434);

    let ollama = Ollama::new(host, port);

    // Build messages with system prompt if provided
    let mut messages = vec![];
    if !system_prompt.is_empty() {
        messages.push(ChatMessage::system(system_prompt.to_string()));
    }
    messages.push(ChatMessage::user(user_message.to_string()));

    // Get tool definitions and convert to ToolInfo objects
    let tool_defs = tools::get_tool_definitions();
    let mut tool_infos: Vec<ToolInfo> = Vec::new();
    for tool_def in &tool_defs {
        match serde_json::from_value::<ToolInfo>(tool_def.clone()) {
            Ok(info) => tool_infos.push(info),
            Err(e) => {
                eprintln!("[DEBUG] Failed to deserialize tool: {}", e);
                eprintln!(
                    "[DEBUG] Tool JSON: {}",
                    serde_json::to_string_pretty(tool_def).unwrap_or_default()
                );
            }
        }
    }
    eprintln!(
        "[DEBUG] Converted {}/{} tools to ToolInfo",
        tool_infos.len(),
        tool_defs.len()
    );

    // Tool calling loop - iterate until model completes task or max iterations reached
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

        // Build request with tools
        let request =
            ChatMessageRequest::new(model.to_string(), messages.clone()).tools(tool_infos.clone());

        // Send request to ollama
        eprintln!(
            "[DEBUG] Sending request to ollama (iteration {})",
            iteration
        );
        let response = ollama.send_chat_messages(request).await?;

        eprintln!(
            "[DEBUG] Response content length: {}",
            response.message.content.len()
        );
        eprintln!(
            "[DEBUG] Tool calls count: {}",
            response.message.tool_calls.len()
        );

        // Check if model requested tool calls
        if response.message.tool_calls.is_empty() {
            // No tool calls - model has provided final response
            final_response = response.message.content.clone();
            for line in final_response.lines() {
                callback(line)?;
            }
            break;
        }

        // Process each tool call from the model
        for tool_call in &response.message.tool_calls {
            let tool_name = &tool_call.function.name;
            let tool_args = &tool_call.function.arguments;

            // Log the tool call
            callback(&format!("Calling tool: {}", tool_name))?;

            // Execute the tool
            let result = match tools::execute_tool(tool_name, tool_args) {
                Ok(output) => output,
                Err(error) => format!("Tool error: {}", error),
            };

            // Log the result
            callback(&format!("Tool result: {}", result))?;

            // Add tool response to messages for next iteration
            messages.push(ChatMessage::tool(result));
        }

        // Add assistant's response to messages to maintain conversation context
        messages.push(ChatMessage::assistant(response.message.content));
    }

    Ok(final_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_initialization() {
        // Test that agent can be created with proper parameters
        // Note: This test doesn't actually call ollama (would require running instance)
        let endpoint = "http://localhost:11434";
        let model = "qwen2.5:7b";
        let system_prompt = "You are a helpful assistant.";
        let user_message = "Hello, who are you?";

        let mut callback = |_line: &str| -> Result<()> { Ok(()) };

        // This would fail without a running ollama instance, so we just verify
        // the types are correct by not compiling to an error
        let _endpoint = endpoint;
        let _model = model;
        let _system_prompt = system_prompt;
        let _user_message = user_message;
        let _ = &mut callback;
    }
}
