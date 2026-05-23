// domain-owned-vocabulary: source.graph.notReady source.graph.expired source.ref.unknown source.snapshot.unknown source.grant.unusable source.policy.fastForwardRequired source.policy.reviewRequired source.policy.signedUpdateRequired
use anyhow::{Result, anyhow};
use constitute_fabric::{HostFabricMemberContributionSpec, build_host_fabric_member_contribution};
use constitute_protocol::{
    FABRIC_MEMBER_CONTRIBUTION_BLOCKED, FABRIC_MEMBER_CONTRIBUTION_RUNNING,
    FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX, HostFabricMemberContribution,
    RECORD_SOURCE_IMPORT_PROOF, RECORD_SOURCE_PROJECT_OPERATION, RECORD_SOURCE_REF_UPDATE,
    RECORD_SOURCE_SNAPSHOT, RECORD_SOURCE_VERSION_GRAPH, RECORD_SOURCE_WRITER_GRANT,
    SOURCE_GRAPH_STATE_READY, SOURCE_IMPORT_STATE_IMPORTED, SOURCE_OPERATION_FETCH,
    SOURCE_OPERATION_IMPORT, SOURCE_OPERATION_PROJECT_LINK, SOURCE_OPERATION_PUSH,
    SOURCE_OPERATION_REF_UPDATE, SOURCE_OPERATION_STATUS, SOURCE_PROJECT_COMPATIBILITY_SUPPORTED,
    SOURCE_PROJECT_OPERATION_STATE_APPLIED, SOURCE_PROJECT_OPERATION_STATE_BLOCKED,
    SOURCE_PROJECT_OPERATION_STATE_REJECTED, SOURCE_PROJECT_OPERATION_STATE_REQUESTED,
    SOURCE_PROJECT_OPERATION_STATE_SUPERSEDED, SOURCE_REF_KIND_BRANCH, SOURCE_UPDATE_STATE_APPLIED,
    SOURCE_UPDATE_STATE_BLOCKED, SOURCE_UPDATE_STATE_REJECTED, SOURCE_UPDATE_STATE_SUPERSEDED,
    SourceGraphPolicy, SourceImportProof, SourceProjectOperation, SourceRefUpdate, SourceSnapshot,
    SourceVersionGraph, SourceWriterGrant, StorageGraphEdge, sha256_hex, source_ref,
    validate_host_fabric_member_contribution, validate_source_import_proof,
    validate_source_project_operation, validate_source_ref_update, validate_source_snapshot,
    validate_source_version_graph, validate_source_writer_grant, validate_storage_graph_edge,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const DEFAULT_NOW: u64 = 1_779_265_000_000;
const REASON_GRAPH_NOT_READY: &str = "source.graph.notReady";
const REASON_GRAPH_EXPIRED: &str = "source.graph.expired";
const REASON_REF_UNKNOWN: &str = "source.ref.unknown";
const REASON_SNAPSHOT_UNKNOWN: &str = "source.snapshot.unknown";
const REASON_GRANT_UNUSABLE: &str = "source.grant.unusable";
const REASON_FAST_FORWARD_REQUIRED: &str = "source.policy.fastForwardRequired";
const REASON_REVIEW_REQUIRED: &str = "source.policy.reviewRequired";
const REASON_SIGNED_UPDATE_REQUIRED: &str = "source.policy.signedUpdateRequired";
const DEFAULT_FABRIC_REF: &str = "fabric:source-lab";
const DEFAULT_HOST_REF: &str = "host:runner-lab";
const DEFAULT_SOURCE_MEMBER_REF: &str =
    "b8a4523a801d84e030f81645097b84f4ba78bd8e4986b62b82ad1e215bbf6312";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGraphFixture {
    pub graph: SourceVersionGraph,
    pub writer_grant: SourceWriterGrant,
    pub parent_snapshot: SourceSnapshot,
    pub head_snapshot: SourceSnapshot,
    pub ref_update: SourceRefUpdate,
    pub import_proof: SourceImportProof,
    pub source_project_operation: SourceProjectOperation,
    pub host_fabric_contribution: HostFabricMemberContribution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGraphStatus {
    pub source_graph_ref: String,
    pub head_snapshot_ref: String,
    pub default_branch_ref: String,
    pub allowed_operations: Vec<String>,
    pub storage_backend_ref: String,
    pub state: String,
    pub snapshot_count: usize,
    pub ref_update_count: usize,
    pub import_proof_count: usize,
    pub source_project_operation_count: usize,
    pub storage_graph_edge_count: usize,
    pub host_fabric_contribution_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGraphState {
    pub graph: SourceVersionGraph,
    #[serde(default)]
    pub writer_grants: Vec<SourceWriterGrant>,
    #[serde(default)]
    pub snapshots: Vec<SourceSnapshot>,
    #[serde(default)]
    pub ref_updates: Vec<SourceRefUpdate>,
    #[serde(default)]
    pub import_proofs: Vec<SourceImportProof>,
    #[serde(default)]
    pub source_project_operations: Vec<SourceProjectOperation>,
    #[serde(default)]
    pub storage_graph_edges: Vec<StorageGraphEdge>,
    #[serde(default)]
    pub host_fabric_contributions: Vec<HostFabricMemberContribution>,
    pub updated_at: u64,
}

#[derive(Clone, Debug)]
pub struct SourceImportRequest {
    pub commit_ref: String,
    pub tree_ref: String,
    pub parent_snapshot_refs: Vec<String>,
    pub storage_object_refs: Vec<String>,
    pub author_ref: String,
    pub message_digest_ref: String,
    pub signature_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub tool_ref: String,
    pub input_ref: String,
    pub now: u64,
}

#[derive(Clone, Debug)]
pub struct SourceProjectLinkRequest {
    pub project_refs: Vec<String>,
    pub work_item_refs: Vec<String>,
    pub actor_ref: String,
    pub evidence_refs: Vec<String>,
    pub now: u64,
    pub expires_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceImportOutcome {
    pub snapshot: SourceSnapshot,
    pub import_proof: SourceImportProof,
    pub source_project_operation: SourceProjectOperation,
    #[serde(default)]
    pub storage_graph_edges: Vec<StorageGraphEdge>,
    pub host_fabric_contribution: HostFabricMemberContribution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProjectLinkOutcome {
    pub source_project_operation: SourceProjectOperation,
    pub host_fabric_contribution: HostFabricMemberContribution,
}

pub fn default_policy() -> SourceGraphPolicy {
    SourceGraphPolicy {
        fast_forward_only: true,
        review_required: true,
        signed_updates_required: true,
        allowed_operations: vec![
            SOURCE_OPERATION_IMPORT.to_string(),
            SOURCE_OPERATION_FETCH.to_string(),
            SOURCE_OPERATION_PUSH.to_string(),
            SOURCE_OPERATION_REF_UPDATE.to_string(),
            SOURCE_OPERATION_STATUS.to_string(),
        ],
    }
}

pub fn build_source_graph_fixture(now: u64) -> Result<SourceGraphFixture> {
    let parent_snapshot = SourceSnapshot {
        kind: Some(RECORD_SOURCE_SNAPSHOT.to_string()),
        source_graph_ref: source_ref("graph", "constitute-git"),
        snapshot_ref: source_ref("snapshot", "parent"),
        commit_ref: "git:commit:0000001".to_string(),
        tree_ref: "git:tree:0000001".to_string(),
        parent_snapshot_refs: vec![],
        storage_object_refs: vec!["storage:object:pack-parent".to_string()],
        author_ref: "identity:root:aux".to_string(),
        message_digest_ref: "digest:sha256:parent-message".to_string(),
        signature_refs: vec!["signature:source:parent".to_string()],
        evidence_refs: vec!["source:evidence:parent-import".to_string()],
        issued_at: now.saturating_sub(10),
    };
    let head_snapshot = SourceSnapshot {
        kind: Some(RECORD_SOURCE_SNAPSHOT.to_string()),
        source_graph_ref: parent_snapshot.source_graph_ref.clone(),
        snapshot_ref: source_ref("snapshot", "head"),
        commit_ref: "git:commit:0000002".to_string(),
        tree_ref: "git:tree:0000002".to_string(),
        parent_snapshot_refs: vec![parent_snapshot.snapshot_ref.clone()],
        storage_object_refs: vec!["storage:object:pack-head".to_string()],
        author_ref: "identity:device:agent".to_string(),
        message_digest_ref: "digest:sha256:head-message".to_string(),
        signature_refs: vec!["signature:source:head".to_string()],
        evidence_refs: vec!["source:evidence:head-import".to_string()],
        issued_at: now,
    };
    let writer_grant = SourceWriterGrant {
        kind: Some(RECORD_SOURCE_WRITER_GRANT.to_string()),
        grant_ref: source_ref("grant", "writer-agent"),
        source_graph_ref: parent_snapshot.source_graph_ref.clone(),
        issuer_ref: "identity:root:aux".to_string(),
        subject_ref: "identity:device:agent".to_string(),
        scope_refs: vec![source_ref("ref", "main")],
        allowed_operations: vec![
            SOURCE_OPERATION_PUSH.to_string(),
            SOURCE_OPERATION_REF_UPDATE.to_string(),
        ],
        evidence_refs: vec!["authority:grant:source-writer".to_string()],
        issued_at: now.saturating_sub(20),
        expires_at: Some(now + 86_400_000),
        revoked_at: None,
    };
    let base_graph = SourceVersionGraph {
        kind: Some(RECORD_SOURCE_VERSION_GRAPH.to_string()),
        source_graph_ref: parent_snapshot.source_graph_ref.clone(),
        owner_ref: "identity:root:aux".to_string(),
        storage_backend_ref: "storage:backend:local".to_string(),
        default_branch_ref: source_ref("ref", "main"),
        head_snapshot_ref: parent_snapshot.snapshot_ref.clone(),
        state: SOURCE_GRAPH_STATE_READY.to_string(),
        policy: default_policy(),
        branch_refs: vec![source_ref("ref", "main")],
        tag_refs: vec![],
        writer_grant_refs: vec![writer_grant.grant_ref.clone()],
        release_refs: vec![],
        evidence_refs: vec![],
        blocked_reasons: vec![],
        issued_at: now.saturating_sub(30),
        expires_at: Some(now + 86_400_000),
    };
    let ref_update = reduce_ref_update(
        &base_graph,
        std::slice::from_ref(&writer_grant),
        &[parent_snapshot.clone(), head_snapshot.clone()],
        SourceRefUpdateRequest {
            branch: "main".to_string(),
            from_snapshot_ref: Some(parent_snapshot.snapshot_ref.clone()),
            to_snapshot_ref: head_snapshot.snapshot_ref.clone(),
            writer_ref: writer_grant.subject_ref.clone(),
            evidence_refs: vec!["source:evidence:signed-update".to_string()],
            witness_refs: vec!["source:witness:runtime".to_string()],
            now,
        },
    )?;
    let import_proof = SourceImportProof {
        kind: Some(RECORD_SOURCE_IMPORT_PROOF.to_string()),
        import_ref: source_ref("import", "initial-pack"),
        source_graph_ref: parent_snapshot.source_graph_ref.clone(),
        tool_ref: "tool:git:pack-import".to_string(),
        input_ref: "git:pack:initial".to_string(),
        output_snapshot_ref: head_snapshot.snapshot_ref.clone(),
        state: SOURCE_IMPORT_STATE_IMPORTED.to_string(),
        imported_object_refs: vec![
            "storage:object:pack-parent".to_string(),
            "storage:object:pack-head".to_string(),
        ],
        evidence_refs: vec!["source:evidence:pack-hash".to_string()],
        blocked_reasons: vec![],
        safe_facts: serde_json::json!({
            "objectCount": 2,
            "format": "git-pack"
        }),
        observed_at: now,
    };
    let graph = SourceVersionGraph {
        kind: Some(RECORD_SOURCE_VERSION_GRAPH.to_string()),
        source_graph_ref: parent_snapshot.source_graph_ref.clone(),
        owner_ref: "identity:root:aux".to_string(),
        storage_backend_ref: "storage:backend:local".to_string(),
        default_branch_ref: source_ref("ref", "main"),
        head_snapshot_ref: head_snapshot.snapshot_ref.clone(),
        state: SOURCE_GRAPH_STATE_READY.to_string(),
        policy: default_policy(),
        branch_refs: vec![source_ref("ref", "main")],
        tag_refs: vec![],
        writer_grant_refs: vec![writer_grant.grant_ref.clone()],
        release_refs: vec![],
        evidence_refs: vec![
            import_proof.import_ref.clone(),
            ref_update.update_ref.clone(),
        ],
        blocked_reasons: vec![],
        issued_at: now,
        expires_at: Some(now + 86_400_000),
    };
    let fixture_storage_edges = source_storage_graph_edges(
        &graph,
        &[parent_snapshot.clone(), head_snapshot.clone()],
        now,
    )?;
    let source_project_operation =
        source_project_operation_for_ref_update(&graph, &ref_update, now)?;
    let host_fabric_contribution = source_content_index_contribution(
        &graph,
        &[parent_snapshot.clone(), head_snapshot.clone()],
        std::slice::from_ref(&ref_update),
        std::slice::from_ref(&import_proof),
        std::slice::from_ref(&source_project_operation),
        &fixture_storage_edges,
        now,
    )?;
    let fixture = SourceGraphFixture {
        graph,
        writer_grant,
        parent_snapshot,
        head_snapshot,
        ref_update,
        import_proof,
        source_project_operation,
        host_fabric_contribution,
    };
    validate_source_graph_fixture(&fixture)?;
    Ok(fixture)
}

pub fn default_source_graph_state(now: u64) -> Result<SourceGraphState> {
    let fixture = build_source_graph_fixture(now)?;
    let mut state = SourceGraphState {
        graph: fixture.graph,
        writer_grants: vec![fixture.writer_grant],
        snapshots: vec![fixture.parent_snapshot, fixture.head_snapshot],
        ref_updates: vec![fixture.ref_update],
        import_proofs: vec![fixture.import_proof],
        source_project_operations: vec![fixture.source_project_operation],
        storage_graph_edges: Vec::new(),
        host_fabric_contributions: vec![fixture.host_fabric_contribution],
        updated_at: now,
    };
    state.storage_graph_edges = source_storage_graph_edges(&state.graph, &state.snapshots, now)?;
    state.host_fabric_contributions =
        vec![source_content_index_contribution_for_state(&state, now)?];
    validate_source_graph_state(&state)?;
    Ok(state)
}

pub fn load_source_graph_state(path: impl AsRef<Path>, now: u64) -> Result<SourceGraphState> {
    let path = path.as_ref();
    if !path.exists() {
        return default_source_graph_state(now);
    }
    let text = fs::read_to_string(path)?;
    let state = serde_json::from_str::<SourceGraphState>(&text)?;
    validate_source_graph_state(&state)?;
    Ok(state)
}

pub fn save_source_graph_state(path: impl AsRef<Path>, state: &SourceGraphState) -> Result<()> {
    validate_source_graph_state(state)?;
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub fn import_snapshot(
    state: &mut SourceGraphState,
    request: SourceImportRequest,
) -> Result<SourceImportOutcome> {
    validate_source_graph_state(state)?;
    if request.storage_object_refs.is_empty() {
        return Err(anyhow!("source import needs storage object refs"));
    }
    if request.signature_refs.is_empty() {
        return Err(anyhow!("source import needs signature refs"));
    }

    let snapshot_ref = source_ref(
        "snapshot",
        &short_ref_id(&format!(
            "{}|{}|{}",
            request.commit_ref, request.tree_ref, request.now
        )),
    );
    let import_ref = source_ref(
        "import",
        &short_ref_id(&format!(
            "{}|{}|{}",
            request.input_ref, snapshot_ref, request.now
        )),
    );
    let snapshot = SourceSnapshot {
        kind: Some(RECORD_SOURCE_SNAPSHOT.to_string()),
        source_graph_ref: state.graph.source_graph_ref.clone(),
        snapshot_ref: snapshot_ref.clone(),
        commit_ref: request.commit_ref,
        tree_ref: request.tree_ref,
        parent_snapshot_refs: request.parent_snapshot_refs,
        storage_object_refs: request.storage_object_refs.clone(),
        author_ref: request.author_ref,
        message_digest_ref: request.message_digest_ref,
        signature_refs: request.signature_refs,
        evidence_refs: request.evidence_refs.clone(),
        issued_at: request.now,
    };
    validate_source_snapshot(&snapshot)?;

    let import_proof = SourceImportProof {
        kind: Some(RECORD_SOURCE_IMPORT_PROOF.to_string()),
        import_ref,
        source_graph_ref: state.graph.source_graph_ref.clone(),
        tool_ref: request.tool_ref,
        input_ref: request.input_ref,
        output_snapshot_ref: snapshot_ref,
        state: SOURCE_IMPORT_STATE_IMPORTED.to_string(),
        imported_object_refs: request.storage_object_refs,
        evidence_refs: request.evidence_refs,
        blocked_reasons: vec![],
        safe_facts: serde_json::json!({
            "snapshotRef": snapshot.snapshot_ref,
            "storageObjectCount": snapshot.storage_object_refs.len()
        }),
        observed_at: request.now,
    };
    validate_source_import_proof(&import_proof)?;

    let storage_graph_edges =
        source_storage_graph_edges_for_snapshot(&state.graph, &snapshot, request.now)?;
    let source_project_operation =
        source_project_operation_for_import(&state.graph, &snapshot, &import_proof, request.now)?;

    state.snapshots.push(snapshot.clone());
    state.import_proofs.push(import_proof.clone());
    state
        .source_project_operations
        .push(source_project_operation.clone());
    state
        .storage_graph_edges
        .extend(storage_graph_edges.clone());
    let host_fabric_contribution = source_content_index_contribution_for_state(state, request.now)?;
    state
        .host_fabric_contributions
        .push(host_fabric_contribution.clone());
    state.updated_at = request.now;
    validate_source_graph_state(state)?;

    Ok(SourceImportOutcome {
        snapshot,
        import_proof,
        source_project_operation,
        storage_graph_edges,
        host_fabric_contribution,
    })
}

pub fn apply_ref_update(
    state: &mut SourceGraphState,
    request: SourceRefUpdateRequest,
) -> Result<SourceRefUpdate> {
    validate_source_graph_state(state)?;
    let update = reduce_ref_update(
        &state.graph,
        &state.writer_grants,
        &state.snapshots,
        request,
    )?;
    if update.state == SOURCE_UPDATE_STATE_APPLIED {
        state.graph.head_snapshot_ref = update.to_snapshot_ref.clone();
        if !state
            .graph
            .evidence_refs
            .iter()
            .any(|value| value == &update.update_ref)
        {
            state.graph.evidence_refs.push(update.update_ref.clone());
        }
        state.graph.issued_at = update.signed_at;
        state.updated_at = update.signed_at;
    }
    state.ref_updates.push(update.clone());
    let source_project_operation =
        source_project_operation_for_ref_update(&state.graph, &update, update.signed_at)?;
    state
        .source_project_operations
        .push(source_project_operation);
    validate_source_graph_state(state)?;
    Ok(update)
}

pub fn link_project_work(
    state: &mut SourceGraphState,
    request: SourceProjectLinkRequest,
) -> Result<SourceProjectLinkOutcome> {
    validate_source_graph_state(state)?;
    if request.project_refs.is_empty() && request.work_item_refs.is_empty() {
        return Err(anyhow!(
            "project link requires project refs or work item refs"
        ));
    }
    if request.actor_ref.trim().is_empty() {
        return Err(anyhow!("project link requires actorRef"));
    }
    if request.evidence_refs.is_empty() {
        return Err(anyhow!("project link requires evidenceRefs"));
    }
    let operation_ref = source_ref(
        "operation",
        &short_ref_id(&format!(
            "{}|{}|{}",
            request.project_refs.join(","),
            request.work_item_refs.join(","),
            request.now
        )),
    );
    let operation = SourceProjectOperation {
        kind: Some(RECORD_SOURCE_PROJECT_OPERATION.to_string()),
        operation_ref,
        source_graph_ref: state.graph.source_graph_ref.clone(),
        subject_ref: state.graph.source_graph_ref.clone(),
        actor_ref: request.actor_ref,
        operation: SOURCE_OPERATION_PROJECT_LINK.to_string(),
        state: SOURCE_PROJECT_OPERATION_STATE_APPLIED.to_string(),
        compatibility_state: SOURCE_PROJECT_COMPATIBILITY_SUPPORTED.to_string(),
        scope_refs: state.graph.branch_refs.clone(),
        source_snapshot_refs: vec![state.graph.head_snapshot_ref.clone()],
        content_index_refs: vec![content_index_ref_for_graph(&state.graph)],
        storage_refs: Vec::new(),
        branch_refs: state.graph.branch_refs.clone(),
        tag_refs: state.graph.tag_refs.clone(),
        release_refs: state.graph.release_refs.clone(),
        project_refs: request.project_refs,
        work_item_refs: request.work_item_refs,
        build_target_refs: Vec::new(),
        build_profile_refs: Vec::new(),
        build_proof_refs: Vec::new(),
        compatibility_refs: vec!["compat:project:workflow-adapter-v1".to_string()],
        proof_refs: Vec::new(),
        evidence_refs: request.evidence_refs,
        rollback_refs: Vec::new(),
        blocked_reasons: Vec::new(),
        safe_facts: serde_json::json!({
            "operation": "projectLink",
            "state": "applied"
        }),
        issued_at: request.now,
        expires_at: request.expires_at,
    };
    validate_source_project_operation(&operation)?;
    state.source_project_operations.push(operation.clone());
    let host_fabric_contribution = source_content_index_contribution_for_state(state, request.now)?;
    state
        .host_fabric_contributions
        .push(host_fabric_contribution.clone());
    state.updated_at = request.now;
    validate_source_graph_state(state)?;
    Ok(SourceProjectLinkOutcome {
        source_project_operation: operation,
        host_fabric_contribution,
    })
}

fn source_project_operation_for_import(
    graph: &SourceVersionGraph,
    snapshot: &SourceSnapshot,
    proof: &SourceImportProof,
    now: u64,
) -> Result<SourceProjectOperation> {
    let operation = SourceProjectOperation {
        kind: Some(RECORD_SOURCE_PROJECT_OPERATION.to_string()),
        operation_ref: source_ref("operation", &short_ref_id(&proof.import_ref)),
        source_graph_ref: graph.source_graph_ref.clone(),
        subject_ref: snapshot.snapshot_ref.clone(),
        actor_ref: snapshot.author_ref.clone(),
        operation: SOURCE_OPERATION_IMPORT.to_string(),
        state: SOURCE_PROJECT_OPERATION_STATE_APPLIED.to_string(),
        compatibility_state: SOURCE_PROJECT_COMPATIBILITY_SUPPORTED.to_string(),
        scope_refs: vec![graph.default_branch_ref.clone()],
        source_snapshot_refs: vec![snapshot.snapshot_ref.clone()],
        content_index_refs: vec![content_index_ref_for_graph(graph)],
        storage_refs: snapshot.storage_object_refs.clone(),
        branch_refs: vec![graph.default_branch_ref.clone()],
        tag_refs: Vec::new(),
        release_refs: graph.release_refs.clone(),
        project_refs: vec!["project:constituency".to_string()],
        work_item_refs: vec!["work-item:git-project-hardening".to_string()],
        build_target_refs: Vec::new(),
        build_profile_refs: Vec::new(),
        build_proof_refs: Vec::new(),
        compatibility_refs: vec!["compat:git:source-import-v1".to_string()],
        proof_refs: vec![proof.import_ref.clone()],
        evidence_refs: proof.evidence_refs.clone(),
        rollback_refs: Vec::new(),
        blocked_reasons: Vec::new(),
        safe_facts: serde_json::json!({
            "operation": "import",
            "storageObjectCount": snapshot.storage_object_refs.len()
        }),
        issued_at: now,
        expires_at: graph.expires_at,
    };
    validate_source_project_operation(&operation)?;
    Ok(operation)
}

fn source_project_operation_for_ref_update(
    graph: &SourceVersionGraph,
    update: &SourceRefUpdate,
    now: u64,
) -> Result<SourceProjectOperation> {
    let operation_state = match update.state.as_str() {
        SOURCE_UPDATE_STATE_APPLIED => SOURCE_PROJECT_OPERATION_STATE_APPLIED,
        SOURCE_UPDATE_STATE_BLOCKED => SOURCE_PROJECT_OPERATION_STATE_BLOCKED,
        SOURCE_UPDATE_STATE_REJECTED => SOURCE_PROJECT_OPERATION_STATE_REJECTED,
        SOURCE_UPDATE_STATE_SUPERSEDED => SOURCE_PROJECT_OPERATION_STATE_SUPERSEDED,
        _ => SOURCE_PROJECT_OPERATION_STATE_REQUESTED,
    };
    let branch_ref = source_ref("ref", &update.ref_name.replace("refs/heads/", ""));
    let operation = SourceProjectOperation {
        kind: Some(RECORD_SOURCE_PROJECT_OPERATION.to_string()),
        operation_ref: source_ref("operation", &short_ref_id(&update.update_ref)),
        source_graph_ref: update.source_graph_ref.clone(),
        subject_ref: branch_ref.clone(),
        actor_ref: update.writer_ref.clone(),
        operation: SOURCE_OPERATION_REF_UPDATE.to_string(),
        state: operation_state.to_string(),
        compatibility_state: SOURCE_PROJECT_COMPATIBILITY_SUPPORTED.to_string(),
        scope_refs: vec![branch_ref.clone()],
        source_snapshot_refs: vec![update.to_snapshot_ref.clone()],
        content_index_refs: vec![content_index_ref_for_graph(graph)],
        storage_refs: Vec::new(),
        branch_refs: vec![branch_ref],
        tag_refs: Vec::new(),
        release_refs: graph.release_refs.clone(),
        project_refs: vec!["project:constituency".to_string()],
        work_item_refs: vec!["work-item:git-project-hardening".to_string()],
        build_target_refs: Vec::new(),
        build_profile_refs: Vec::new(),
        build_proof_refs: Vec::new(),
        compatibility_refs: vec!["compat:git:ref-update-v1".to_string()],
        proof_refs: update.witness_refs.clone(),
        evidence_refs: update.evidence_refs.clone(),
        rollback_refs: update.from_snapshot_ref.clone().into_iter().collect(),
        blocked_reasons: update.blocked_reasons.clone(),
        safe_facts: serde_json::json!({
            "operation": "refUpdate",
            "refName": update.ref_name.clone(),
            "state": update.state.clone()
        }),
        issued_at: now,
        expires_at: update.valid_until,
    };
    validate_source_project_operation(&operation)?;
    Ok(operation)
}

fn content_index_ref_for_graph(graph: &SourceVersionGraph) -> String {
    format!(
        "content-index:source:{}",
        short_ref_id(&graph.source_graph_ref)
    )
}

#[derive(Clone, Debug)]
pub struct SourceRefUpdateOptions {
    pub state: String,
    pub branch: String,
    pub from_snapshot_ref: Option<String>,
    pub to_snapshot_ref: String,
    pub writer_ref: String,
    pub now: u64,
}

#[derive(Clone, Debug)]
pub struct SourceRefUpdateRequest {
    pub branch: String,
    pub from_snapshot_ref: Option<String>,
    pub to_snapshot_ref: String,
    pub writer_ref: String,
    pub evidence_refs: Vec<String>,
    pub witness_refs: Vec<String>,
    pub now: u64,
}

pub fn build_ref_update(options: SourceRefUpdateOptions) -> SourceRefUpdate {
    let blocked_reasons = match options.state.as_str() {
        "blocked" => vec![REASON_FAST_FORWARD_REQUIRED.to_string()],
        "rejected" => vec![REASON_REVIEW_REQUIRED.to_string()],
        _ => vec![],
    };
    SourceRefUpdate {
        kind: Some(RECORD_SOURCE_REF_UPDATE.to_string()),
        update_ref: source_ref("update", &format!("{}-{}", options.branch, options.now)),
        source_graph_ref: source_ref("graph", "constitute-git"),
        ref_name: format!("refs/heads/{}", options.branch),
        ref_kind: SOURCE_REF_KIND_BRANCH.to_string(),
        from_snapshot_ref: options.from_snapshot_ref,
        to_snapshot_ref: options.to_snapshot_ref,
        writer_ref: options.writer_ref,
        state: options.state,
        grant_refs: vec![source_ref("grant", "writer-agent")],
        evidence_refs: vec!["source:evidence:signed-update".to_string()],
        witness_refs: vec!["source:witness:runtime".to_string()],
        blocked_reasons,
        policy: default_policy(),
        signed_at: options.now,
        valid_until: Some(options.now + 3_600_000),
    }
}

pub fn reduce_fixture_ref_update(request: SourceRefUpdateRequest) -> Result<SourceRefUpdate> {
    let fixture = build_source_graph_fixture(request.now)?;
    let mut base_graph = fixture.graph.clone();
    base_graph.head_snapshot_ref = fixture.parent_snapshot.snapshot_ref.clone();
    reduce_ref_update(
        &base_graph,
        &[fixture.writer_grant],
        &[fixture.parent_snapshot, fixture.head_snapshot],
        request,
    )
}

pub fn reduce_ref_update(
    graph: &SourceVersionGraph,
    grants: &[SourceWriterGrant],
    snapshots: &[SourceSnapshot],
    request: SourceRefUpdateRequest,
) -> Result<SourceRefUpdate> {
    validate_source_version_graph(graph)?;
    for grant in grants {
        validate_source_writer_grant(grant)?;
    }
    for snapshot in snapshots {
        validate_source_snapshot(snapshot)?;
    }

    let branch_ref = source_ref("ref", &request.branch);
    let mut blocked_reasons = Vec::new();
    if graph.state != SOURCE_GRAPH_STATE_READY {
        blocked_reasons.push(REASON_GRAPH_NOT_READY.to_string());
    }
    if graph
        .expires_at
        .is_some_and(|expires_at| expires_at <= request.now)
    {
        blocked_reasons.push(REASON_GRAPH_EXPIRED.to_string());
    }
    if !graph.branch_refs.iter().any(|value| value == &branch_ref) {
        blocked_reasons.push(REASON_REF_UNKNOWN.to_string());
    }
    if graph.policy.fast_forward_only {
        match request.from_snapshot_ref.as_deref() {
            Some(from_snapshot_ref) if from_snapshot_ref == graph.head_snapshot_ref => {}
            _ => blocked_reasons.push(REASON_FAST_FORWARD_REQUIRED.to_string()),
        }
    }
    if graph.policy.signed_updates_required && request.evidence_refs.is_empty() {
        blocked_reasons.push(REASON_SIGNED_UPDATE_REQUIRED.to_string());
    }
    if graph.policy.review_required && request.witness_refs.is_empty() {
        blocked_reasons.push(REASON_REVIEW_REQUIRED.to_string());
    }
    if !snapshot_known(snapshots, &graph.source_graph_ref, &request.to_snapshot_ref) {
        blocked_reasons.push(REASON_SNAPSHOT_UNKNOWN.to_string());
    }
    if let Some(from_snapshot_ref) = request.from_snapshot_ref.as_deref() {
        if !snapshot_known(snapshots, &graph.source_graph_ref, from_snapshot_ref) {
            blocked_reasons.push(REASON_SNAPSHOT_UNKNOWN.to_string());
        }
    }

    let usable_grant_refs = usable_source_writer_grant_refs(graph, grants, &request, &branch_ref);
    if usable_grant_refs.is_empty() {
        blocked_reasons.push(REASON_GRANT_UNUSABLE.to_string());
    }
    blocked_reasons.sort();
    blocked_reasons.dedup();

    let update = SourceRefUpdate {
        kind: Some(RECORD_SOURCE_REF_UPDATE.to_string()),
        update_ref: source_ref("update", &format!("{}-{}", request.branch, request.now)),
        source_graph_ref: graph.source_graph_ref.clone(),
        ref_name: format!("refs/heads/{}", request.branch),
        ref_kind: SOURCE_REF_KIND_BRANCH.to_string(),
        from_snapshot_ref: request.from_snapshot_ref,
        to_snapshot_ref: request.to_snapshot_ref,
        writer_ref: request.writer_ref,
        state: if blocked_reasons.is_empty() {
            SOURCE_UPDATE_STATE_APPLIED.to_string()
        } else {
            SOURCE_UPDATE_STATE_BLOCKED.to_string()
        },
        grant_refs: usable_grant_refs,
        evidence_refs: request.evidence_refs,
        witness_refs: request.witness_refs,
        blocked_reasons,
        policy: graph.policy.clone(),
        signed_at: request.now,
        valid_until: Some(request.now + 3_600_000),
    };
    validate_source_ref_update(&update)?;
    Ok(update)
}

fn snapshot_known(
    snapshots: &[SourceSnapshot],
    source_graph_ref: &str,
    snapshot_ref: &str,
) -> bool {
    snapshots.iter().any(|snapshot| {
        snapshot.source_graph_ref == source_graph_ref && snapshot.snapshot_ref == snapshot_ref
    })
}

fn usable_source_writer_grant_refs(
    graph: &SourceVersionGraph,
    grants: &[SourceWriterGrant],
    request: &SourceRefUpdateRequest,
    branch_ref: &str,
) -> Vec<String> {
    grants
        .iter()
        .filter(|grant| grant.source_graph_ref == graph.source_graph_ref)
        .filter(|grant| grant.subject_ref == request.writer_ref)
        .filter(|grant| {
            graph
                .writer_grant_refs
                .iter()
                .any(|value| value == &grant.grant_ref)
        })
        .filter(|grant| {
            grant
                .allowed_operations
                .iter()
                .any(|value| value == SOURCE_OPERATION_PUSH || value == SOURCE_OPERATION_REF_UPDATE)
        })
        .filter(|grant| grant.scope_refs.iter().any(|value| value == branch_ref))
        .filter(|grant| grant.issued_at <= request.now)
        .filter(|grant| {
            grant
                .expires_at
                .is_none_or(|expires_at| expires_at > request.now)
        })
        .filter(|grant| {
            grant
                .revoked_at
                .is_none_or(|revoked_at| revoked_at > request.now)
        })
        .map(|grant| grant.grant_ref.clone())
        .collect()
}

pub fn build_status() -> Result<SourceGraphStatus> {
    source_graph_status(&default_source_graph_state(DEFAULT_NOW)?)
}

pub fn source_graph_status(state: &SourceGraphState) -> Result<SourceGraphStatus> {
    validate_source_graph_state(state)?;
    Ok(SourceGraphStatus {
        source_graph_ref: state.graph.source_graph_ref.clone(),
        head_snapshot_ref: state.graph.head_snapshot_ref.clone(),
        default_branch_ref: state.graph.default_branch_ref.clone(),
        allowed_operations: state.graph.policy.allowed_operations.clone(),
        storage_backend_ref: state.graph.storage_backend_ref.clone(),
        state: state.graph.state.clone(),
        snapshot_count: state.snapshots.len(),
        ref_update_count: state.ref_updates.len(),
        import_proof_count: state.import_proofs.len(),
        source_project_operation_count: state.source_project_operations.len(),
        storage_graph_edge_count: state.storage_graph_edges.len(),
        host_fabric_contribution_count: state.host_fabric_contributions.len(),
    })
}

pub fn validate_source_graph_fixture(fixture: &SourceGraphFixture) -> Result<()> {
    validate_source_version_graph(&fixture.graph)?;
    validate_source_writer_grant(&fixture.writer_grant)?;
    validate_source_snapshot(&fixture.parent_snapshot)?;
    validate_source_snapshot(&fixture.head_snapshot)?;
    validate_source_ref_update(&fixture.ref_update)?;
    validate_source_import_proof(&fixture.import_proof)?;
    validate_host_fabric_member_contribution(&fixture.host_fabric_contribution)?;
    if fixture.graph.source_graph_ref != fixture.head_snapshot.source_graph_ref {
        return Err(anyhow!("graph and head snapshot sourceGraphRef diverge"));
    }
    if fixture.graph.head_snapshot_ref != fixture.head_snapshot.snapshot_ref {
        return Err(anyhow!("graph headSnapshotRef must match head snapshot"));
    }
    if fixture.host_fabric_contribution.role != FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX {
        return Err(anyhow!(
            "source fixture host-fabric contribution must be sourceContentIndex"
        ));
    }
    if fixture.host_fabric_contribution.contract_ref != fixture.graph.source_graph_ref {
        return Err(anyhow!(
            "source fixture host-fabric contribution contract mismatch"
        ));
    }
    Ok(())
}

pub fn validate_source_graph_state(state: &SourceGraphState) -> Result<()> {
    validate_source_version_graph(&state.graph)?;
    for grant in &state.writer_grants {
        validate_source_writer_grant(grant)?;
    }
    for snapshot in &state.snapshots {
        validate_source_snapshot(snapshot)?;
        if snapshot.source_graph_ref != state.graph.source_graph_ref {
            return Err(anyhow!("source state snapshot sourceGraphRef diverges"));
        }
    }
    for update in &state.ref_updates {
        validate_source_ref_update(update)?;
        if update.source_graph_ref != state.graph.source_graph_ref {
            return Err(anyhow!("source state update sourceGraphRef diverges"));
        }
    }
    for proof in &state.import_proofs {
        validate_source_import_proof(proof)?;
        if proof.source_graph_ref != state.graph.source_graph_ref {
            return Err(anyhow!("source state import sourceGraphRef diverges"));
        }
    }
    for operation in &state.source_project_operations {
        validate_source_project_operation(operation)?;
        if operation.source_graph_ref != state.graph.source_graph_ref {
            return Err(anyhow!(
                "source state project operation sourceGraphRef diverges"
            ));
        }
    }
    if !snapshot_known(
        &state.snapshots,
        &state.graph.source_graph_ref,
        &state.graph.head_snapshot_ref,
    ) {
        return Err(anyhow!(
            "source graph head snapshot is not present in state"
        ));
    }
    for edge in &state.storage_graph_edges {
        validate_storage_graph_edge(edge)?;
    }
    for host_fabric_contribution in &state.host_fabric_contributions {
        validate_host_fabric_member_contribution(host_fabric_contribution)?;
        if host_fabric_contribution.contract_ref != state.graph.source_graph_ref {
            return Err(anyhow!(
                "source state host-fabric contribution contract mismatch"
            ));
        }
    }
    if state.updated_at == 0 {
        return Err(anyhow!("source graph state missing updatedAt"));
    }
    Ok(())
}

pub fn source_content_index_contribution_for_state(
    state: &SourceGraphState,
    now: u64,
) -> Result<HostFabricMemberContribution> {
    validate_source_version_graph(&state.graph)?;
    source_content_index_contribution(
        &state.graph,
        &state.snapshots,
        &state.ref_updates,
        &state.import_proofs,
        &state.source_project_operations,
        &state.storage_graph_edges,
        now,
    )
}

pub fn source_content_index_contribution(
    graph: &SourceVersionGraph,
    snapshots: &[SourceSnapshot],
    ref_updates: &[SourceRefUpdate],
    import_proofs: &[SourceImportProof],
    source_project_operations: &[SourceProjectOperation],
    storage_graph_edges: &[StorageGraphEdge],
    now: u64,
) -> Result<HostFabricMemberContribution> {
    validate_source_version_graph(graph)?;
    for snapshot in snapshots {
        validate_source_snapshot(snapshot)?;
    }
    for update in ref_updates {
        validate_source_ref_update(update)?;
    }
    for proof in import_proofs {
        validate_source_import_proof(proof)?;
    }
    for operation in source_project_operations {
        validate_source_project_operation(operation)?;
    }
    for edge in storage_graph_edges {
        validate_storage_graph_edge(edge)?;
    }

    let blocked_reasons = graph.blocked_reasons.clone();
    let ready = graph.state == SOURCE_GRAPH_STATE_READY && blocked_reasons.is_empty();
    let output_refs = [
        vec![graph.head_snapshot_ref.clone()],
        graph.branch_refs.clone(),
        graph.tag_refs.clone(),
        storage_graph_edges
            .iter()
            .map(|edge| edge.edge_id.clone())
            .collect(),
    ]
    .concat();
    let evidence_refs = [
        graph.evidence_refs.clone(),
        import_proofs
            .iter()
            .map(|proof| proof.import_ref.clone())
            .collect(),
        ref_updates
            .iter()
            .map(|update| update.update_ref.clone())
            .collect(),
        source_project_operations
            .iter()
            .map(|operation| operation.operation_ref.clone())
            .collect(),
    ]
    .concat();
    let contribution = build_host_fabric_member_contribution(HostFabricMemberContributionSpec {
        contribution_id: format!(
            "fabric-contribution:source-content-index:{}",
            short_ref_id(&graph.head_snapshot_ref)
        ),
        fabric_ref: DEFAULT_FABRIC_REF.to_string(),
        host_ref: DEFAULT_HOST_REF.to_string(),
        member_ref: DEFAULT_SOURCE_MEMBER_REF.to_string(),
        participant_ref: graph.owner_ref.clone(),
        role: FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX.to_string(),
        role_ref: format!("role:{FABRIC_MEMBER_ROLE_SOURCE_CONTENT_INDEX}"),
        state: if ready {
            FABRIC_MEMBER_CONTRIBUTION_RUNNING.to_string()
        } else {
            FABRIC_MEMBER_CONTRIBUTION_BLOCKED.to_string()
        },
        contract_ref: graph.source_graph_ref.clone(),
        subject_ref: graph.head_snapshot_ref.clone(),
        module_refs: vec![
            "module:source-content-index".to_string(),
            "adapter:git:source-graph".to_string(),
        ],
        source_refs: [
            vec![graph.source_graph_ref.clone(), graph.head_snapshot_ref.clone()],
            graph.branch_refs.clone(),
            graph.tag_refs.clone(),
            snapshots
                .iter()
                .map(|snapshot| snapshot.snapshot_ref.clone())
                .collect(),
            ref_updates
                .iter()
                .map(|update| update.update_ref.clone())
                .collect(),
            import_proofs
                .iter()
                .map(|proof| proof.import_ref.clone())
                .collect(),
        ]
        .concat(),
        capability_refs: vec!["capability:source:content-index".to_string()],
        grant_refs: graph.writer_grant_refs.clone(),
        input_refs: snapshots
            .iter()
            .map(|snapshot| snapshot.snapshot_ref.clone())
            .collect(),
        output_refs,
        evidence_refs,
        lifecycle_plan_refs: vec![format!(
            "lifecycle-plan:source-content-index:{}",
            short_ref_id(&graph.source_graph_ref)
        )],
        release_refs: graph.release_refs.clone(),
        resource_posture: None,
        blocked_reasons,
        safe_facts: serde_json::json!({
            "sourceGraphRef": graph.source_graph_ref,
            "headSnapshotRef": graph.head_snapshot_ref,
            "snapshotCount": snapshots.len(),
            "refUpdateCount": ref_updates.len(),
            "importProofCount": import_proofs.len(),
            "sourceProjectOperationCount": source_project_operations.len(),
            "storageGraphEdgeCount": storage_graph_edges.len()
        }),
        observed_at: now,
        expires_at: graph.expires_at,
    })?;
    validate_host_fabric_member_contribution(&contribution)?;
    Ok(contribution)
}

pub fn source_storage_graph_edges(
    graph: &SourceVersionGraph,
    snapshots: &[SourceSnapshot],
    now: u64,
) -> Result<Vec<StorageGraphEdge>> {
    let mut edges = Vec::new();
    for snapshot in snapshots {
        if snapshot.source_graph_ref != graph.source_graph_ref {
            continue;
        }
        edges.extend(source_storage_graph_edges_for_snapshot(
            graph, snapshot, now,
        )?);
    }
    Ok(edges)
}

fn source_storage_graph_edges_for_snapshot(
    graph: &SourceVersionGraph,
    snapshot: &SourceSnapshot,
    now: u64,
) -> Result<Vec<StorageGraphEdge>> {
    validate_source_version_graph(graph)?;
    validate_source_snapshot(snapshot)?;
    let mut edges = Vec::new();
    for storage_ref in &snapshot.storage_object_refs {
        let edge = StorageGraphEdge {
            edge_id: format!(
                "storage:graph-edge:{}",
                short_ref_id(&format!(
                    "{}|{}|{}",
                    snapshot.snapshot_ref, storage_ref, now
                ))
            ),
            container_id: graph.storage_backend_ref.clone(),
            from_ref: snapshot.snapshot_ref.clone(),
            relation: "sourceSnapshot.stores".to_string(),
            to_ref: storage_ref.clone(),
            detail_ref: None,
            created_at: now,
        };
        validate_storage_graph_edge(&edge)?;
        edges.push(edge);
    }
    Ok(edges)
}

fn short_ref_id(value: &str) -> String {
    sha256_hex(value).chars().take(16).collect()
}

pub fn default_now() -> u64 {
    DEFAULT_NOW
}
