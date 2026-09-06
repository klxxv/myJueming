//! Import profiles and previews; decoding and segmentation are separate primitives.

use serde::{Deserialize, Serialize};

mod decode;
mod segmentation;

pub use decode::decode_bytes;
pub use segmentation::segment_text;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Gb18030,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentationMode {
    NonEmptyLine,
    SentenceRules,
    LegacyTaggedLine,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImportProfile {
    pub encoding: Encoding,
    pub segmentation_mode: SegmentationMode,
    pub strip_seg_wrappers: bool,
    pub strip_pos_suffixes: bool,
    pub compact_cjk_interchar_spaces: bool,
}
impl ImportProfile {
    pub fn new(encoding: Encoding, segmentation_mode: SegmentationMode) -> Self {
        let legacy = segmentation_mode == SegmentationMode::LegacyTaggedLine;
        Self {
            encoding,
            segmentation_mode,
            strip_seg_wrappers: legacy,
            strip_pos_suffixes: legacy,
            compact_cjk_interchar_spaces: legacy,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedText {
    pub text: String,
    pub encoding: Encoding,
    pub had_bom: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentPreview {
    pub ordinal: usize,
    pub content: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentationPreview {
    pub segments: Vec<SegmentPreview>,
    pub cleaned_text: String,
    pub warnings: Vec<String>,
}
