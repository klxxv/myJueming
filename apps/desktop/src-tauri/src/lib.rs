//! Tauri application wiring; IPC adapters and session state live in dedicated modules.

mod agent_runtime;
mod commands;
mod connection;
mod state;
use jueming_agent_runtime::AgentRuntimePort;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppKernelState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let host = app.state::<state::AppKernelState>().host.clone();
            let research_bundle = app.path().resource_dir()?.join("resources/research");
            #[cfg(debug_assertions)]
            let research_bundle = std::env::var_os("JUEMING_RESEARCH_BUNDLE")
                .map(std::path::PathBuf::from)
                .unwrap_or(research_bundle);
            host.configure_research(app.path().app_data_dir()?, research_bundle)?;
            let mut events = host.subscribe();
            let app_handle = app.handle().clone();
            let event_host = host.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match events.recv().await {
                        Ok(event) => {
                            if event.kind == "project_changed" {
                                let _ = app_handle.emit(
                                    "agent_runtime_resync",
                                    serde_json::json!({
                                        "reason": "project_changed",
                                        "project_id": event.payload.get("project_id"),
                                    }),
                                );
                            }
                            let _ = app_handle.emit("agent_subscribe", event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            if let Ok(projection) = event_host.projection() {
                                let event = jueming_application::AppEvent {
                                    contract_version: "1.0".into(),
                                    sequence: projection.sequence,
                                    kind: "operation_changed".into(),
                                    binding_id: None,
                                    origin: "native".into(),
                                    payload: serde_json::json!({"resync_required": true}),
                                };
                                let _ = app_handle.emit("agent_subscribe", event);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
            // Session-only random secret; never persisted to project or settings.
            let token = jueming_agent_transport::AuthToken::new(format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            ))?;
            let server = tauri::async_runtime::block_on(jueming_agent_transport::start_server(
                host.clone(),
                jueming_agent_transport::ServerConfig::disabled(token.clone()),
            ))?;
            app.manage(connection::AgentConnectionState { server, token });
            let runtime = app
                .path()
                .app_data_dir()
                .map_err(|_| "app data directory is unavailable")
                .and_then(|app_data| {
                    jueming_agent_runtime::initialize(host, app_data)
                        .map_err(|_| "embedded agent runtime initialization failed")
                });
            match runtime {
                Ok(runtime) => {
                    let recovered = runtime.status()?.recovered_interrupted_runs;
                    let mut runtime_events = runtime.subscribe();
                    let runtime_app = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        loop {
                            match runtime_events.recv().await {
                                Ok(event) => {
                                    let _ = runtime_app.emit("agent_runtime_event", event);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                    let _ = runtime_app.emit(
                                        "agent_runtime_resync",
                                        serde_json::json!({"reason": "event_gap"}),
                                    );
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            }
                        }
                    });
                    if recovered > 0 {
                        let _ = app.handle().emit(
                            "agent_runtime_resync",
                            serde_json::json!({"reason": "runtime_recovered"}),
                        );
                    }
                    app.manage(agent_runtime::AgentRuntimeState::available(runtime));
                }
                Err(_) => {
                    app.manage(agent_runtime::AgentRuntimeState::unavailable());
                }
            };
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::canonical::execute_command,
            commands::canonical::load_parallel_slice,
            commands::agent::agent_call,
            commands::agent::agent_projection,
            connection::agent_connection_status,
            connection::agent_connection_set_enabled,
            connection::agent_connection_config,
            agent_runtime::agent_runtime_status,
            agent_runtime::agent_runtime_configure,
            agent_runtime::agent_runtime_start,
            agent_runtime::agent_runtime_cancel,
            agent_runtime::agent_runtime_history,
            agent_runtime::agent_runtime_sessions,
            commands::import::preview_import,
            commands::import::list_supported_languages,
            commands::project::create_project,
            commands::project::open_project,
            commands::project::get_project_summary,
            commands::project::get_current_project,
            commands::project::flush_project,
            commands::project::clear_cache,
            commands::history::list_revisions,
            commands::history::compare_revision,
            commands::search::search_segments,
            commands::search::preview_replace,
            commands::bookmark::list_bookmarks,
            commands::annotation::list_annotations,
            commands::export::export_project,
            commands::settings::load_app_settings,
            commands::settings::save_app_settings,
            commands::settings::reset_app_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
