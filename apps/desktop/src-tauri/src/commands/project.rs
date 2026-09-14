//! Project IPC adapters backed by the authoritative LocalAppHost.

use crate::state::AppKernelState;
use jueming_protocol::{CreateProjectRequest, ProjectSummary, WorkspaceProject};
use tauri::State;

#[tauri::command]
pub(crate) fn create_project(
    request: CreateProjectRequest,
    state: State<'_, AppKernelState>,
) -> Result<WorkspaceProject, String> {
    state
        .host
        .create_project(&request)
        .and_then(|snapshot| {
            jueming_kernel::KernelService
                .workspace_project(&snapshot)
                .map_err(|e| jueming_application::AppError::new("invalid_project", e.to_string()))
        })
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) fn open_project(
    project_path: String,
    state: State<'_, AppKernelState>,
) -> Result<WorkspaceProject, String> {
    state
        .host
        .open_project(project_path)
        .and_then(|snapshot| {
            jueming_kernel::KernelService
                .workspace_project(&snapshot)
                .map_err(|e| jueming_application::AppError::new("invalid_project", e.to_string()))
        })
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) fn get_project_summary(
    state: State<'_, AppKernelState>,
) -> Result<ProjectSummary, String> {
    state
        .host
        .read(|kernel, snapshot, _| kernel.summarize(snapshot))
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) fn get_current_project(
    state: State<'_, AppKernelState>,
) -> Result<WorkspaceProject, String> {
    state
        .host
        .read(|k, s, _| k.workspace_project(s))
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) fn flush_project(state: State<'_, AppKernelState>) -> Result<(), String> {
    state.host.flush().map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) async fn clear_cache(state: State<'_, AppKernelState>) -> Result<u64, String> {
    let host = state.host.clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.clear_cache().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The cache cleanup worker failed: {error}"))?
}
