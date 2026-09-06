//! Read-only export through the current LocalAppHost snapshot.
use crate::state::AppKernelState;
use jueming_protocol::ExportRequest;
use tauri::State;
#[tauri::command]
pub(crate) fn export_project(
    request: ExportRequest,
    state: State<'_, AppKernelState>,
) -> Result<(), String> {
    state
        .host
        .read(|k, s, _| k.export_to_file(s, &request))
        .map_err(|e| e.to_string())
}
