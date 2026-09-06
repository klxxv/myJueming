//! Alignment commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::{AlignmentGapEdge, AlignmentId, ProjectSnapshot, SegmentId};
use tauri::State;
#[tauri::command]
pub(crate) fn insert_alignment_gap(
    segment_id: SegmentId,
    edge: AlignmentGapEdge,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.insert_alignment_gap(p, s, segment_id, edge)
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn link_segments(
    source_segment_ids: Vec<SegmentId>,
    target_segment_ids: Vec<SegmentId>,
    replace_existing: Option<bool>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.link_segments(
                p,
                s,
                source_segment_ids,
                target_segment_ids,
                replace_existing.unwrap_or(false),
            )
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn unlink_alignment(
    alignment_id: AlignmentId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.unlink_alignment(p, s, alignment_id))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn merge_alignments(
    alignment_ids: Vec<AlignmentId>,
    unlinked_segment_ids: Option<Vec<SegmentId>>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.merge_alignments(
                p,
                s,
                alignment_ids,
                unlinked_segment_ids.unwrap_or_default(),
            )
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn group_alignment(
    alignment_ids: Vec<AlignmentId>,
    unlinked_segment_ids: Vec<SegmentId>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.group_alignment(p, s, alignment_ids, unlinked_segment_ids)
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn split_alignment(
    alignment_id: AlignmentId,
    source_groups: Vec<Vec<SegmentId>>,
    target_groups: Vec<Vec<SegmentId>>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.split_alignment(p, s, alignment_id, source_groups, target_groups)
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn ungroup_alignment(
    alignment_id: AlignmentId,
    source_groups: Vec<Vec<SegmentId>>,
    target_groups: Vec<Vec<SegmentId>>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.ungroup_alignment(p, s, alignment_id, source_groups, target_groups)
        })
        .map_err(|e| e.to_string())
}
