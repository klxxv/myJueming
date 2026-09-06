//! Export DTOs for the versioned Kernel boundary.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Json,
    Xml,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub output_path: String,
    #[serde(default = "default_true")]
    pub include_unlinked: bool,
    #[serde(default = "default_separator")]
    pub side_separator: String,
}

fn default_true() -> bool {
    true
}
fn default_separator() -> String {
    " ".into()
}
