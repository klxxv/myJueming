//! Alignment IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{AlignmentGapEdge, AlignmentId, ProjectSnapshot, SegmentId};
use tauri::State;

#[tauri::command]
pub(crate) fn insert_alignment_gap(
    segment_id: SegmentId,
    edge: AlignmentGapEdge,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .insert_alignment_gap(&current.path, &current.snapshot, segment_id, edge)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn link_segments(
    source_segment_ids: Vec<SegmentId>,
    target_segment_ids: Vec<SegmentId>,
    replace_existing: Option<bool>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .link_segments(
            &current.path,
            &current.snapshot,
            source_segment_ids,
            target_segment_ids,
            replace_existing.unwrap_or(false),
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn unlink_alignment(
    alignment_id: AlignmentId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .unlink_alignment(&current.path, &current.snapshot, alignment_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn merge_alignments(
    alignment_ids: Vec<AlignmentId>,
    unlinked_segment_ids: Option<Vec<SegmentId>>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .merge_alignments(
            &current.path,
            &current.snapshot,
            alignment_ids,
            unlinked_segment_ids.unwrap_or_default(),
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn group_alignment(
    alignment_ids: Vec<AlignmentId>,
    unlinked_segment_ids: Vec<SegmentId>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .group_alignment(
            &current.path,
            &current.snapshot,
            alignment_ids,
            unlinked_segment_ids,
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

/// Transitional IPC alias. New clients call `group_alignment`.
#[tauri::command]
pub(crate) fn split_alignment(
    alignment_id: AlignmentId,
    source_groups: Vec<Vec<SegmentId>>,
    target_groups: Vec<Vec<SegmentId>>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .split_alignment(
            &current.path,
            &current.snapshot,
            alignment_id,
            source_groups,
            target_groups,
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn ungroup_alignment(
    alignment_id: AlignmentId,
    source_groups: Vec<Vec<SegmentId>>,
    target_groups: Vec<Vec<SegmentId>>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .ungroup_alignment(
            &current.path,
            &current.snapshot,
            alignment_id,
            source_groups,
            target_groups,
        )
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}
