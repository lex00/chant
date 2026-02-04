//! Model Context Protocol (MCP) server implementation.
//!
//! # Doc Audit
//! - audited: 2026-01-25
//! - docs: reference/mcp.md
//! - ignore: false

pub mod handlers;
pub mod protocol;
pub mod server;

pub use handlers::{handle_method, handle_notification};
pub use server::run_server;

#[cfg(test)]
mod tests {
    use super::handlers::{handle_method, handle_notification};
    use super::protocol::{JsonRpcResponse, PROTOCOL_VERSION, SERVER_NAME};
    use serde_json::json;

    #[test]
    fn test_handle_initialize() {
        let result = handle_method("initialize", None).unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
    }

    #[test]
    fn test_handle_tools_list() {
        let result = handle_method("tools/list", None).unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 18);
        // Query tools (7)
        assert_eq!(tools[0]["name"], "chant_spec_list");
        assert_eq!(tools[1]["name"], "chant_spec_get");
        assert_eq!(tools[2]["name"], "chant_ready");
        assert_eq!(tools[3]["name"], "chant_status");
        assert_eq!(tools[4]["name"], "chant_log");
        assert_eq!(tools[5]["name"], "chant_search");
        assert_eq!(tools[6]["name"], "chant_diagnose");
        // Mutating tools (11)
        assert_eq!(tools[7]["name"], "chant_spec_update");
        assert_eq!(tools[8]["name"], "chant_add");
        assert_eq!(tools[9]["name"], "chant_finalize");
        assert_eq!(tools[10]["name"], "chant_resume");
        assert_eq!(tools[11]["name"], "chant_cancel");
        assert_eq!(tools[12]["name"], "chant_archive");
        assert_eq!(tools[13]["name"], "chant_verify");
        assert_eq!(tools[14]["name"], "chant_work_start");
        assert_eq!(tools[15]["name"], "chant_work_list");
        assert_eq!(tools[16]["name"], "chant_pause");
        assert_eq!(tools[17]["name"], "chant_takeover");
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

    #[test]
    fn test_chant_status_schema_has_brief_and_activity() {
        let result = handle_method("tools/list", None).unwrap();
        let tools = result["tools"].as_array().unwrap();
        let status_tool = tools.iter().find(|t| t["name"] == "chant_status").unwrap();

        let props = &status_tool["inputSchema"]["properties"];
        assert!(
            props.get("brief").is_some(),
            "chant_status should have 'brief' property"
        );
        assert!(
            props.get("include_activity").is_some(),
            "chant_status should have 'include_activity' property"
        );

        // Check descriptions
        assert!(props["brief"]["description"]
            .as_str()
            .unwrap()
            .contains("single-line"));
        assert!(props["include_activity"]["description"]
            .as_str()
            .unwrap()
            .contains("activity"));
    }

    #[test]
    fn test_chant_ready_has_limit_param() {
        let result = handle_method("tools/list", None).unwrap();
        let tools = result["tools"].as_array().unwrap();
        let ready_tool = tools.iter().find(|t| t["name"] == "chant_ready").unwrap();

        let props = &ready_tool["inputSchema"]["properties"];
        assert!(
            props.get("limit").is_some(),
            "chant_ready should have 'limit' property"
        );
        assert_eq!(props["limit"]["type"], "integer");
        assert!(props["limit"]["description"]
            .as_str()
            .unwrap()
            .contains("50"));
    }

    #[test]
    fn test_chant_spec_list_has_limit_param() {
        let result = handle_method("tools/list", None).unwrap();
        let tools = result["tools"].as_array().unwrap();
        let list_tool = tools
            .iter()
            .find(|t| t["name"] == "chant_spec_list")
            .unwrap();

        let props = &list_tool["inputSchema"]["properties"];
        assert!(
            props.get("limit").is_some(),
            "chant_spec_list should have 'limit' property"
        );
        assert_eq!(props["limit"]["type"], "integer");
        assert!(props["limit"]["description"]
            .as_str()
            .unwrap()
            .contains("50"));
    }

    #[test]
    fn test_handle_notification() {
        // Should not panic
        handle_notification("notifications/initialized", None);
        handle_notification("unknown_notification", None);
    }
}
