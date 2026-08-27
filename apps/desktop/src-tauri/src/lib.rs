use std::sync::Mutex;

use jueming_kernel::KernelService;
use jueming_protocol::{
    CreateProjectRequest, ImportPreviewResponse, PreviewImportRequest, ProjectSnapshot,
    ProjectSummary, SegmentId,
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
            update_segment,
            move_segment,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
