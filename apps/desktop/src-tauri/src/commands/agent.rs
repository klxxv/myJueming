//! Trusted native AG-UI command boundary.
//!
//! `agent_call` is only registered in the local Tauri invoke handler. External
//! MCP transports receive `Arc<LocalAppHost>` and must call `dispatch`, which
//! rejects `proposal.approve`; this command deliberately uses `dispatch_native`.

use crate::state::AppKernelState;
use jueming_application::{AgentCall, AgentReply};
use tauri::State;

#[tauri::command]
pub(crate) async fn agent_call(
    call: AgentCall,
    state: State<'_, AppKernelState>,
) -> Result<AgentReply, String> {
    let host = state.host.clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.dispatch_native(call)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The agent gateway worker failed: {error}"))?
}

#[tauri::command]
pub(crate) async fn agent_projection(
    state: State<'_, AppKernelState>,
) -> Result<AgentReply, String> {
    let host = state.host.clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.projection().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The agent projection worker failed: {error}"))?
}
