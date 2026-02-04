//! MCP server main loop and request handling.

use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};

use super::handlers::{handle_method, handle_notification};
use super::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Run the MCP server, reading from stdin and writing to stdout.
pub fn run_server() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line.context("Failed to read from stdin")?;

        if line.trim().is_empty() {
            continue;
        }

        let response = handle_request(&line);

        if let Some(resp) = response {
            let output = serde_json::to_string(&resp)?;
            writeln!(stdout, "{}", output)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

/// Handle a single JSON-RPC request line.
///
/// # Request Processing
///
/// 1. Parse JSON-RPC 2.0 request from the line
/// 2. Validate `jsonrpc` field is `"2.0"`
/// 3. Dispatch to appropriate handler based on `method`
/// 4. Return response or `None` for notifications
///
/// # Error Handling
///
/// - **Parse Error (-32700)**: JSON is invalid or malformed
/// - **Invalid Version (-32600)**: `jsonrpc` field is not `"2.0"`
/// - **Server Error (-32603)**: Handler function returns `Err`
/// - **No Response**: Notifications (requests without `id`) are handled silently
///
/// # Returns
///
/// - `Some(response)`: For requests (with `id`)
/// - `None`: For notifications (without `id`)
pub fn handle_request(line: &str) -> Option<JsonRpcResponse> {
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            return Some(JsonRpcResponse::error(
                Value::Null,
                -32700,
                &format!("Parse error: {}", e),
            ));
        }
    };

    // Validate jsonrpc version
    if request.jsonrpc != "2.0" {
        return Some(JsonRpcResponse::error(
            request.id.unwrap_or(Value::Null),
            -32600,
            "Invalid JSON-RPC version",
        ));
    }

    // Notifications (no id) don't get responses
    let id = match request.id {
        Some(id) => id,
        None => {
            // Handle notification (no response needed)
            handle_notification(&request.method, request.params.as_ref());
            return None;
        }
    };

    let result = handle_method(&request.method, request.params.as_ref());

    match result {
        Ok(value) => Some(JsonRpcResponse::success(id, value)),
        Err(e) => Some(JsonRpcResponse::error(id, -32603, &e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_request_parse_error() {
        let response = handle_request("{invalid json}");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32700);
    }

    #[test]
    fn test_handle_request_invalid_version() {
        let response = handle_request(r#"{"jsonrpc":"1.0","method":"test","id":1}"#);
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    }

    #[test]
    fn test_handle_request_notification() {
        let response = handle_request(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        assert!(response.is_none());
    }
}
