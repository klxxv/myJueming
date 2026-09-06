//! Export IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::ExportRequest;
use tauri::State;

#[tauri::command]
pub(crate) fn export_project(
    request: ExportRequest,
    state: State<'_, AppKernelState>,
) -> Result<(), String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .export_to_file(&current.snapshot, &request)
        .map_err(|error| error.to_string())
}
