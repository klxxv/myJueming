//! Thin trusted native facade for the optional embedded model runtime.
use jueming_agent_runtime::{
    AgentRuntime, AgentRuntimePort, HistoryRequest, RunHandle, RunRequest, RuntimeConfiguration,
    RuntimeConfigurationInput, RuntimeStatus, SessionHistory, SessionSummary,
};
use std::sync::Arc;
use tauri::State;

/// The embedded model runtime is optional. A local chat-history database failure
/// must not prevent the core desktop application or MCP transport from starting.
pub(crate) struct AgentRuntimeState {
    runtime: Result<Arc<AgentRuntime>, String>,
}

impl AgentRuntimeState {
    pub(crate) fn available(runtime: Arc<AgentRuntime>) -> Self {
        Self {
            runtime: Ok(runtime),
        }
    }

    pub(crate) fn unavailable() -> Self {
        Self {
            runtime: Err(
                "embedded agent runtime is unavailable because its local chat storage could not be initialized"
                    .into(),
            ),
        }
    }

    fn runtime(&self) -> Result<Arc<AgentRuntime>, String> {
        self.runtime.clone()
    }
}

#[tauri::command]
pub(crate) async fn agent_runtime_status(
    runtime: State<'_, AgentRuntimeState>,
) -> Result<RuntimeStatus, String> {
    let runtime = runtime.runtime()?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.status().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn agent_runtime_configure(
    config: RuntimeConfigurationInput,
    runtime: State<'_, AgentRuntimeState>,
) -> Result<RuntimeConfiguration, String> {
    let runtime = runtime.runtime()?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.configure(config).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn agent_runtime_start(
    request: RunRequest,
    runtime: State<'_, AgentRuntimeState>,
) -> Result<RunHandle, String> {
    let runtime = runtime.runtime()?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.start(request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn agent_runtime_cancel(
    run_id: String,
    runtime: State<'_, AgentRuntimeState>,
) -> Result<(), String> {
    let runtime = runtime.runtime()?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.cancel(&run_id).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn agent_runtime_history(
    request: HistoryRequest,
    runtime: State<'_, AgentRuntimeState>,
) -> Result<SessionHistory, String> {
    let runtime = runtime.runtime()?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.history(request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn agent_runtime_sessions(
    project_id: String,
    runtime: State<'_, AgentRuntimeState>,
) -> Result<Vec<SessionSummary>, String> {
    let runtime = runtime.runtime()?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime
            .sessions(&project_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
