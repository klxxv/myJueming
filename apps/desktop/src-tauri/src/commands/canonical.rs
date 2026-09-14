//! All native canonical writes require the same revision-bound envelope.
use crate::state::AppKernelState;
use jueming_application::AppError;
use jueming_protocol::{CommandEnvelope, CommandResult, ParallelSlice, ParallelSliceRequest};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub(crate) async fn execute_command(
    command: CommandEnvelope<Value>,
    state: State<'_, AppKernelState>,
) -> Result<CommandResult, AppError> {
    let host = state.host.clone();
    tauri::async_runtime::spawn_blocking(move || host.execute_native_command(command))
        .await
        .map_err(|error| AppError::new("transport_error", error.to_string()))?
}

#[tauri::command]
pub(crate) async fn load_parallel_slice(
    request: ParallelSliceRequest,
    state: State<'_, AppKernelState>,
) -> Result<ParallelSlice, AppError> {
    let host = state.host.clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.read(|k, s, _| k.load_parallel_slice(s, &request))
    })
    .await
    .map_err(|error| AppError::new("transport_error", error.to_string()))?
}
