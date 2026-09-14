//! Bookmark sidecar commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::BookmarkPreview;
use tauri::State;
#[tauri::command]
pub(crate) fn list_bookmarks(
    state: State<'_, AppKernelState>,
) -> Result<Vec<BookmarkPreview>, String> {
    state
        .host
        .read(|k, s, _| k.list_bookmarks(s))
        .map_err(|e| e.to_string())
}
