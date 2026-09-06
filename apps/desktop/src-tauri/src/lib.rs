//! Tauri application wiring; IPC adapters and session state live in dedicated modules.

mod commands;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppKernelState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::import::preview_import,
            commands::import::list_supported_languages,
            commands::project::create_project,
            commands::project::open_project,
            commands::project::get_project_summary,
            commands::project::get_current_project,
            commands::project::flush_project,
            commands::project::clear_cache,
            commands::segment::update_segment,
            commands::segment::move_segment,
            commands::segment::reorder_segments,
            commands::alignment::insert_alignment_gap,
            commands::alignment::link_segments,
            commands::alignment::unlink_alignment,
            commands::alignment::merge_alignments,
            commands::alignment::group_alignment,
            commands::alignment::split_alignment,
            commands::alignment::ungroup_alignment,
            commands::segment::merge_segments,
            commands::segment::split_segment,
            commands::history::list_revisions,
            commands::history::compare_revision,
            commands::history::undo,
            commands::history::redo,
            commands::history::restore_revision,
            commands::search::search_segments,
            commands::search::preview_replace,
            commands::search::apply_replace,
            commands::bookmark::create_bookmark,
            commands::bookmark::list_bookmarks,
            commands::bookmark::update_bookmark,
            commands::bookmark::delete_bookmark,
            commands::annotation::create_annotation,
            commands::annotation::list_annotations,
            commands::annotation::update_annotation,
            commands::annotation::delete_annotation,
            commands::annotation::resolve_annotation,
            commands::export::export_project,
            commands::settings::load_app_settings,
            commands::settings::save_app_settings,
            commands::settings::reset_app_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
