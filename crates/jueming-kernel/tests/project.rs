//! Project regressions through the public Kernel facade.

mod common;

use common::government_request;
use jueming_core::RevisionId;
use jueming_kernel::{KernelError, KernelService};
use jueming_protocol::TextInput;

#[test]
fn project_create_save_open_round_trip() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("government.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let summary = service.summarize(&snapshot).expect("summary");
    assert_eq!(summary.source_count, 8);
    assert_eq!(summary.target_count, 8);
    assert_eq!(summary.alignment_count, 8);
    assert_eq!(summary.source_unlinked_count, 0);
    assert_eq!(summary.target_unlinked_count, 0);

    let restored = service.open_project(&path).expect("open project");
    assert_eq!(restored, snapshot);
    assert_eq!(
        restored.segments[0].segment_id,
        snapshot.segments[0].segment_id
    );
}

#[test]
fn one_extra_target_is_unlinked() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("unlinked.jm");
    let mut request = government_request(&path);
    if let TextInput::Paste { text, .. } = &mut request.target.input {
        text.push_str("\nOne extra target segment.");
    }
    let service = KernelService;
    let snapshot = service.create_project(&request).expect("create project");
    let summary = service.summarize(&snapshot).expect("summary");
    assert_eq!(summary.alignment_count, 8);
    assert_eq!(summary.target_unlinked_count, 1);
}

#[test]
fn edits_advance_revision_and_survive_reopen() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("edited.jm");
    let service = KernelService;
    let snapshot = service
        .create_project(&government_request(&path))
        .expect("create project");
    let segment_id = snapshot.segments[0].segment_id;
    let edited = service
        .update_segment(&path, &snapshot, segment_id, "修订后的第一句。")
        .expect("edit segment");
    assert_eq!(edited.project.current_revision_id, RevisionId::new(2));
    assert_eq!(edited.revisions.len(), 2);
    assert_eq!(
        edited
            .segments
            .iter()
            .find(|segment| segment.segment_id == segment_id)
            .expect("edited segment")
            .content,
        "修订后的第一句。"
    );
    assert_eq!(service.open_project(&path).expect("reopen"), edited);
}

#[test]
fn language_catalogue_is_exposed_and_legacy_chinese_is_normalized_on_create() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("language-catalogue.jm");
    let service = KernelService;
    let languages = service.supported_languages();
    assert_eq!(languages.len(), 10);
    assert_eq!(languages[0].language_id, "en");
    let project = service
        .create_project(&government_request(&path))
        .expect("legacy zh-CN request stays compatible");
    assert_eq!(project.project.source_language, "zh");
    assert_eq!(project.documents[0].language_id, "zh");
    let mut unsupported = government_request(&temporary.path().join("unsupported.jm"));
    unsupported.source.language_id = "ar".into();
    assert!(matches!(
        service.create_project(&unsupported),
        Err(KernelError::UnsupportedLanguage(language)) if language == "ar"
    ));
}
