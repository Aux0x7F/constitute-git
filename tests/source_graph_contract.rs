use constitute_git::{
    SourceImportRequest, SourceProjectLinkRequest, SourceRefStoreJournalRequest,
    SourceRefUpdateOptions, SourceRefUpdateRequest, build_ref_update, build_source_graph_fixture,
    build_source_ref_store_journal, build_status, default_now, default_source_graph_state,
    import_snapshot, link_project_work, reduce_fixture_ref_update, reduce_ref_update,
    replay_source_ref_store_journal, source_graph_status, source_ref_store_current_from_projection,
    validate_source_graph_fixture, validate_source_graph_state,
};
use constitute_protocol::{
    FABRIC_MEMBER_CONTRIBUTION_RUNNING, FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX,
    RECORD_SOURCE_APPLIED_REF_PROJECTION, SOURCE_PROJECT_OPERATION_STATE_APPLIED,
    SOURCE_UPDATE_STATE_APPLIED, SOURCE_UPDATE_STATE_BLOCKED, SourceAppliedRefProjection,
    SourceRefUpdate, SourceVersionIndexDeltaEntry, sha256_hex,
    validate_host_fabric_member_contribution, validate_source_project_operation,
    validate_source_ref_store_journal, validate_source_ref_store_replay_posture,
    validate_source_ref_update,
};
use std::{fs, process::Command};

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

fn storage_object_ref(value: &str) -> String {
    format!("storage:object:{}", sha256_hex(value))
}

fn sample_applied_ref_projection() -> SourceAppliedRefProjection {
    SourceAppliedRefProjection {
        kind: Some(RECORD_SOURCE_APPLIED_REF_PROJECTION.to_string()),
        state: SOURCE_UPDATE_STATE_APPLIED.to_string(),
        projection_ref: "source-applied-ref-projection:native:test".to_string(),
        report_ref: Some("operator-report:source-promotion-apply-test".to_string()),
        apply_ref: Some("source-promotion-apply:test".to_string()),
        repo_ref: "repo:constitute-cli".to_string(),
        target_ref: "source:ref:native-dev:constitute-cli:main".to_string(),
        lifecycle_manifest_ref: "lifecycle:manifest:native-dev:constitute-cli:test".to_string(),
        promotion_intent_ref: "promotion:intent:source-candidate:constitute-cli:test".to_string(),
        source_ref_transition_ref: "source-ref-transition:native-dev:constitute-cli:test"
            .to_string(),
        version_index_delta_ref: "version-index-delta:native-dev:constitute-cli:test".to_string(),
        witness_ref: "witness:source-promotion-apply:constitute-cli:test".to_string(),
        rollback_ref: "rollback:source-snapshot:constitute-cli:test".to_string(),
        from_source_snapshot_ref: "source:snapshot:native-dev:constitute-cli:old".to_string(),
        to_source_snapshot_ref: "source:snapshot:native-dev:constitute-cli:new".to_string(),
        from_content_index_ref: "content-index:native-dev:constitute-cli:old".to_string(),
        to_content_index_ref: "content-index:native-dev:constitute-cli:new".to_string(),
        from_selected_version_ref: "version-selection:native-dev:constitute-cli:old".to_string(),
        to_selected_version_ref: "version-selection:native-dev:constitute-cli:new".to_string(),
        to_version_index_entry: SourceVersionIndexDeltaEntry {
            entry_ref: Some("version-index-entry:native-dev:constitute-cli:new".to_string()),
            contract_ref: Some("contract:constitute-cli".to_string()),
            contract_version_ref: Some("contract-version:constitute-cli:0.1.0".to_string()),
            selected_version_ref: Some(
                "version-selection:native-dev:constitute-cli:new".to_string(),
            ),
            module_ref: Some("module:native-dev:constitute-cli".to_string()),
            repo_ref: Some("repo:constitute-cli".to_string()),
            declared_version: Some("0.1.0".to_string()),
            source_snapshot_ref: Some("source:snapshot:native-dev:constitute-cli:new".to_string()),
            content_index_ref: Some("content-index:native-dev:constitute-cli:new".to_string()),
            tree_hash_ref: Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            ),
            artifact_ref: None,
            selected_by_ref: Some("source:ref:native-dev:constitute-cli:main".to_string()),
            authority_refs: vec!["authority:source-promotion:operator-dev".to_string()],
            writer_grant_refs: vec!["grant:source-promotion:operator-dev".to_string()],
        },
        authority_refs: vec!["authority:source-promotion:operator-dev".to_string()],
        grant_refs: vec!["grant:source-promotion:operator-dev".to_string()],
        proof_gate_refs: vec!["proof-target:build:native-dev:constitute-cli".to_string()],
        evidence_refs: vec!["proof-event:source-promotion:test".to_string()],
        storage_refs: vec![storage_object_ref("source-ref-store")],
        storage_pin_refs: vec!["storage:pin-intent:source-ref-store-test".to_string()],
        storage_availability_refs: vec!["storage-availability:source-ref-store-test".to_string()],
        blocked_reasons: vec![],
        safe_facts: serde_json::json!({
            "appliedProjectionIsNotDurableSourceStore": true
        }),
        observed_at: Some("2026-05-25T11:20:00Z".to_string()),
    }
}

fn sample_source_ref_update() -> SourceRefUpdate {
    build_ref_update(SourceRefUpdateOptions {
        state: SOURCE_UPDATE_STATE_APPLIED.to_string(),
        branch: "main".to_string(),
        from_snapshot_ref: Some("source:snapshot:native-dev:constitute-cli:old".to_string()),
        to_snapshot_ref: "source:snapshot:native-dev:constitute-cli:new".to_string(),
        writer_ref: "identity:device:operator-dev".to_string(),
        now: default_now() + 42,
    })
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
    assert_eq!(
        fixture.source_project_operation.state,
        SOURCE_PROJECT_OPERATION_STATE_APPLIED
    );
    validate_source_project_operation(&fixture.source_project_operation)
        .expect("source operation validates");
    validate_host_fabric_member_contribution(&fixture.host_fabric_contribution)
        .expect("host-fabric contribution validates");
    assert_eq!(
        fixture.host_fabric_contribution.role,
        FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX
    );
    assert_eq!(
        fixture.host_fabric_contribution.state,
        FABRIC_MEMBER_CONTRIBUTION_RUNNING
    );
    assert_eq!(
        fixture.host_fabric_contribution.subject_ref,
        fixture.graph.head_snapshot_ref
    );
    assert_eq!(
        fixture.host_fabric_contribution.participant_ref,
        fixture.graph.owner_ref
    );
    assert_eq!(
        fixture.host_fabric_contribution.role_ref,
        "role:sourceContentIndex"
    );
    assert!(
        fixture
            .host_fabric_contribution
            .module_refs
            .contains(&"module:source-content-index".to_string())
    );
    assert!(
        fixture
            .host_fabric_contribution
            .source_refs
            .contains(&fixture.graph.head_snapshot_ref)
    );
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

#[test]
fn source_graph_state_carries_snapshots_updates_and_storage_edges() {
    let state = default_source_graph_state(default_now()).expect("state builds");
    validate_source_graph_state(&state).expect("state validates");
    let status = source_graph_status(&state).expect("status builds");
    assert_eq!(status.snapshot_count, 2);
    assert_eq!(status.import_proof_count, 1);
    assert_eq!(status.source_project_operation_count, 1);
    assert_eq!(status.storage_graph_edge_count, 2);
    assert_eq!(status.host_fabric_contribution_count, 1);
    assert!(
        state
            .storage_graph_edges
            .iter()
            .all(|edge| edge.relation == "sourceSnapshot.stores")
    );
}

#[test]
fn source_import_adds_snapshot_import_proof_and_storage_edges() {
    let mut state = default_source_graph_state(default_now()).expect("state builds");
    let parent = state.graph.head_snapshot_ref.clone();
    let outcome = import_snapshot(
        &mut state,
        SourceImportRequest {
            commit_ref: "git:commit:0000003".to_string(),
            tree_ref: "git:tree:0000003".to_string(),
            parent_snapshot_refs: vec![parent],
            storage_object_refs: vec![storage_object_ref("pack-next")],
            author_ref: "identity:device:agent".to_string(),
            message_digest_ref: "digest:sha256:next-message".to_string(),
            signature_refs: vec!["signature:source:next".to_string()],
            evidence_refs: vec!["source:evidence:next-import".to_string()],
            tool_ref: "tool:git:pack-import".to_string(),
            input_ref: "git:pack:next".to_string(),
            now: default_now() + 1,
        },
    )
    .expect("import applies");

    assert_eq!(outcome.storage_graph_edges.len(), 1);
    validate_source_project_operation(&outcome.source_project_operation)
        .expect("import operation validates");
    assert_eq!(
        outcome.host_fabric_contribution.role,
        FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX
    );
    assert_eq!(state.snapshots.len(), 3);
    assert_eq!(state.import_proofs.len(), 2);
    assert_eq!(state.source_project_operations.len(), 2);
    assert_eq!(state.host_fabric_contributions.len(), 2);
    assert!(
        state
            .snapshots
            .iter()
            .any(|snapshot| snapshot.snapshot_ref == outcome.snapshot.snapshot_ref)
    );
}

#[test]
fn stateful_ref_apply_moves_head_only_when_applied() {
    let mut state = default_source_graph_state(default_now()).expect("state builds");
    let old_head = state.graph.head_snapshot_ref.clone();
    let outcome = import_snapshot(
        &mut state,
        SourceImportRequest {
            commit_ref: "git:commit:0000003".to_string(),
            tree_ref: "git:tree:0000003".to_string(),
            parent_snapshot_refs: vec![old_head.clone()],
            storage_object_refs: vec![storage_object_ref("pack-next")],
            author_ref: "identity:device:agent".to_string(),
            message_digest_ref: "digest:sha256:next-message".to_string(),
            signature_refs: vec!["signature:source:next".to_string()],
            evidence_refs: vec!["source:evidence:next-import".to_string()],
            tool_ref: "tool:git:pack-import".to_string(),
            input_ref: "git:pack:next".to_string(),
            now: default_now() + 1,
        },
    )
    .expect("import applies");

    let applied = constitute_git::apply_ref_update(
        &mut state,
        SourceRefUpdateRequest {
            branch: "main".to_string(),
            from_snapshot_ref: Some(old_head),
            to_snapshot_ref: outcome.snapshot.snapshot_ref.clone(),
            writer_ref: "identity:device:agent".to_string(),
            evidence_refs: vec!["source:evidence:signed-update".to_string()],
            witness_refs: vec!["source:witness:runtime".to_string()],
            now: default_now() + 2,
        },
    )
    .expect("update applies");

    assert_eq!(applied.state, SOURCE_UPDATE_STATE_APPLIED);
    assert_eq!(state.graph.head_snapshot_ref, outcome.snapshot.snapshot_ref);
    assert_eq!(state.source_project_operations.len(), 3);

    let blocked = constitute_git::apply_ref_update(
        &mut state,
        SourceRefUpdateRequest {
            branch: "main".to_string(),
            from_snapshot_ref: Some("source:snapshot:stale".to_string()),
            to_snapshot_ref: "source:snapshot:head".to_string(),
            writer_ref: "identity:device:agent".to_string(),
            evidence_refs: vec!["source:evidence:signed-update".to_string()],
            witness_refs: vec!["source:witness:runtime".to_string()],
            now: default_now() + 3,
        },
    )
    .expect("blocked update reduces");

    assert_eq!(blocked.state, SOURCE_UPDATE_STATE_BLOCKED);
    assert_eq!(state.source_project_operations.len(), 4);
    assert_eq!(state.graph.head_snapshot_ref, outcome.snapshot.snapshot_ref);
}

#[test]
fn project_link_operation_records_adapter_posture_without_github_ownership() {
    let mut state = default_source_graph_state(default_now()).expect("state builds");
    let outcome = link_project_work(
        &mut state,
        SourceProjectLinkRequest {
            project_refs: vec!["project:constituency".to_string()],
            work_item_refs: vec!["work-item:git-project-hardening".to_string()],
            actor_ref: "identity:device:agent".to_string(),
            evidence_refs: vec!["adapter:github-project:item:git-hardening".to_string()],
            now: default_now() + 4,
            expires_at: Some(default_now() + 86_400_000),
        },
    )
    .expect("project link applies");

    validate_source_project_operation(&outcome.source_project_operation)
        .expect("project link operation validates");
    assert_eq!(
        outcome.source_project_operation.operation,
        constitute_protocol::SOURCE_OPERATION_PROJECT_LINK
    );
    assert_eq!(
        outcome.source_project_operation.project_refs,
        vec!["project:constituency"]
    );
    assert_eq!(
        outcome.source_project_operation.work_item_refs,
        vec!["work-item:git-project-hardening"]
    );
    assert_eq!(state.source_project_operations.len(), 2);
    assert_eq!(state.host_fabric_contributions.len(), 2);
    assert!(
        !serde_json::to_string(&outcome.source_project_operation)
            .expect("json")
            .contains("https://api.github.com")
    );
}

#[test]
fn source_ref_store_journal_reduces_from_applied_projection() {
    let projection = sample_applied_ref_projection();
    let update = sample_source_ref_update();
    let journal = build_source_ref_store_journal(SourceRefStoreJournalRequest {
        source_graph_ref: "source:graph:source-ref-store:test".to_string(),
        applied_projection: projection.clone(),
        prior_transitions: vec![],
        source_ref_updates: vec![update.clone()],
        storage_object_refs: projection.storage_refs.clone(),
        storage_availability_refs: projection.storage_availability_refs.clone(),
        storage_pin_intent_refs: projection.storage_pin_refs.clone(),
        storage_pin_attestation_refs: vec![
            "storage:pin-attestation:source-ref-store-test".to_string(),
        ],
        evidence_refs: vec!["evidence:source-ref-store:rust-reducer".to_string()],
        updated_at: Some("2026-05-25T11:20:01Z".to_string()),
    })
    .expect("journal reduces");

    validate_source_ref_store_journal(&journal).expect("journal validates");
    assert_eq!(journal.state, "ready");
    assert_eq!(journal.current.target_ref, projection.target_ref);
    assert_eq!(
        journal.source_ref_update_refs,
        vec![update.update_ref.clone()]
    );
    assert_eq!(
        journal.current.source_ref_update_refs,
        journal.source_ref_update_refs
    );
    assert_eq!(journal.source_ref_updates[0].update_ref, update.update_ref);
    assert_eq!(journal.transition_count, 1);
    assert_eq!(journal.storage_object_refs, projection.storage_refs);
    assert_eq!(
        journal.safe_facts["operatorJsIsNotStoreOwner"],
        serde_json::Value::Bool(true)
    );

    let replay = replay_source_ref_store_journal(
        &journal,
        &journal.target_ref,
        vec!["evidence:source-ref-store-replay:rust-reducer".to_string()],
        Some("2026-05-25T11:20:02Z".to_string()),
    )
    .expect("replay reduces");
    validate_source_ref_store_replay_posture(&replay).expect("replay validates");
    assert_eq!(replay.state, "ready");
    assert_eq!(
        replay.current_transition_ref,
        projection.source_ref_transition_ref
    );
    assert_eq!(
        replay.source_ref_update_refs,
        journal.source_ref_update_refs
    );
    assert_eq!(
        replay.safe_facts["operatorReportOrderingIsNotStateSelector"],
        serde_json::Value::Bool(true)
    );
}

#[test]
fn source_ref_store_replay_blocks_target_mismatch() {
    let projection = sample_applied_ref_projection();
    let journal = build_source_ref_store_journal(SourceRefStoreJournalRequest {
        source_graph_ref: "source:graph:source-ref-store:test".to_string(),
        applied_projection: projection,
        prior_transitions: vec![],
        source_ref_updates: vec![],
        storage_object_refs: vec![],
        storage_availability_refs: vec![],
        storage_pin_intent_refs: vec![],
        storage_pin_attestation_refs: vec![],
        evidence_refs: vec![],
        updated_at: None,
    })
    .expect("journal reduces");

    let replay =
        replay_source_ref_store_journal(&journal, "source:ref:native-dev:other:main", vec![], None)
            .expect("mismatch still reduces to blocked posture");
    validate_source_ref_store_replay_posture(&replay).expect("blocked replay validates");
    assert_eq!(replay.state, "blocked");
    assert_eq!(
        replay.blocked_reasons,
        vec!["source.refStore.targetMismatch"]
    );
}

#[test]
fn source_ref_store_current_entry_preserves_applied_projection_refs() {
    let projection = sample_applied_ref_projection();
    let current =
        source_ref_store_current_from_projection(&projection).expect("current entry builds");
    assert_eq!(current.target_ref, projection.target_ref);
    assert_eq!(
        current.to_source_snapshot_ref,
        projection.to_source_snapshot_ref
    );
    assert_eq!(
        current.to_version_index_entry,
        projection.to_version_index_entry
    );
    assert_eq!(current.storage_refs, projection.storage_refs);
}

#[test]
fn cli_emits_source_ref_store_journal_and_replay_posture() {
    let mut applied_path = std::env::temp_dir();
    applied_path.push(format!(
        "constitute-git-applied-source-ref-{}.json",
        std::process::id()
    ));
    let mut journal_path = std::env::temp_dir();
    journal_path.push(format!(
        "constitute-git-source-ref-store-{}.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&applied_path);
    let _ = fs::remove_file(&journal_path);
    let mut update_path = std::env::temp_dir();
    update_path.push(format!(
        "constitute-git-source-ref-update-{}.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&update_path);
    fs::write(
        &applied_path,
        serde_json::to_string_pretty(&sample_applied_ref_projection()).expect("projection json"),
    )
    .expect("write applied projection");
    fs::write(
        &update_path,
        serde_json::to_string_pretty(&sample_source_ref_update()).expect("source update json"),
    )
    .expect("write source update");

    let bin = env!("CARGO_BIN_EXE_constitute-git");
    let journal = Command::new(bin)
        .args(["store", "journal", "--input"])
        .arg(&applied_path)
        .args(["--source-ref-update"])
        .arg(&update_path)
        .args([
            "--source-graph",
            "source:graph:source-ref-store:test",
            "--storage-pin-attestation",
            "storage:pin-attestation:source-ref-store-cli",
        ])
        .output()
        .expect("store journal runs");
    assert!(
        journal.status.success(),
        "{}",
        String::from_utf8_lossy(&journal.stderr)
    );
    fs::write(&journal_path, &journal.stdout).expect("write journal");
    let journal_json: serde_json::Value =
        serde_json::from_slice(&journal.stdout).expect("journal json parses");
    assert_eq!(journal_json["state"], "ready");
    assert_eq!(
        journal_json["sourceRefUpdates"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        journal_json["sourceRefUpdateRefs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        journal_json["safeFacts"]["operatorJsIsNotStoreOwner"],
        serde_json::Value::Bool(true)
    );

    let replay = Command::new(bin)
        .args(["store", "replay", "--input"])
        .arg(&journal_path)
        .args([
            "--expected-target",
            "source:ref:native-dev:constitute-cli:main",
        ])
        .output()
        .expect("store replay runs");
    assert!(
        replay.status.success(),
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    let replay_json: serde_json::Value =
        serde_json::from_slice(&replay.stdout).expect("replay json parses");
    assert_eq!(replay_json["state"], "ready");
    assert_eq!(
        replay_json["safeFacts"]["sourceRefStoreReplayIsNativeReducer"],
        serde_json::Value::Bool(true)
    );
    let _ = fs::remove_file(&applied_path);
    let _ = fs::remove_file(&journal_path);
    let _ = fs::remove_file(&update_path);
}

#[test]
fn cli_persists_source_graph_state() {
    let mut path = std::env::temp_dir();
    path.push(format!("constitute-git-state-{}.json", std::process::id()));
    let _ = fs::remove_file(&path);

    let bin = env!("CARGO_BIN_EXE_constitute-git");
    let init = Command::new(bin)
        .args(["init", "--state"])
        .arg(&path)
        .output()
        .expect("init runs");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let import = Command::new(bin)
        .args(["import", "snapshot", "--state"])
        .arg(&path)
        .args([
            "--clear-default-storage-object",
            "true",
            "--storage-object",
            &storage_object_ref("pack-cli"),
            "--clear-default-signature",
            "true",
            "--signature",
            "signature:source:cli",
            "--now",
            "1779265000001",
        ])
        .output()
        .expect("import runs");
    assert!(
        import.status.success(),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );
    let import_json: serde_json::Value =
        serde_json::from_slice(&import.stdout).expect("import json parses");
    let snapshot_ref = import_json["snapshot"]["snapshotRef"]
        .as_str()
        .expect("snapshot ref")
        .to_string();

    let apply = Command::new(bin)
        .args(["ref", "apply", "--state"])
        .arg(&path)
        .args([
            "--from",
            "source:snapshot:head",
            "--to",
            &snapshot_ref,
            "--now",
            "1779265000002",
        ])
        .output()
        .expect("ref apply runs");
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );

    let project = Command::new(bin)
        .args(["project", "link", "--state"])
        .arg(&path)
        .args([
            "--clear-default-project",
            "true",
            "--project",
            "project:constituency",
            "--clear-default-work-item",
            "true",
            "--work-item",
            "work-item:git-project-hardening",
            "--clear-default-evidence",
            "true",
            "--evidence",
            "adapter:project:cli",
            "--now",
            "1779265000003",
        ])
        .output()
        .expect("project link runs");
    assert!(
        project.status.success(),
        "{}",
        String::from_utf8_lossy(&project.stderr)
    );
    let project_json: serde_json::Value =
        serde_json::from_slice(&project.stdout).expect("project json parses");
    assert_eq!(
        project_json["sourceProjectOperation"]["operation"],
        "projectLink"
    );

    let status = Command::new(bin)
        .args(["status", "--state"])
        .arg(&path)
        .output()
        .expect("status runs");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value =
        serde_json::from_slice(&status.stdout).expect("status json parses");
    assert_eq!(status_json["headSnapshotRef"], snapshot_ref);
    assert_eq!(status_json["snapshotCount"], 3);
    assert_eq!(status_json["sourceProjectOperationCount"], 4);

    let _ = fs::remove_file(&path);
}
