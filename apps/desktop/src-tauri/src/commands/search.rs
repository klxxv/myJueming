//! Search IPC adapters backed by the authoritative LocalAppHost.

use crate::state::AppKernelState;
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
    state
        .host
        .read(|kernel, snapshot, _| kernel.search_segments(snapshot, &request))
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) fn preview_replace(
    request: ReplacePreviewRequest,
    state: State<'_, AppKernelState>,
) -> Result<jueming_protocol::ReplacePreviewResponse, String> {
    state
        .host
        .read(|kernel, snapshot, _| kernel.preview_replace(snapshot, &request))
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub(crate) fn apply_replace(
    request: ReplaceApplyRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |kernel, snapshot, path| {
            kernel.apply_replace(path, snapshot, &request)
        })
        .map_err(|error| error.to_string())
}
