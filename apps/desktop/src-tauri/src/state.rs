//! Desktop IPC state delegates all project ownership to `LocalAppHost`.

use jueming_application::LocalAppHost;
use std::sync::Arc;

pub(crate) struct AppKernelState {
    pub(crate) host: Arc<LocalAppHost>,
}

impl Default for AppKernelState {
    fn default() -> Self {
        Self {
            host: Arc::new(LocalAppHost::new()),
        }
    }
}
