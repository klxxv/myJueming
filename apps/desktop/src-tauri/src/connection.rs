//! Native settings control the session-scoped external bridge.
use jueming_agent_transport::{AuthToken, ServerHandle, ServerStatus};
use serde::Serialize;
use serde_json::json;
use tauri::State;

pub(crate) struct AgentConnectionState {
    pub(crate) server: ServerHandle,
    pub(crate) token: AuthToken,
}

#[derive(Serialize)]
pub(crate) struct ConnectionInfo {
    status: ServerStatus,
    mcp_command: Option<String>,
}

fn discover_mcp_binary() -> Option<String> {
    let executable = std::env::current_exe().ok()?;
    let binary = executable.with_file_name(if cfg!(windows) {
        "jueming-mcp.exe"
    } else {
        "jueming-mcp"
    });
    binary
        .is_file()
        .then(|| binary.to_string_lossy().into_owned())
}

#[tauri::command]
pub(crate) fn agent_connection_status(state: State<'_, AgentConnectionState>) -> ConnectionInfo {
    ConnectionInfo {
        status: state.server.status(),
        mcp_command: discover_mcp_binary(),
    }
}

#[tauri::command]
pub(crate) async fn agent_connection_set_enabled(
    enabled: bool,
    state: State<'_, AgentConnectionState>,
) -> Result<ConnectionInfo, String> {
    let status = state
        .server
        .set_enabled(enabled)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ConnectionInfo {
        status,
        mcp_command: discover_mcp_binary(),
    })
}

/// Called only by the explicit native Settings "copy configuration" action.
/// The secret never enters general settings, events, `.jm`, or log output.
#[tauri::command]
pub(crate) fn agent_connection_config(
    command: String,
    state: State<'_, AgentConnectionState>,
) -> Result<String, String> {
    let status = state.server.status();
    let address = status.bound_addr.ok_or("请先启用本次会话的外部连接")?;
    if !status.enabled {
        return Err("外部连接已关闭".into());
    }
    let path = std::path::Path::new(&command);
    if !path.is_absolute() || !path.is_file() {
        return Err("请选择实际的 jueming-mcp 可执行文件".into());
    }
    Ok(json!({"mcpServers":{"jueming":{"command":command,"env":{"JUEMING_AGENT_ENDPOINT":format!("http://{address}"),"JUEMING_AGENT_TOKEN":state.token.expose_to_sidecar()}}}}).to_string())
}
