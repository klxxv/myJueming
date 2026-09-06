//! Import IPC adapters; domain work stays in KernelService.

use jueming_kernel::KernelService;
use jueming_protocol::{ImportPreviewResponse, PreviewImportRequest};

#[tauri::command]
pub(crate) fn preview_import(
    request: PreviewImportRequest,
) -> Result<ImportPreviewResponse, String> {
    KernelService
        .preview_import(&request.input, &request.profile)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_supported_languages() -> Vec<jueming_protocol::SupportedLanguage> {
    KernelService.supported_languages()
}
