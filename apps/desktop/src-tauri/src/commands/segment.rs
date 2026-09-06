//! Segment commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::{ProjectSnapshot, SegmentId};
use tauri::State;
#[tauri::command]
pub(crate) fn update_segment(
    segment_id: SegmentId,
    content: String,
    state: State<'_, AppKernelState>,
) -> Result<(), String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.update_segment(p, s, segment_id, &content)
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn move_segment(
    segment_id: SegmentId,
    before_segment_id: Option<SegmentId>,
    after_segment_id: Option<SegmentId>,
    state: State<'_, AppKernelState>,
) -> Result<(), String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.move_segment(p, s, segment_id, before_segment_id, after_segment_id)
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn reorder_segments(
    ordered_segment_ids: Vec<SegmentId>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.reorder_segments(p, s, ordered_segment_ids)
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn merge_segments(
    segment_ids: Vec<SegmentId>,
    merged_content: String,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.merge_segments(p, s, segment_ids, &merged_content)
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn split_segment(
    segment_id: SegmentId,
    parts: Vec<String>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.split_segment(p, s, segment_id, parts))
        .map_err(|e| e.to_string())
}
