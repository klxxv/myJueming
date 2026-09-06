//! Append-only history commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::{ProjectSnapshot, RevisionComparison, RevisionId, RevisionListResponse};
use tauri::State;
#[tauri::command]
pub(crate) fn list_revisions(
    state: State<'_, AppKernelState>,
) -> Result<RevisionListResponse, String> {
    state
        .host
        .read(|k, _, p| k.list_revisions(p))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn compare_revision(
    from_revision_id: RevisionId,
    to_revision_id: RevisionId,
    state: State<'_, AppKernelState>,
) -> Result<RevisionComparison, String> {
    state
        .host
        .read(|k, _, p| k.compare_revision(p, from_revision_id, to_revision_id))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn undo(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.undo(p, s))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn redo(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.redo(p, s))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn restore_revision(
    target_revision_id: RevisionId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.restore_revision(p, s, target_revision_id)
        })
        .map_err(|e| e.to_string())
}
