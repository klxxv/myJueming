//! History IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{ProjectSnapshot, RevisionComparison, RevisionId, RevisionListResponse};
use tauri::State;

#[tauri::command]
pub(crate) fn list_revisions(
    state: State<'_, AppKernelState>,
) -> Result<RevisionListResponse, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .list_revisions(&current.path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn compare_revision(
    from_revision_id: RevisionId,
    to_revision_id: RevisionId,
    state: State<'_, AppKernelState>,
) -> Result<RevisionComparison, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .compare_revision(&current.path, from_revision_id, to_revision_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn undo(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .undo(&current.path, &current.snapshot)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn redo(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .redo(&current.path, &current.snapshot)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn restore_revision(
    target_revision_id: RevisionId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .restore_revision(&current.path, &current.snapshot, target_revision_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}
