//! Annotation sidecar commands serialized by LocalAppHost.
use crate::state::AppKernelState;
use jueming_protocol::{
    AnnotationCreateRequest, AnnotationId, AnnotationUpdateRequest, HumanAnnotation,
    ProjectSnapshot,
};
use tauri::State;
#[tauri::command]
pub(crate) fn create_annotation(
    request: AnnotationCreateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.create_annotation(p, s, request))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn list_annotations(
    state: State<'_, AppKernelState>,
) -> Result<Vec<HumanAnnotation>, String> {
    state
        .host
        .read(|k, s, _| k.list_annotations(s))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn update_annotation(
    request: AnnotationUpdateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.update_annotation(p, s, request))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn delete_annotation(
    annotation_id: AnnotationId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| k.delete_annotation(p, s, annotation_id))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn resolve_annotation(
    annotation_id: AnnotationId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    state
        .host
        .mutate("native", |k, s, p| {
            k.resolve_annotation(p, s, annotation_id)
        })
        .map_err(|e| e.to_string())
}
