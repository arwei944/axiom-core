//! MCP Server implementation for exposing axiom capabilities as MCP tools.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::State, routing::post, serve, Json, Router};
use tokio::net::TcpListener;

use crate::protocol::{McpCapability, McpError, McpRequest, McpResponse};
use crate::tools::ToolRegistry;

#[derive(Clone)]
pub struct McpServerState {
    registry: Arc<ToolRegistry>,
    capability: McpCapability,
}

pub struct McpServer {
    state: McpServerState,
    addr: SocketAddr,
}

impl McpServer {
    pub fn new(registry: Arc<ToolRegistry>, addr: SocketAddr) -> Self {
        let tools = registry.list();
        let capability = McpCapability {
            name: "axiom-kernel".to_string(),
            protocol_version: "1.0.0".to_string(),
            tools,
            description: Some("Axiom Kernel runtime capabilities exposed as MCP tools".to_string()),
        };

        Self {
            state: McpServerState {
                registry,
                capability,
            },
            addr,
        }
    }

    pub async fn serve(self) -> Result<(), std::io::Error> {
        let app = Router::new()
            .route("/capabilities", post(capabilities_handler))
            .route("/tool", post(tool_handler))
            .route("/shutdown", post(shutdown_handler))
            .with_state(self.state);

        let listener = TcpListener::bind(self.addr).await?;
        serve(listener, app).await
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn capability(&self) -> &McpCapability {
        &self.state.capability
    }
}

async fn capabilities_handler(State(state): State<McpServerState>) -> Json<McpResponse> {
    Json(McpResponse::Capabilities(state.capability))
}

async fn tool_handler(
    State(state): State<McpServerState>,
    Json(request): Json<McpRequest>,
) -> Json<McpResponse> {
    let McpRequest::ToolCall(tool_call) = request else {
        return Json(McpResponse::Error(
            McpError::InvalidRequest("expected ToolCall request".into()).into(),
        ));
    };

    let result = state
        .registry
        .execute(&tool_call.tool_name, &tool_call.arguments)
        .await;

    Json(McpResponse::ToolResult(crate::protocol::McpToolResult {
        id: tool_call.id,
        result: result.map_err(|e| e.into()),
    }))
}

async fn shutdown_handler() -> Json<McpResponse> {
    Json(McpResponse::Shutdown)
}
