//! Segment IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{ProjectSnapshot, SegmentId};
use tauri::State;

#[tauri::command]
pub(crate) fn update_segment(
    segment_id: SegmentId,
    content: String,
    state: State<'_, AppKernelState>,
) -> Result<(), String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .update_segment(&current.path, &current.snapshot, segment_id, &content)
        .map_err(|error| error.to_string())?;
    current.snapshot = next;
    Ok(())
}

#[tauri::command]
pub(crate) fn move_segment(
    segment_id: SegmentId,
    before_segment_id: Option<SegmentId>,
    after_segment_id: Option<SegmentId>,
    state: State<'_, AppKernelState>,
) -> Result<(), String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .move_segment(
            &current.path,
            &current.snapshot,
            segment_id,
            before_segment_id,
            after_segment_id,
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next;
    Ok(())
}

#[tauri::command]
pub(crate) fn reorder_segments(
    ordered_segment_ids: Vec<SegmentId>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .reorder_segments(&current.path, &current.snapshot, ordered_segment_ids)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn merge_segments(
    segment_ids: Vec<SegmentId>,
    merged_content: String,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .merge_segments(
            &current.path,
            &current.snapshot,
            segment_ids,
            &merged_content,
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn split_segment(
    segment_id: SegmentId,
    parts: Vec<String>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .split_segment(&current.path, &current.snapshot, segment_id, parts)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}
