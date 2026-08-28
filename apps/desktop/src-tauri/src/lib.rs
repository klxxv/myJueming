use std::sync::Mutex;

use jueming_kernel::KernelService;
use jueming_protocol::{
    AlignmentId, AnnotationCreateRequest, AnnotationUpdateRequest, BookmarkCreateRequest,
    BookmarkUpdateRequest, CreateProjectRequest, ExportRequest, ImportPreviewResponse,
    PreviewImportRequest, ProjectSnapshot, ProjectSummary, ReplaceApplyRequest,
    ReplacePreviewRequest, RevisionComparison, RevisionId, RevisionListResponse,
    SearchSegmentsRequest, SearchSegmentsResponse, SegmentId,
};
use tauri::State;

struct OpenProject {
    path: String,
    snapshot: ProjectSnapshot,
}

#[derive(Default)]
struct AppKernelState {
    current: Mutex<Option<OpenProject>>,
}

fn lock_error() -> String {
    "The local Kernel state is unavailable; restart Jueming Aligner.".into()
}

#[tauri::command]
fn preview_import(request: PreviewImportRequest) -> Result<ImportPreviewResponse, String> {
    KernelService
        .preview_import(&request.input, &request.profile)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_project(
    request: CreateProjectRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let path = request.project_path.clone();
    let snapshot = KernelService
        .create_project(&request)
        .map_err(|error| error.to_string())?;
    *state.current.lock().map_err(|_| lock_error())? = Some(OpenProject {
        path,
        snapshot: snapshot.clone(),
    });
    Ok(snapshot)
}

#[tauri::command]
fn open_project(
    project_path: String,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let snapshot = KernelService
        .open_project(&project_path)
        .map_err(|error| error.to_string())?;
    *state.current.lock().map_err(|_| lock_error())? = Some(OpenProject {
        path: project_path,
        snapshot: snapshot.clone(),
    });
    Ok(snapshot)
}

#[tauri::command]
fn get_project_summary(state: State<'_, AppKernelState>) -> Result<ProjectSummary, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .summarize(&current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_current_project(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    state
        .current
        .lock()
        .map_err(|_| lock_error())?
        .as_ref()
        .map(|current| current.snapshot.clone())
        .ok_or_else(|| "No local project is open.".to_owned())
}

#[tauri::command]
fn flush_project(state: State<'_, AppKernelState>) -> Result<(), String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .save_project(&current.path, &current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn clear_cache(state: State<'_, AppKernelState>) -> Result<u64, String> {
    let path = {
        let guard = state.current.lock().map_err(|_| lock_error())?;
        guard
            .as_ref()
            .map(|current| current.path.clone())
            .ok_or_else(|| "No local project is open.".to_owned())?
    };
    tauri::async_runtime::spawn_blocking(move || {
        KernelService
            .clear_cache(&path)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("The cache cleanup worker failed: {error}"))?
}

#[tauri::command]
fn update_segment(
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
fn move_segment(
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
fn reorder_segments(
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
fn link_segments(
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
fn unlink_alignment(
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
fn merge_alignments(
    alignment_ids: Vec<AlignmentId>,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .merge_alignments(&current.path, &current.snapshot, alignment_ids)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn split_alignment(
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
fn list_revisions(state: State<'_, AppKernelState>) -> Result<RevisionListResponse, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .list_revisions(&current.path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn compare_revision(
    from_revision_id: RevisionId,
    to_revision_id: RevisionId,
    state: State<'_, AppKernelState>,
) -> Result<RevisionComparison, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .compare_revision(&current.path, from_revision_id, to_revision_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn undo(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .undo(&current.path, &current.snapshot)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn redo(state: State<'_, AppKernelState>) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .redo(&current.path, &current.snapshot)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn restore_revision(
    target_revision_id: RevisionId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .restore_revision(&current.path, &current.snapshot, target_revision_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn search_segments(
    request: SearchSegmentsRequest,
    state: State<'_, AppKernelState>,
) -> Result<SearchSegmentsResponse, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .search_segments(&current.snapshot, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_replace(
    request: ReplacePreviewRequest,
    state: State<'_, AppKernelState>,
) -> Result<jueming_protocol::ReplacePreviewResponse, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .preview_replace(&current.snapshot, &request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn apply_replace(
    request: ReplaceApplyRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .apply_replace(&current.path, &current.snapshot, &request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn create_bookmark(
    request: BookmarkCreateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .create_bookmark(&current.path, &current.snapshot, request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn list_bookmarks(
    state: State<'_, AppKernelState>,
) -> Result<Vec<jueming_protocol::Bookmark>, String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .list_bookmarks(&current.snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_bookmark(
    request: BookmarkUpdateRequest,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .update_bookmark(&current.path, &current.snapshot, request)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn delete_bookmark(
    bookmark_id: jueming_protocol::BookmarkId,
    state: State<'_, AppKernelState>,
) -> Result<ProjectSnapshot, String> {
    let mut guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_mut()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    let next = KernelService
        .delete_bookmark(&current.path, &current.snapshot, bookmark_id)
        .map_err(|error| error.to_string())?;
    current.snapshot = next.clone();
    Ok(next)
}

#[tauri::command]
fn create_annotation(
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
fn list_annotations(
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
fn update_annotation(
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
fn delete_annotation(
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
fn resolve_annotation(
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

#[tauri::command]
fn export_project(request: ExportRequest, state: State<'_, AppKernelState>) -> Result<(), String> {
    let guard = state.current.lock().map_err(|_| lock_error())?;
    let current = guard
        .as_ref()
        .ok_or_else(|| "No local project is open.".to_owned())?;
    KernelService
        .export_to_file(&current.snapshot, &request)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppKernelState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            preview_import,
            create_project,
            open_project,
            get_project_summary,
            get_current_project,
            flush_project,
            clear_cache,
            update_segment,
            move_segment,
            reorder_segments,
            link_segments,
            unlink_alignment,
            merge_alignments,
            split_alignment,
            list_revisions,
            compare_revision,
            undo,
            redo,
            restore_revision,
            search_segments,
            preview_replace,
            apply_replace,
            create_bookmark,
            list_bookmarks,
            update_bookmark,
            delete_bookmark,
            create_annotation,
            list_annotations,
            update_annotation,
            delete_annotation,
            resolve_annotation,
            export_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
