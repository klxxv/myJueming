//! Annotation sidecar commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::HumanAnnotation;
use tauri::State;
#[tauri::command]
pub(crate) fn list_annotations(
    state: State<'_, AppKernelState>,
) -> Result<Vec<HumanAnnotation>, String> {
    state
        .host
        .read(|k, s, _| k.list_annotations(s))
        .map_err(|e| e.to_string())
}
