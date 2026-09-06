use serde::{Deserialize, Serialize};

use jueming_storage::{ProjectLayout, StorageError};
use std::fs;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Snapshot {
    revision: u64,
    text: String,
}

#[test]
fn revision_snapshots_round_trip_without_domain_or_protocol_types() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let layout = ProjectLayout::new(temporary.path().join("generic.jm")).expect("layout");
    let historical = Snapshot {
        revision: 1,
        text: "historical".into(),
    };
    layout.write_revision_snapshot(1, &historical).unwrap();
    layout
        .write_snapshot(&Snapshot {
            revision: 2,
            text: "current".into(),
        })
        .unwrap();

    assert_eq!(
        layout.read_revision_snapshot::<Snapshot>(1).unwrap(),
        historical
    );
    assert_eq!(layout.read_snapshot::<Snapshot>().unwrap().revision, 2);
}

#[test]
fn rejects_non_project_directory() {
    let error = ProjectLayout::new("example").expect_err("extension is required");
    assert!(matches!(error, StorageError::InvalidProjectRoot(_)));
}

#[test]
fn snapshot_round_trip_and_replace_are_atomic() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let layout = ProjectLayout::new(temporary.path().join("sample.jm")).expect("layout");

    layout
        .write_snapshot(&Snapshot {
            revision: 1,
            text: "first".into(),
        })
        .expect("first write");
    layout
        .write_snapshot(&Snapshot {
            revision: 2,
            text: "second".into(),
        })
        .expect("replacement write");

    let restored: Snapshot = layout.read_snapshot().expect("read snapshot");
    assert_eq!(
        restored,
        Snapshot {
            revision: 2,
            text: "second".into(),
        }
    );
    let temporary_files = fs::read_dir(layout.root())
        .expect("list project")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(temporary_files, 0);
}

#[test]
fn cache_cleanup_never_touches_snapshot_or_revisions() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let layout = ProjectLayout::new(temporary.path().join("sample.jm")).expect("layout");
    layout
        .write_snapshot(&Snapshot {
            revision: 2,
            text: "canonical".into(),
        })
        .expect("write canonical snapshot");
    layout
        .write_revision_snapshot(
            2,
            &Snapshot {
                revision: 2,
                text: "history".into(),
            },
        )
        .expect("write revision snapshot");
    fs::create_dir_all(layout.cache_dir().join("search")).expect("cache folder");
    fs::write(
        layout.cache_dir().join("search/index.bin"),
        b"derived-cache",
    )
    .expect("cache file");

    assert_eq!(layout.clear_cache().expect("clear cache"), 13);
    assert!(layout.cache_dir().is_dir());
    assert_eq!(
        fs::read_dir(layout.cache_dir())
            .expect("empty cache")
            .count(),
        0
    );
    assert!(layout.snapshot_path().is_file());
    assert!(layout.revision_snapshot_path(2).is_file());
}
