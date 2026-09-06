//! Device-scoped application settings persisted outside `.jm` projects.

use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const SETTINGS_STORE_PATH: &str = "settings.json";
const SETTINGS_KEY: &str = "app-settings";

#[tauri::command]
pub(crate) fn load_app_settings(app: AppHandle) -> Result<Option<Value>, String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("Unable to open the local settings store: {error}"))?;
    Ok(store.get(SETTINGS_KEY))
}

#[tauri::command]
pub(crate) fn save_app_settings(app: AppHandle, settings: Value) -> Result<(), String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("Unable to open the local settings store: {error}"))?;
    store.set(SETTINGS_KEY, settings);
    store
        .save()
        .map_err(|error| format!("Unable to save local settings: {error}"))
}

#[tauri::command]
pub(crate) fn reset_app_settings(app: AppHandle) -> Result<(), String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|error| format!("Unable to open the local settings store: {error}"))?;
    store.delete(SETTINGS_KEY);
    store
        .save()
        .map_err(|error| format!("Unable to reset local settings: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_store_is_separate_from_project_layout() {
        assert!(!SETTINGS_STORE_PATH.contains(".jm"));
        assert_eq!(SETTINGS_KEY, "app-settings");
    }
}
