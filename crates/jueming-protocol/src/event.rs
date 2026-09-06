//! Event DTOs for the versioned Kernel boundary.

use jueming_core::{AlignmentId, RevisionId, SegmentId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum KernelEvent {
    RevisionAdvanced {
        revision_id: RevisionId,
    },
    SegmentChanged {
        segment_id: SegmentId,
        revision_id: RevisionId,
    },
    AlignmentChanged {
        alignment_id: AlignmentId,
        revision_id: RevisionId,
    },
    SaveStateChanged {
        dirty: bool,
        revision_id: RevisionId,
    },
    IndexUpdated {
        revision_id: RevisionId,
    },
}
