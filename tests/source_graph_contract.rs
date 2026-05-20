use constitute_git::{
    SourceRefUpdateOptions, build_ref_update, build_source_graph_fixture, build_status,
    default_now, validate_source_graph_fixture,
};
use constitute_protocol::{
    SOURCE_UPDATE_STATE_APPLIED, SOURCE_UPDATE_STATE_BLOCKED, validate_source_ref_update,
};

#[test]
fn fixture_is_protocol_validated_source_graph() {
    let fixture = build_source_graph_fixture(default_now()).expect("fixture builds");
    validate_source_graph_fixture(&fixture).expect("fixture validates");
    assert_eq!(
        fixture.graph.head_snapshot_ref,
        fixture.head_snapshot.snapshot_ref
    );
    assert!(fixture.import_proof.safe_facts.get("payload").is_none());
}

#[test]
fn fast_forward_update_requires_previous_snapshot() {
    let update = build_ref_update(SourceRefUpdateOptions {
        state: SOURCE_UPDATE_STATE_APPLIED.to_string(),
        branch: "main".to_string(),
        from_snapshot_ref: None,
        to_snapshot_ref: "source:snapshot:head".to_string(),
        writer_ref: "identity:device:agent".to_string(),
        now: default_now(),
    });
    assert!(validate_source_ref_update(&update).is_err());
}

#[test]
fn blocked_ref_update_names_policy_reason() {
    let update = build_ref_update(SourceRefUpdateOptions {
        state: SOURCE_UPDATE_STATE_BLOCKED.to_string(),
        branch: "main".to_string(),
        from_snapshot_ref: Some("source:snapshot:parent".to_string()),
        to_snapshot_ref: "source:snapshot:head".to_string(),
        writer_ref: "identity:device:agent".to_string(),
        now: default_now(),
    });
    validate_source_ref_update(&update).expect("blocked update is valid posture");
    assert_eq!(
        update.blocked_reasons,
        vec!["source.policy.fastForwardRequired"]
    );
}

#[test]
fn status_is_bounded_and_storage_backed() {
    let status = build_status().expect("status builds");
    assert!(status.source_graph_ref.starts_with("source:graph:"));
    assert!(status.storage_backend_ref.starts_with("storage:backend:"));
}
