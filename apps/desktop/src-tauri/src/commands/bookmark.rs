//! Bookmark sidecar commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::{
    BookmarkCreateRequest, BookmarkId, BookmarkPreview, BookmarkUpdateRequest, ProjectSnapshot,
};
use tauri::State;
#[tauri::command]
pub(crate) fn create_bookmark(
    request: BookmarkCreateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.create_bookmark(p, s, request))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn list_bookmarks(
    state: State<'_, AppKernelState>,
) -> Result<Vec<BookmarkPreview>, String> {
    state
        .host
        .read(|k, s, _| k.list_bookmarks(s))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn update_bookmark(
    request: BookmarkUpdateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.update_bookmark(p, s, request))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn delete_bookmark(
    bookmark_id: BookmarkId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.delete_bookmark(p, s, bookmark_id))
        .map_err(|e| e.to_string())
}
