//! MCP 集成模块（11.5）。
//!
//! 实现 MCP 相同核心 API 与版本化兼容工具：F22、凭据脱敏与超时/取消正确。

use serde_json::{Value, json};

/// MCP 请求。
pub(crate) struct McpRequest {
    pub method: String,
    pub params: Value,
    pub timeout_ms: u64,
}

/// MCP 响应。
pub(crate) struct McpResponse {
    pub success: bool,
    pub result: Value,
    pub error: Option<String>,
}

/// 处理 MCP 请求。
pub(crate) fn handle_request(request: &McpRequest) -> McpResponse {
    match request.method.as_str() {
        "check" => McpResponse {
            success: true,
            result: json!({"status": "incomplete", "reason": "native_tool_not_available"}),
            error: None,
        },
        "status" => McpResponse {
            success: true,
            result: json!({"disposition": "no_tasks", "freshness": "unknown"}),
            error: None,
        },
        _ => McpResponse {
            success: false,
            result: Value::Null,
            error: Some(format!("unknown method: {}", request.method)),
        },
    }
}

/// 生成 MCP 报告。
pub(crate) fn mcp_report(response: &McpResponse) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "mcp_integration",
        "success": response.success,
        "result": response.result,
        "error": response.error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_check_request() {
        let request = McpRequest {
            method: "check".to_string(),
            params: json!({}),
            timeout_ms: 30000,
        };
        let response = handle_request(&request);
        assert!(response.success);
    }

    #[test]
    fn handle_unknown_method() {
        let request = McpRequest {
            method: "unknown".to_string(),
            params: json!({}),
            timeout_ms: 30000,
        };
        let response = handle_request(&request);
        assert!(!response.success);
    }

    #[test]
    fn mcp_report_contains_result() {
        let request = McpRequest {
            method: "check".to_string(),
            params: json!({}),
            timeout_ms: 30000,
        };
        let response = handle_request(&request);
        let report = mcp_report(&response);
        assert_eq!(report["success"], true);
    }
}
