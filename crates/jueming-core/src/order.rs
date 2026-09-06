//! Display ordering and coverage invariants for stable segment IDs.

use crate::error::CoreError;
use crate::ids::{DocumentId, RevisionId, SegmentId, SegmentOrderId};
use crate::segment::Segment;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PositionKey(pub String);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderedSegmentRef {
    pub segment_id: SegmentId,
    pub position_key: PositionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentOrder {
    pub segment_order_id: SegmentOrderId,
    pub document_id: DocumentId,
    pub entries: Vec<OrderedSegmentRef>,
    pub updated_revision_id: RevisionId,
}
impl SegmentOrder {
    pub fn initial(document_id: DocumentId, segments: &[Segment], revision: RevisionId) -> Self {
        Self {
            segment_order_id: SegmentOrderId::new(),
            document_id,
            entries: segments
                .iter()
                .enumerate()
                .map(|(i, s)| OrderedSegmentRef {
                    segment_id: s.segment_id,
                    position_key: PositionKey(format!("{:020}", i)),
                })
                .collect(),
            updated_revision_id: revision,
        }
    }
}

pub fn validate_segment_order(order: &SegmentOrder, segments: &[Segment]) -> Result<(), CoreError> {
    if order.entries.len() != segments.len()
        || order
            .entries
            .windows(2)
            .any(|pair| pair[0].position_key >= pair[1].position_key)
    {
        return Err(CoreError::InvalidSegmentOrder);
    }
    let expected: HashSet<_> = segments.iter().map(|s| s.segment_id).collect();
    let actual: HashSet<_> = order.entries.iter().map(|e| e.segment_id).collect();
    if expected.len() != segments.len() || actual != expected {
        return Err(CoreError::InvalidSegmentOrder);
    }
    if segments.iter().any(|s| s.document_id != order.document_id) {
        return Err(CoreError::WrongDocument);
    }
    Ok(())
}
