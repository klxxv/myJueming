//! Export regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_kernel::KernelService;
use jueming_protocol::{ExportFormat, ExportRequest};

#[test]
fn exports_include_cardinality_and_unlinked_without_revision() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("export.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let source = snapshot
        .segments
        .iter()
        .filter(|segment| segment.document_id == snapshot.documents[0].document_id)
        .map(|segment| segment.segment_id)
        .collect::<Vec<_>>();
    let target = snapshot
        .segments
        .iter()
        .filter(|segment| segment.document_id == snapshot.documents[1].document_id)
        .map(|segment| segment.segment_id)
        .collect::<Vec<_>>();
    let complex = service
        .link_segments(
            &path,
            &snapshot,
            vec![source[0]],
            vec![target[0], target[1]],
            true,
        )
        .expect("1:n");
    let before_revision = complex.project.current_revision_id;
    let txt_path = temporary.path().join("out.txt");
    service
        .export_to_file(
            &complex,
            &ExportRequest {
                format: ExportFormat::Txt,
                output_path: txt_path.to_string_lossy().into_owned(),
                include_unlinked: true,
                side_separator: " / ".into(),
            },
        )
        .expect("txt export");
    let txt = std::fs::read_to_string(&txt_path).expect("read txt");
    assert!(txt.contains("1:n") && txt.contains("unlinked"));
    let json_path = temporary.path().join("out.json");
    service
        .export_to_file(
            &complex,
            &ExportRequest {
                format: ExportFormat::Json,
                output_path: json_path.to_string_lossy().into_owned(),
                include_unlinked: true,
                side_separator: " ".into(),
            },
        )
        .expect("json export");
    assert!(
        std::fs::read_to_string(&json_path)
            .unwrap()
            .contains("alignments")
    );
    let xml_path = temporary.path().join("out.xml");
    service
        .export_to_file(
            &complex,
            &ExportRequest {
                format: ExportFormat::Xml,
                output_path: xml_path.to_string_lossy().into_owned(),
                include_unlinked: true,
                side_separator: " ".into(),
            },
        )
        .expect("xml export");
    let xml = std::fs::read_to_string(&xml_path).unwrap();
    assert!(xml.contains("cardinality=\"1:n\"") && xml.contains("<unlinked"));
    assert_eq!(complex.project.current_revision_id, before_revision);
}
