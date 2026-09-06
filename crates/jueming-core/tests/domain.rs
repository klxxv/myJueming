//! Public API regression tests.

use jueming_core::{Alignment, Cardinality, ProjectId, RevisionId, SegmentId};

#[test]
fn revision_id_preserves_values_above_javascript_integer_precision() {
    let revision = RevisionId::new(u64::MAX);
    let encoded = serde_json::to_value(revision).unwrap();
    assert_eq!(encoded, u64::MAX.to_string());
    assert_eq!(
        serde_json::from_value::<RevisionId>(encoded).unwrap(),
        revision
    );
    assert!(serde_json::from_str::<RevisionId>("42").is_err());
}

#[test]
fn ids_are_uuid_v7_and_serde_transparent() {
    let id = SegmentId::new();
    assert_eq!(id.as_uuid().get_version_num(), 7);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(serde_json::from_str::<SegmentId>(&json).unwrap(), id);
}

#[test]
fn revision_id_is_a_decimal_wire_string() {
    assert_eq!(
        serde_json::to_string(&RevisionId::new(42)).unwrap(),
        "\"42\""
    );
    assert_eq!(
        serde_json::from_str::<RevisionId>("\"42\"")
            .unwrap()
            .value(),
        42
    );
}

#[test]
fn cardinalities_are_derived() {
    let project = ProjectId::new();
    let r = RevisionId::new(1);
    assert_eq!(
        Alignment::new(project, vec![SegmentId::new()], vec![SegmentId::new()], r)
            .unwrap()
            .cardinality,
        Cardinality::OneToOne
    );
    assert_eq!(
        Alignment::new(
            project,
            vec![SegmentId::new()],
            vec![SegmentId::new(), SegmentId::new()],
            r
        )
        .unwrap()
        .cardinality,
        Cardinality::OneToMany
    );
    assert_eq!(
        Alignment::new(
            project,
            vec![SegmentId::new(), SegmentId::new()],
            vec![SegmentId::new()],
            r
        )
        .unwrap()
        .cardinality,
        Cardinality::ManyToOne
    );
    assert_eq!(
        Alignment::new(
            project,
            vec![SegmentId::new(), SegmentId::new()],
            vec![SegmentId::new(), SegmentId::new()],
            r
        )
        .unwrap()
        .cardinality,
        Cardinality::ManyToMany
    );
}
