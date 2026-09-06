//! Search IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{
    ProjectSnapshot, ReplaceApplyRequest, ReplacePreviewRequest, SearchSegmentsRequest,
    SearchSegmentsResponse,
};
use tauri::State;

#[tauri::command]
pub(crate) fn search_segments(
    request: SearchSegmentsRequest,
    state: State<'_, AppKernelState>,
) -> Result<SearchSegmentsResponse, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .search_segments(&current.snapshot, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn preview_replace(
    request: ReplacePreviewRequest,
    state: State<'_, AppKernelState>,
) -> Result<jueming_protocol::ReplacePreviewResponse, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .preview_replace(&current.snapshot, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn apply_replace(
    request: ReplaceApplyRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .apply_replace(&current.path, &current.snapshot, &request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}
