//! Trusted native command adapter. Metadata is checked under the project lock.
use super::{AppError, LocalAppHost, kernel_error};
use jueming_protocol::{
    AlignmentGapEdge, AlignmentRefPayload, AnnotationCreateRequest, AnnotationId,
    AnnotationUpdateRequest, BookmarkCreateRequest, BookmarkId, BookmarkUpdateRequest,
    CONTRACT_VERSION, CommandEnvelope, CommandReceipt, CommandResult, GroupAlignmentPayload,
    LinkSegmentsPayload, MergeSegmentsPayload, MoveSegmentPayload, ReplaceApplyRequest, RevisionId,
    SegmentId, SplitSegmentPayload, UngroupAlignmentPayload, UpdateSegmentPayload,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
enum NativeCommand {
    UpdateSegment(UpdateSegmentPayload),
    MoveSegment(MoveSegmentPayload),
    ReorderSegments {
        ordered_segment_ids: Vec<SegmentId>,
    },
    InsertAlignmentGap {
        #[serde(default)]
        target_document_id: Option<jueming_protocol::DocumentId>,
        segment_id: SegmentId,
        edge: AlignmentGapEdge,
    },
    LinkSegments(LinkSegmentsPayload),
    UnlinkAlignment(AlignmentRefPayload),
    MergeSegments(MergeSegmentsPayload),
    SplitSegment(SplitSegmentPayload),
    #[serde(alias = "merge_alignments", alias = "merge_alignment")]
    GroupAlignment(GroupAlignmentPayload),
    #[serde(alias = "split_alignment")]
    UngroupAlignment(UngroupAlignmentPayload),
    CreateBookmark(BookmarkCreateRequest),
    UpdateBookmark(BookmarkUpdateRequest),
    DeleteBookmark {
        bookmark_id: BookmarkId,
    },
    CreateAnnotation(AnnotationCreateRequest),
    UpdateAnnotation(AnnotationUpdateRequest),
    DeleteAnnotation {
        annotation_id: AnnotationId,
    },
    ResolveAnnotation {
        annotation_id: AnnotationId,
    },
    ApplyReplace(ReplaceApplyRequest),
    ConfirmResearch(jueming_protocol::ResearchRecord),
    MergeResearchGroups(jueming_protocol::ResearchGroupMergeRequest),
    Undo {},
    Redo {},
    RestoreRevision {
        target_revision_id: RevisionId,
    },
}

impl LocalAppHost {
    pub fn execute_native_command(
        &self,
        envelope: CommandEnvelope<Value>,
    ) -> Result<CommandResult, AppError> {
        if envelope.contract_version != CONTRACT_VERSION {
            return Err(AppError::new(
                "unsupported",
                "Unsupported command contract version.",
            ));
        }
        let command: NativeCommand =
            serde_json::from_value(json!({"kind": envelope.kind, "payload": envelope.payload}))
                .map_err(|e| AppError::invalid(e.to_string()))?;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&envelope).map_err(|e| AppError::invalid(e.to_string()))?,
        )
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
        let mut state = self.lock()?;
        let current = &state
            .current
            .as_ref()
            .ok_or_else(AppError::unavailable)?
            .snapshot;
        if current.project.project_id != envelope.project_id {
            return Err(AppError::new(
                "project_mismatch",
                "The command belongs to another project.",
            ));
        }
        if let Some(receipt) = current
            .command_receipts
            .iter()
            .find(|r| r.command_id == envelope.command_id)
        {
            if receipt.fingerprint != fingerprint {
                return Err(AppError::new(
                    "command_id_conflict",
                    "A command ID cannot be reused with different arguments.",
                ));
            }
            return Ok(CommandResult {
                command_id: receipt.command_id,
                project_id: envelope.project_id,
                committed_revision_id: receipt.committed_revision_id,
                status: "duplicate".into(),
            });
        }
        if current.project.current_revision_id != envelope.base_revision_id {
            return Err(kernel_error(jueming_kernel::KernelError::StaleRevision {
                expected: current.project.current_revision_id,
                provided: envelope.base_revision_id,
            }));
        }
        let receipt = CommandReceipt {
            command_id: envelope.command_id,
            fingerprint,
            committed_revision_id: RevisionId::new(envelope.base_revision_id.value() + 1),
        };
        let next = self.mutate_locked(&mut state, "native", |k, current, path| {
            // Kernel operations copy the receipt into the same atomic snapshot
            // as the resulting revision. Failed validation never publishes it.
            let mut input = current.clone();
            input.command_receipts.push(receipt);
            let next = match command {
                NativeCommand::ConfirmResearch(record) => k.confirm_research(path, &input, record),
                NativeCommand::MergeResearchGroups(request) => {
                    k.merge_research_groups(path, &input, request)
                }
                NativeCommand::UpdateSegment(p) => {
                    k.update_segment(path, &input, p.segment_id, &p.content)
                }
                NativeCommand::MoveSegment(p) => k.move_segment(
                    path,
                    &input,
                    p.segment_id,
                    p.before_segment_id,
                    p.after_segment_id,
                ),
                NativeCommand::ReorderSegments {
                    ordered_segment_ids,
                } => k.reorder_segments(path, &input, ordered_segment_ids),
                NativeCommand::InsertAlignmentGap {
                    segment_id,
                    edge,
                    target_document_id,
                } => k.insert_alignment_gap_for_document(
                    path,
                    &input,
                    segment_id,
                    edge,
                    target_document_id,
                ),
                NativeCommand::LinkSegments(p) => k.link_segments(
                    path,
                    &input,
                    p.source_segment_ids,
                    p.target_segment_ids,
                    p.replace_existing,
                ),
                NativeCommand::UnlinkAlignment(p) => {
                    k.unlink_alignment(path, &input, p.alignment_id)
                }
                NativeCommand::MergeSegments(p) => {
                    k.merge_segments(path, &input, p.segment_ids, &p.merged_content)
                }
                NativeCommand::SplitSegment(p) => {
                    k.split_segment(path, &input, p.segment_id, p.parts)
                }
                NativeCommand::GroupAlignment(p) => {
                    k.group_alignment(path, &input, p.alignment_ids, p.unlinked_segment_ids)
                }
                NativeCommand::UngroupAlignment(p) => k.ungroup_alignment(
                    path,
                    &input,
                    p.alignment_id,
                    p.source_groups,
                    p.target_groups,
                ),
                NativeCommand::CreateBookmark(p) => k.create_bookmark(path, &input, p),
                NativeCommand::UpdateBookmark(p) => k.update_bookmark(path, &input, p),
                NativeCommand::DeleteBookmark { bookmark_id } => {
                    k.delete_bookmark(path, &input, bookmark_id)
                }
                NativeCommand::CreateAnnotation(p) => k.create_annotation(path, &input, p),
                NativeCommand::UpdateAnnotation(p) => k.update_annotation(path, &input, p),
                NativeCommand::DeleteAnnotation { annotation_id } => {
                    k.delete_annotation(path, &input, annotation_id)
                }
                NativeCommand::ResolveAnnotation { annotation_id } => {
                    k.resolve_annotation(path, &input, annotation_id)
                }
                NativeCommand::ApplyReplace(p) => k.apply_replace(path, &input, &p),
                NativeCommand::Undo {} => k.undo(path, &input),
                NativeCommand::Redo {} => k.redo(path, &input),
                NativeCommand::RestoreRevision { target_revision_id } => {
                    k.restore_revision(path, &input, target_revision_id)
                }
            }?;
            if next.project.current_revision_id == current.project.current_revision_id {
                k.commit_unchanged_command(path, next)
            } else {
                Ok(next)
            }
        })?;
        Ok(CommandResult {
            command_id: envelope.command_id,
            project_id: envelope.project_id,
            committed_revision_id: next.project.current_revision_id,
            status: "committed".into(),
        })
    }
}
