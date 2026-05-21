use constitute_git::{
    SourceRefUpdateOptions, SourceRefUpdateRequest, build_ref_update, build_source_graph_fixture,
    build_status, default_now, reduce_fixture_ref_update, reduce_ref_update,
    validate_source_graph_fixture,
};
use constitute_protocol::{
    SOURCE_UPDATE_STATE_APPLIED, SOURCE_UPDATE_STATE_BLOCKED, validate_source_ref_update,
};

fn default_request() -> SourceRefUpdateRequest {
    SourceRefUpdateRequest {
        branch: "main".to_string(),
        from_snapshot_ref: Some("source:snapshot:parent".to_string()),
        to_snapshot_ref: "source:snapshot:head".to_string(),
        writer_ref: "identity:device:agent".to_string(),
        evidence_refs: vec!["source:evidence:signed-update".to_string()],
        witness_refs: vec!["source:witness:runtime".to_string()],
        now: default_now(),
    }
}

#[test]
fn fixture_is_protocol_validated_source_graph() {
    let fixture = build_source_graph_fixture(default_now()).expect("fixture builds");
    validate_source_graph_fixture(&fixture).expect("fixture validates");
    assert_eq!(
        fixture.graph.head_snapshot_ref,
        fixture.head_snapshot.snapshot_ref
    );
    assert!(fixture.import_proof.safe_facts.get("payload").is_none());
    assert_eq!(fixture.ref_update.state, SOURCE_UPDATE_STATE_APPLIED);
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

#[test]
fn reduced_ref_update_applies_with_usable_writer_grant() {
    let update = reduce_fixture_ref_update(default_request()).expect("update reduces");
    validate_source_ref_update(&update).expect("reduced update validates");
    assert_eq!(update.state, SOURCE_UPDATE_STATE_APPLIED);
    assert_eq!(update.grant_refs, vec!["source:grant:writer-agent"]);
    assert!(update.blocked_reasons.is_empty());
}

#[test]
fn reduced_ref_update_blocks_without_usable_writer_grant() {
    let mut request = default_request();
    request.writer_ref = "identity:device:unknown".to_string();

    let update = reduce_fixture_ref_update(request).expect("update reduces");
    validate_source_ref_update(&update).expect("blocked update validates");
    assert_eq!(update.state, SOURCE_UPDATE_STATE_BLOCKED);
    assert!(update.grant_refs.is_empty());
    assert!(
        update
            .blocked_reasons
            .contains(&"source.grant.unusable".to_string())
    );
}

#[test]
fn reduced_ref_update_blocks_stale_fast_forward_base() {
    let mut request = default_request();
    request.from_snapshot_ref = Some("source:snapshot:head".to_string());

    let update = reduce_fixture_ref_update(request).expect("update reduces");
    validate_source_ref_update(&update).expect("blocked update validates");
    assert_eq!(update.state, SOURCE_UPDATE_STATE_BLOCKED);
    assert!(
        update
            .blocked_reasons
            .contains(&"source.policy.fastForwardRequired".to_string())
    );
}

#[test]
fn reduced_ref_update_blocks_missing_signature_evidence() {
    let mut request = default_request();
    request.evidence_refs.clear();

    let update = reduce_fixture_ref_update(request).expect("update reduces");
    validate_source_ref_update(&update).expect("blocked update validates");
    assert_eq!(update.state, SOURCE_UPDATE_STATE_BLOCKED);
    assert!(
        update
            .blocked_reasons
            .contains(&"source.policy.signedUpdateRequired".to_string())
    );
}

#[test]
fn reducer_uses_supplied_graph_baseline_not_fixture_flags() {
    let fixture = build_source_graph_fixture(default_now()).expect("fixture builds");
    let mut graph = fixture.graph.clone();
    graph.head_snapshot_ref = fixture.parent_snapshot.snapshot_ref.clone();

    let update = reduce_ref_update(
        &graph,
        &[fixture.writer_grant],
        &[fixture.parent_snapshot, fixture.head_snapshot],
        default_request(),
    )
    .expect("update reduces");

    assert_eq!(update.state, SOURCE_UPDATE_STATE_APPLIED);
}
