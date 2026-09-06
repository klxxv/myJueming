//! Desktop session state shared by IPC adapters.

use jueming_protocol::ProjectSnapshot;
use std::sync::Mutex;

pub(crate) struct OpenProject {
    pub(crate) path: String,
    pub(crate) snapshot: ProjectSnapshot,
}

#[derive(Default)]
pub(crate) struct AppKernelState {
    pub(crate) current: Mutex<Option<OpenProject>>,
}

pub(crate) fn lock_error() -> String {
    "The local Kernel state is unavailable; restart Jueming Aligner.".into()
}
