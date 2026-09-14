//! Read-only TXT/JSON/XML rendering and atomic file export.

use crate::projection::alignment_by_segment;
use crate::validation::validate_snapshot;
use crate::{KernelError, KernelService};
use jueming_core::{Segment, SegmentId};
use jueming_protocol::{ExportFormat, ExportRequest, ProjectSnapshot};
use std::collections::{HashMap, HashSet};
use std::path::Path;

impl KernelService {
    pub fn export(
        &self,
        snapshot: &ProjectSnapshot,
        request: &ExportRequest,
    ) -> Result<Vec<u8>, KernelError> {
        validate_snapshot(snapshot)?;
        if snapshot.documents.len() > 2 && request.format != ExportFormat::Json {
            let mut sections = Vec::new();
            for target in snapshot.documents.iter().skip(1) {
                let mut pair = snapshot.clone();
                pair.documents = vec![snapshot.documents[0].clone(), target.clone()];
                let pair_ids =
                    crate::projection::alignment_ids_for_document(snapshot, target.document_id);
                pair.alignments
                    .retain(|alignment| pair_ids.contains(&alignment.alignment_id));
                pair.segments.retain(|segment| {
                    segment.document_id == snapshot.documents[0].document_id
                        || segment.document_id == target.document_id
                });
                sections.push(match request.format {
                    ExportFormat::Txt => format!("# {} [{}]\n{}", target.title, target.document_id, export_txt(&pair, request)),
                    ExportFormat::Xml => format!("<comparison source_document_id=\"{}\" target_document_id=\"{}\" title=\"{}\">{}</comparison>", snapshot.documents[0].document_id, target.document_id, xml_escape(&target.title), export_xml(&pair, request).trim_start_matches("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")),
                    ExportFormat::Json => unreachable!(),
                });
            }
            let body = sections.join("\n");
            return Ok(if request.format == ExportFormat::Xml {
                format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><jueming-comparisons>{body}</jueming-comparisons>").into_bytes()
            } else {
                body.into_bytes()
            });
        }
        match request.format {
            ExportFormat::Txt => Ok(export_txt(snapshot, request).into_bytes()),
            ExportFormat::Json => serde_json::to_vec_pretty(snapshot)
                .map_err(|source| KernelError::ExportJson(source.to_string())),
            ExportFormat::Xml => Ok(export_xml(snapshot, request).into_bytes()),
        }
    }

    pub fn export_to_file(
        &self,
        snapshot: &ProjectSnapshot,
        request: &ExportRequest,
    ) -> Result<(), KernelError> {
        let bytes = self.export(snapshot, request)?;
        jueming_storage::write_bytes_atomic(Path::new(&request.output_path), &bytes)?;
        Ok(())
    }
}

fn unlinked_segments(
    snapshot: &ProjectSnapshot,
    document_id: jueming_core::DocumentId,
) -> Vec<&Segment> {
    let linked: HashSet<_> = snapshot
        .alignments
        .iter()
        .flat_map(|alignment| {
            alignment
                .source_segment_ids
                .iter()
                .chain(alignment.target_segment_ids.iter())
        })
        .copied()
        .collect();
    snapshot
        .segments
        .iter()
        .filter(|segment| {
            segment.document_id == document_id && !linked.contains(&segment.segment_id)
        })
        .collect()
}

fn export_txt(snapshot: &ProjectSnapshot, request: &ExportRequest) -> String {
    let source_id = snapshot.documents[0].document_id;
    let target_id = snapshot.documents[1].document_id;
    let segments: HashMap<_, _> = snapshot
        .segments
        .iter()
        .map(|segment| (segment.segment_id, segment.content.as_str()))
        .collect();
    let join_side = |ids: &[SegmentId]| {
        ids.iter()
            .filter_map(|id| segments.get(id).copied())
            .collect::<Vec<_>>()
            .join(&request.side_separator)
    };
    let mut lines = snapshot
        .alignments
        .iter()
        .map(|alignment| {
            format!(
                "{}\t{}\t{}",
                serde_json::to_string(&alignment.cardinality)
                    .unwrap()
                    .trim_matches('"'),
                join_side(&alignment.source_segment_ids),
                join_side(&alignment.target_segment_ids)
            )
        })
        .collect::<Vec<_>>();
    if request.include_unlinked {
        for segment in unlinked_segments(snapshot, source_id) {
            lines.push(format!("unlinked\t{}\t", segment.content));
        }
        for segment in unlinked_segments(snapshot, target_id) {
            lines.push(format!("unlinked\t\t{}", segment.content));
        }
    }
    lines.join("\n") + if lines.is_empty() { "" } else { "\n" }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn export_xml(snapshot: &ProjectSnapshot, request: &ExportRequest) -> String {
    let segments: HashMap<_, _> = snapshot
        .segments
        .iter()
        .map(|segment| (segment.segment_id, segment.content.as_str()))
        .collect();
    let side = |name: &str, ids: &[SegmentId]| {
        format!(
            "<{name}>{}</{name}>",
            ids.iter()
                .filter_map(|id| segments.get(id).copied())
                .map(xml_escape)
                .collect::<Vec<_>>()
                .join(&xml_escape(&request.side_separator))
        )
    };
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><jueming-alignment>");
    for alignment in &snapshot.alignments {
        let cardinality = serde_json::to_string(&alignment.cardinality).unwrap();
        let cardinality = cardinality.trim_matches('"');
        out.push_str(&format!(
            "<alignment id=\"{}\" cardinality=\"{}\">{}{}</alignment>",
            alignment.alignment_id,
            cardinality,
            side("source", &alignment.source_segment_ids),
            side("target", &alignment.target_segment_ids)
        ));
    }
    if request.include_unlinked {
        for segment in snapshot
            .segments
            .iter()
            .filter(|segment| !alignment_by_segment(snapshot).contains_key(&segment.segment_id))
        {
            out.push_str(&format!(
                "<unlinked segment_id=\"{}\" document_id=\"{}\">{}</unlinked>",
                segment.segment_id,
                segment.document_id,
                xml_escape(&segment.content)
            ));
        }
    }
    out.push_str("</jueming-alignment>");
    out
}
