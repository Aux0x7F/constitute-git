use constitute_git::{
    SourceImportRequest, SourceProjectLinkRequest, SourceRefUpdateOptions, SourceRefUpdateRequest,
    build_ref_update, build_source_graph_fixture, build_status, default_now,
    default_source_graph_state, import_snapshot, link_project_work, reduce_fixture_ref_update,
    reduce_ref_update, source_graph_status, validate_source_graph_fixture,
    validate_source_graph_state,
};
use constitute_protocol::{
    FABRIC_MEMBER_CONTRIBUTION_RUNNING, FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX,
    SOURCE_PROJECT_OPERATION_STATE_APPLIED, SOURCE_UPDATE_STATE_APPLIED,
    SOURCE_UPDATE_STATE_BLOCKED, validate_host_fabric_member_contribution,
    validate_source_project_operation, validate_source_ref_update,
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
    assert!(fixture
        .host_fabric_contribution
        .module_refs
        .contains(&"module:source-content-index".to_string()));
    assert!(fixture
        .host_fabric_contribution
        .source_refs
        .contains(&fixture.graph.head_snapshot_ref));
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
            storage_object_refs: vec!["storage:object:pack-next".to_string()],
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
            storage_object_refs: vec!["storage:object:pack-next".to_string()],
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
            "storage:object:pack-cli",
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
