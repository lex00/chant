//! JSON-RPC 2.0 protocol types and MCP constants.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP Server info
pub const SERVER_NAME: &str = "chant";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response
///
/// Represents a JSON-RPC 2.0 response message. Either `result` or `error` will be present,
/// but not both.
///
/// # Success Response
///
/// When the request succeeds, `result` contains the response data and `error` is `None`.
///
/// # Error Response
///
/// When the request fails, `error` contains error details and `result` is `None`.
///
/// # Fields
///
/// - `jsonrpc`: Version string, always `"2.0"`
/// - `result`: Success data (tool result or handler response)
/// - `error`: Error details if request failed
/// - `id`: Request ID from the original request (for correlation)
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

/// JSON-RPC 2.0 Error
///
/// Represents an error in a JSON-RPC response.
///
/// # Error Codes
///
/// - `-32700`: Parse error (invalid JSON)
/// - `-32600`: Invalid JSON-RPC version (jsonrpc != "2.0")
/// - `-32603`: Server error (internal handler error)
///
/// # Fields
///
/// - `code`: JSON-RPC error code (negative integer)
/// - `message`: Human-readable error description
/// - `data`: Optional additional error context
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    /// Create a successful response.
    ///
    /// # Arguments
    ///
    /// - `id`: Request ID to echo back
    /// - `result`: Response data (typically a JSON object)
    ///
    /// # Returns
    ///
    /// A response with `result` set and `error` as `None`.
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response.
    ///
    /// # Arguments
    ///
    /// - `id`: Request ID to echo back
    /// - `code`: JSON-RPC error code (negative integer)
    ///   - `-32700`: Parse error
    ///   - `-32600`: Invalid JSON-RPC version
    ///   - `-32603`: Server error
    /// - `message`: Human-readable error description
    ///
    /// # Returns
    ///
    /// A response with `error` set and `result` as `None`.
    pub fn error(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
            id,
        }
    }
}
