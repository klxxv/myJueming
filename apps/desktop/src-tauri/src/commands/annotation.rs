//! Annotation IPC adapters; domain work stays in KernelService.

use crate::state::{AppKernelState, lock_error};
use jueming_kernel::KernelService;
use jueming_protocol::{AnnotationCreateRequest, AnnotationUpdateRequest, ProjectSnapshot};
use tauri::State;

#[tauri::command]
pub(crate) fn create_annotation(
    request: AnnotationCreateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .create_annotation(&current.path, &current.snapshot, request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn list_annotations(
    state: State<'_, AppKernelState>,
) -> Result<Vec<jueming_protocol::HumanAnnotation>, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .list_annotations(&current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_annotation(
    request: AnnotationUpdateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .update_annotation(&current.path, &current.snapshot, request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn delete_annotation(
    annotation_id: jueming_protocol::AnnotationId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .delete_annotation(&current.path, &current.snapshot, annotation_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
pub(crate) fn resolve_annotation(
    annotation_id: jueming_protocol::AnnotationId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .resolve_annotation(&current.path, &current.snapshot, annotation_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}
