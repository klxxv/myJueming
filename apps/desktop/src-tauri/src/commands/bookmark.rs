//! Bookmark IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{BookmarkCreateRequest, BookmarkUpdateRequest, ProjectSnapshot};
use tauri::State;

#[tauri::command]
pub(crate) fn create_bookmark(
    request: BookmarkCreateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .create_bookmark(&current.path, &current.snapshot, request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn list_bookmarks(
    state: State<'_, AppKernelState>,
) -> Result<Vec<jueming_protocol::BookmarkPreview>, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .list_bookmarks(&current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_bookmark(
    request: BookmarkUpdateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .update_bookmark(&current.path, &current.snapshot, request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn delete_bookmark(
    bookmark_id: jueming_protocol::BookmarkId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .delete_bookmark(&current.path, &current.snapshot, bookmark_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}
