use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::spec::{load_all_specs, resolve_spec, SpecStatus};

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: Value, code: i32, message: &str) -> Self {
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

/// MCP Server info
const SERVER_NAME: &str = "chant";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

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

fn handle_request(line: &str) -> Option<JsonRpcResponse> {
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

fn handle_notification(method: &str, _params: Option<&Value>) {
    // Handle notifications that don't require a response
    match method {
        "notifications/initialized" => {
            // Client is ready
        }
        _ => {
            // Unknown notification, ignore
        }
    }
}

fn handle_method(method: &str, params: Option<&Value>) -> Result<Value> {
    match method {
        "initialize" => handle_initialize(params),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params),
        _ => anyhow::bail!("Method not found: {}", method),
    }
}

fn handle_initialize(_params: Option<&Value>) -> Result<Value> {
    Ok(json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION
        }
    }))
}

fn handle_tools_list() -> Result<Value> {
    Ok(json!({
        "tools": [
            {
                "name": "chant_spec_list",
                "description": "List all chant specs in the current project",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "description": "Filter by status (pending, in_progress, completed, failed)"
                        }
                    }
                }
            },
            {
                "name": "chant_spec_get",
                "description": "Get details of a chant spec",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "chant_spec_update",
                "description": "Update a chant spec status or add output",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Spec ID (full or partial)"
                        },
                        "status": {
                            "type": "string",
                            "description": "New status (pending, in_progress, completed, failed)"
                        },
                        "output": {
                            "type": "string",
                            "description": "Output text to append to spec body"
                        }
                    },
                    "required": ["id"]
                }
            }
        ]
    }))
}

fn handle_tools_call(params: Option<&Value>) -> Result<Value> {
    let params = params.ok_or_else(|| anyhow::anyhow!("Missing params"))?;

    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

    let arguments = params.get("arguments");

    match name {
        "chant_spec_list" => tool_chant_spec_list(arguments),
        "chant_spec_get" => tool_chant_spec_get(arguments),
        "chant_spec_update" => tool_chant_spec_update(arguments),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

fn tool_chant_spec_list(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "Chant not initialized. Run `chant init` first."
                }
            ],
            "isError": true
        }));
    }

    let mut specs = load_all_specs(&specs_dir)?;
    specs.sort_by(|a, b| a.id.cmp(&b.id));

    // Filter by status if provided
    if let Some(args) = arguments {
        if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
            let filter_status = match status_str {
                "pending" => Some(SpecStatus::Pending),
                "in_progress" => Some(SpecStatus::InProgress),
                "completed" => Some(SpecStatus::Completed),
                "failed" => Some(SpecStatus::Failed),
                _ => None,
            };

            if let Some(status) = filter_status {
                specs.retain(|s| s.frontmatter.status == status);
            }
        }
    }

    let specs_json: Vec<Value> = specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "title": s.title,
                "status": format!("{:?}", s.frontmatter.status).to_lowercase(),
                "type": s.frontmatter.r#type,
                "depends_on": s.frontmatter.depends_on,
                "labels": s.frontmatter.labels
            })
        })
        .collect();

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&specs_json)?
            }
        ]
    }))
}

fn tool_chant_spec_get(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "Chant not initialized. Run `chant init` first."
                }
            ],
            "isError": true
        }));
    }

    let id = arguments
        .and_then(|a| a.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let spec_json = json!({
        "id": spec.id,
        "title": spec.title,
        "status": format!("{:?}", spec.frontmatter.status).to_lowercase(),
        "type": spec.frontmatter.r#type,
        "depends_on": spec.frontmatter.depends_on,
        "labels": spec.frontmatter.labels,
        "target_files": spec.frontmatter.target_files,
        "context": spec.frontmatter.context,
        "prompt": spec.frontmatter.prompt,
        "branch": spec.frontmatter.branch,
        "commits": spec.frontmatter.commits,
        "pr": spec.frontmatter.pr,
        "completed_at": spec.frontmatter.completed_at,
        "model": spec.frontmatter.model,
        "body": spec.body
    });

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&spec_json)?
            }
        ]
    }))
}

fn tool_chant_spec_update(arguments: Option<&Value>) -> Result<Value> {
    let specs_dir = PathBuf::from(".chant/specs");

    if !specs_dir.exists() {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "Chant not initialized. Run `chant init` first."
                }
            ],
            "isError": true
        }));
    }

    let args = arguments.ok_or_else(|| anyhow::anyhow!("Missing arguments"))?;

    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;

    let mut spec = match resolve_spec(&specs_dir, id) {
        Ok(s) => s,
        Err(e) => {
            return Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": e.to_string()
                    }
                ],
                "isError": true
            }));
        }
    };

    let mut updated = false;

    // Update status if provided
    if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
        let new_status = match status_str {
            "pending" => SpecStatus::Pending,
            "in_progress" => SpecStatus::InProgress,
            "completed" => SpecStatus::Completed,
            "failed" => SpecStatus::Failed,
            _ => {
                return Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": format!("Invalid status: {}. Must be one of: pending, in_progress, completed, failed", status_str)
                        }
                    ],
                    "isError": true
                }));
            }
        };
        spec.frontmatter.status = new_status;
        updated = true;
    }

    // Append output if provided
    if let Some(output) = args.get("output").and_then(|v| v.as_str()) {
        if !output.is_empty() {
            if !spec.body.ends_with('\n') && !spec.body.is_empty() {
                spec.body.push('\n');
            }
            spec.body.push_str("\n## Output\n\n");
            spec.body.push_str(output);
            spec.body.push('\n');
            updated = true;
        }
    }

    if !updated {
        return Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": "No updates specified. Provide 'status' or 'output' parameter."
                }
            ],
            "isError": true
        }));
    }

    // Save the spec
    let spec_path = specs_dir.join(format!("{}.md", spec.id));
    spec.save(&spec_path)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": format!("Updated spec: {}", spec.id)
            }
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_initialize() {
        let result = handle_initialize(None).unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
    }

    #[test]
    fn test_handle_tools_list() {
        let result = handle_tools_list().unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0]["name"], "chant_spec_list");
        assert_eq!(tools[1]["name"], "chant_spec_get");
        assert_eq!(tools[2]["name"], "chant_spec_update");
    }

    #[test]
    fn test_json_rpc_response_success() {
        let resp = JsonRpcResponse::success(json!(1), json!({"test": true}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse::error(json!(1), -32600, "Invalid request");
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32600);
    }
}
