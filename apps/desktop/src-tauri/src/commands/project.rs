//! Project IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, OpenProject, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{CreateProjectRequest, ProjectSnapshot, ProjectSummary};
use tauri::State;

#[tauri::command]
pub(crate) fn create_project(
    request: CreateProjectRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let path = request.project_path.clone();
    let snapshot = KernelService
        .create_project(&request)
        .map_err(|error| error.to_string())?;
    *state.current.lock().map_err(|_| lock_error())? = Some(OpenProject {
        path,
        snapshot: snapshot.clone(),
    });
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn open_project(
    project_path: String,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let snapshot = KernelService
        .open_project(&project_path)
        .map_err(|error| error.to_string())?;
    *state.current.lock().map_err(|_| lock_error())? = Some(OpenProject {
        path: project_path,
        snapshot: snapshot.clone(),
    });
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn get_project_summary(
    state: State<'_, AppKernelState>,
) -> Result<ProjectSummary, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .summarize(&current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn get_current_project(
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .current
        .lock()
        .map_err(|_| lock_error())?
        .as_ref()
        .map(|current| current.snapshot.clone())
        .ok_or_else(|| "No local project is open.".to_owned())
}

#[tauri::command]
pub(crate) fn flush_project(state: State<'_, AppKernelState>) -> Result<(), String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .save_project(&current.path, &current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) async fn clear_cache(state: State<'_, AppKernelState>) -> Result<u64, String> {
    let path = {
        let guard = state.current.lock().map_err(|_| lock_error())?;
        guard
            .as_ref()
            .map(|current| current.path.clone())
            .ok_or_else(|| "No local project is open.".to_owned())?
    };
    tauri::async_runtime::spawn_blocking(move || {
        KernelService
            .clear_cache(&path)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The cache cleanup worker failed: {error}"))?
}
