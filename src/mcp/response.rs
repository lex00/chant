//! MCP response helpers for constructing consistent JSON responses.

use serde_json::{json, Value};

/// Create an MCP text response.
///
/// This is the standard format for successful tool responses in MCP.
pub fn mcp_text_response(text: impl Into<String>) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text.into()
            }
        ]
    })
}

/// Create an MCP error response.
///
/// This format is used for tool-level errors (not JSON-RPC protocol errors).
pub fn mcp_error_response(text: impl Into<String>) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text.into()
            }
        ],
        "isError": true
    })
}
