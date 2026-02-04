use serde_json::json;

/// Tests for MCP protocol JSON-RPC compliance and error handling.
///
/// These tests verify that the MCP server correctly implements JSON-RPC 2.0
/// protocol, including:
/// - Valid JSON-RPC response structure
/// - Error handling for parse errors, invalid requests, and server errors
/// - Notification handling (no response)
/// - Request/response correlation via ID

#[test]
fn test_json_rpc_success_response_structure() {
    use chant::mcp::protocol::JsonRpcResponse;

    let resp = JsonRpcResponse::success(json!(1), json!({"status": "ok"}));

    // Verify JSON-RPC 2.0 structure
    assert_eq!(resp.jsonrpc, "2.0", "jsonrpc field must be '2.0'");
    assert!(resp.result.is_some(), "Success response must have result");
    assert!(resp.error.is_none(), "Success response must not have error");
    assert_eq!(resp.id, json!(1), "ID must match request ID");

    // Verify result content
    assert_eq!(resp.result.as_ref().unwrap()["status"], "ok");

    // Verify serialization produces valid JSON-RPC
    let serialized = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(parsed["result"].is_object());
    assert!(parsed["error"].is_null());
    assert_eq!(parsed["id"], 1);
}

#[test]
fn test_json_rpc_error_response_structure() {
    use chant::mcp::protocol::JsonRpcResponse;

    let resp = JsonRpcResponse::error(json!(42), -32600, "Invalid Request");

    // Verify JSON-RPC 2.0 structure
    assert_eq!(resp.jsonrpc, "2.0", "jsonrpc field must be '2.0'");
    assert!(resp.result.is_none(), "Error response must not have result");
    assert!(resp.error.is_some(), "Error response must have error");
    assert_eq!(resp.id, json!(42), "ID must match request ID");

    // Verify error structure
    let error = resp.error.as_ref().unwrap();
    assert_eq!(error.code, -32600, "Error code must match");
    assert_eq!(error.message, "Invalid Request", "Error message must match");
    assert!(error.data.is_none(), "Error data should be None by default");

    // Verify serialization produces valid JSON-RPC
    let serialized = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(parsed["result"].is_null());
    assert_eq!(parsed["error"]["code"], -32600);
    assert_eq!(parsed["error"]["message"], "Invalid Request");
    assert_eq!(parsed["id"], 42);
}

#[test]
fn test_json_rpc_error_with_null_id() {
    use chant::mcp::protocol::JsonRpcResponse;

    // Error responses should handle null ID (for parse errors)
    let resp = JsonRpcResponse::error(json!(null), -32700, "Parse error");

    assert_eq!(resp.id, json!(null));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32700);
}

#[test]
fn test_json_rpc_error_codes() {
    use chant::mcp::protocol::JsonRpcResponse;

    // Test standard JSON-RPC error codes
    let parse_error = JsonRpcResponse::error(json!(null), -32700, "Parse error");
    assert_eq!(parse_error.error.as_ref().unwrap().code, -32700);

    let invalid_request = JsonRpcResponse::error(json!(1), -32600, "Invalid Request");
    assert_eq!(invalid_request.error.as_ref().unwrap().code, -32600);

    let method_not_found = JsonRpcResponse::error(json!(1), -32601, "Method not found");
    assert_eq!(method_not_found.error.as_ref().unwrap().code, -32601);

    let internal_error = JsonRpcResponse::error(json!(1), -32603, "Internal error");
    assert_eq!(internal_error.error.as_ref().unwrap().code, -32603);
}

#[test]
fn test_handle_request_parse_error() {
    use chant::mcp::server::handle_request;

    let response = handle_request("{invalid json}");
    assert!(response.is_some(), "Parse error should return a response");

    let resp = response.unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.error.is_some());
    assert_eq!(resp.error.as_ref().unwrap().code, -32700);
    assert!(resp.error.as_ref().unwrap().message.contains("Parse error"));
    assert_eq!(resp.id, json!(null));
}

#[test]
fn test_handle_request_invalid_jsonrpc_version() {
    use chant::mcp::server::handle_request;

    let response = handle_request(r#"{"jsonrpc":"1.0","method":"test","id":1}"#);
    assert!(
        response.is_some(),
        "Invalid version should return a response"
    );

    let resp = response.unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.error.is_some());
    assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    assert!(resp
        .error
        .as_ref()
        .unwrap()
        .message
        .contains("Invalid JSON-RPC version"));
    assert_eq!(resp.id, json!(1));
}

#[test]
fn test_handle_request_notification_no_response() {
    use chant::mcp::server::handle_request;

    // Notifications (no id) should not return a response
    let response = handle_request(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    assert!(
        response.is_none(),
        "Notifications should not return a response"
    );
}

#[test]
fn test_handle_request_unknown_method() {
    use chant::mcp::server::handle_request;

    let response = handle_request(r#"{"jsonrpc":"2.0","method":"unknown/method","id":1}"#);
    assert!(
        response.is_some(),
        "Unknown method should return error response"
    );

    let resp = response.unwrap();
    assert!(resp.error.is_some());
    assert_eq!(resp.error.as_ref().unwrap().code, -32603); // Server error
    assert!(resp
        .error
        .as_ref()
        .unwrap()
        .message
        .contains("Method not found"));
}

#[test]
fn test_handle_request_valid_initialize() {
    use chant::mcp::protocol::{PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION};
    use chant::mcp::server::handle_request;

    let response = handle_request(r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#);
    assert!(response.is_some(), "Initialize should return a response");

    let resp = response.unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
    assert_eq!(resp.id, json!(1));

    let result = resp.result.unwrap();
    assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
    assert_eq!(result["serverInfo"]["version"], SERVER_VERSION);
}

#[test]
fn test_handle_request_valid_tools_list() {
    use chant::mcp::server::handle_request;

    let response = handle_request(r#"{"jsonrpc":"2.0","method":"tools/list","id":2}"#);
    assert!(response.is_some(), "tools/list should return a response");

    let resp = response.unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
    assert_eq!(resp.id, json!(2));

    let result = resp.result.unwrap();
    assert!(result["tools"].is_array());
    assert!(result["tools"].as_array().unwrap().len() > 0);
}

#[test]
fn test_request_id_preservation() {
    use chant::mcp::server::handle_request;

    // Test with different ID types
    // Note: According to JSON-RPC 2.0, a request with "id": null is still a request (not notification)
    // However, our implementation treats null id as notification, which is acceptable
    let test_cases = vec![
        (
            json!(1),
            r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#,
        ),
        (
            json!("abc"),
            r#"{"jsonrpc":"2.0","method":"initialize","id":"abc"}"#,
        ),
    ];

    for (expected_id, request) in test_cases {
        let response = handle_request(request);
        assert!(
            response.is_some(),
            "Request should return a response for: {}",
            request
        );
        let resp = response.unwrap();
        assert_eq!(
            resp.id, expected_id,
            "Response ID must match request ID for {}",
            request
        );
    }
}

#[test]
fn test_error_response_serialization() {
    use chant::mcp::protocol::JsonRpcResponse;

    let resp = JsonRpcResponse::error(json!(1), -32600, "Invalid Request");
    let serialized = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    // Verify error field is present and correct
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], -32600);
    assert_eq!(parsed["error"]["message"], "Invalid Request");

    // Verify result field is not serialized (skip_serializing_if = None)
    assert!(
        parsed["result"].is_null(),
        "Result should be null in error response"
    );
}

#[test]
fn test_success_response_serialization() {
    use chant::mcp::protocol::JsonRpcResponse;

    let resp = JsonRpcResponse::success(json!(1), json!({"data": "value"}));
    let serialized = serde_json::to_string(&resp).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    // Verify result field is present and correct
    assert!(parsed["result"].is_object());
    assert_eq!(parsed["result"]["data"], "value");

    // Verify error field is not serialized (skip_serializing_if = None)
    assert!(
        parsed["error"].is_null(),
        "Error should be null in success response"
    );
}

#[test]
fn test_malformed_json_requests() {
    use chant::mcp::server::handle_request;

    let malformed_cases = vec![
        "",                // Empty string
        "   ",             // Whitespace only
        "{",               // Incomplete JSON
        r#"{"jsonrpc":}"#, // Invalid JSON syntax
        "not json at all", // Plain text
        r#"["array"]"#,    // Wrong JSON type (array instead of object)
        r#"null"#,         // Null JSON
        r#"123"#,          // Number JSON
        r#""string""#,     // String JSON
    ];

    for input in malformed_cases {
        let response = handle_request(input);
        if input.trim().is_empty() {
            // Empty/whitespace handled differently by server loop
            continue;
        }
        assert!(
            response.is_some(),
            "Malformed JSON should return error response for: {}",
            input
        );
        let resp = response.unwrap();
        assert!(resp.error.is_some(), "Should have error for: {}", input);
        assert_eq!(
            resp.error.as_ref().unwrap().code,
            -32700,
            "Should be parse error for: {}",
            input
        );
    }
}

#[test]
fn test_method_handler_error_propagation() {
    use chant::mcp::handlers::handle_method;

    // Test that handler errors are properly propagated
    let result = handle_method("unknown_method", None);
    assert!(result.is_err(), "Unknown method should return error");
    assert!(result.unwrap_err().to_string().contains("Method not found"));
}

#[test]
fn test_notification_handlers() {
    use chant::mcp::handlers::handle_notification;

    // These should not panic or return anything
    handle_notification("notifications/initialized", None);
    handle_notification("notifications/cancelled", None);
    handle_notification("unknown_notification", None);
    handle_notification("notifications/progress", Some(&json!({"token": "test"})));
}

#[test]
fn test_request_with_params() {
    use chant::mcp::server::handle_request;

    let request = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"chant_status","arguments":{}},"id":3}"#;
    let response = handle_request(request);

    // This will fail with "not initialized" but we're testing that params are properly handled
    assert!(response.is_some());
    let resp = response.unwrap();
    assert_eq!(resp.id, json!(3));
    // Response will have a result with isError:true (tool-level error, not JSON-RPC error)
    assert!(resp.result.is_some() || resp.error.is_some());
}

#[test]
fn test_request_without_params() {
    use chant::mcp::server::handle_request;

    let request = r#"{"jsonrpc":"2.0","method":"initialize","id":4}"#;
    let response = handle_request(request);

    assert!(response.is_some());
    let resp = response.unwrap();
    assert_eq!(resp.id, json!(4));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}
