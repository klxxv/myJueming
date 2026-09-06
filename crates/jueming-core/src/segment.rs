//! Canonical segment content and identity-preserving edits.

use crate::ids::{DocumentId, RevisionId, SegmentId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContentRef {
    pub key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Sentence,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub segment_id: SegmentId,
    pub document_id: DocumentId,
    pub kind: SegmentKind,
    pub content_ref: ContentRef,
    pub content: String,
    pub content_hash: String,
    pub created_revision_id: RevisionId,
    pub updated_revision_id: RevisionId,
}

impl Segment {
    pub fn new(document_id: DocumentId, content: impl Into<String>, revision: RevisionId) -> Self {
        let content = content.into();
        Self {
            segment_id: SegmentId::new(),
            document_id,
            kind: SegmentKind::Sentence,
            content_ref: ContentRef { key: String::new() },
            content_hash: content_hash(&content),
            content,
            created_revision_id: revision,
            updated_revision_id: revision,
        }
    }

    pub fn update_content(&mut self, content: impl Into<String>, revision: RevisionId) {
        let content = content.into();
        self.content_hash = content_hash(&content);
        self.content = content;
        self.updated_revision_id = revision;
    }
}

fn content_hash(value: &str) -> String {
    // FNV-1a is intentionally dependency-free at this layer; storage may add a
    // cryptographic digest when it materializes ContentRef chunks.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
