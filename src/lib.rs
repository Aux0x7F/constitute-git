use anyhow::{Result, anyhow};
use constitute_protocol::{
    RECORD_SOURCE_IMPORT_PROOF, RECORD_SOURCE_REF_UPDATE, RECORD_SOURCE_SNAPSHOT,
    RECORD_SOURCE_VERSION_GRAPH, RECORD_SOURCE_WRITER_GRANT, SOURCE_GRAPH_STATE_READY,
    SOURCE_IMPORT_STATE_IMPORTED, SOURCE_OPERATION_FETCH, SOURCE_OPERATION_IMPORT,
    SOURCE_OPERATION_PUSH, SOURCE_OPERATION_REF_UPDATE, SOURCE_OPERATION_STATUS,
    SOURCE_REF_KIND_BRANCH, SOURCE_UPDATE_STATE_APPLIED, SourceGraphPolicy, SourceImportProof,
    SourceRefUpdate, SourceSnapshot, SourceVersionGraph, SourceWriterGrant, source_ref,
    validate_source_import_proof, validate_source_ref_update, validate_source_snapshot,
    validate_source_version_graph, validate_source_writer_grant,
};
use serde::Serialize;

const DEFAULT_NOW: u64 = 1_779_265_000_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGraphFixture {
    pub graph: SourceVersionGraph,
    pub writer_grant: SourceWriterGrant,
    pub parent_snapshot: SourceSnapshot,
    pub head_snapshot: SourceSnapshot,
    pub ref_update: SourceRefUpdate,
    pub import_proof: SourceImportProof,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceGraphStatus {
    pub source_graph_ref: String,
    pub head_snapshot_ref: String,
    pub default_branch_ref: String,
    pub allowed_operations: Vec<String>,
    pub storage_backend_ref: String,
    pub state: String,
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
    let ref_update = build_ref_update(SourceRefUpdateOptions {
        state: SOURCE_UPDATE_STATE_APPLIED.to_string(),
        branch: "main".to_string(),
        from_snapshot_ref: Some(parent_snapshot.snapshot_ref.clone()),
        to_snapshot_ref: head_snapshot.snapshot_ref.clone(),
        writer_ref: writer_grant.subject_ref.clone(),
        now,
    });
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
        evidence_refs: vec![import_proof.import_ref.clone()],
        blocked_reasons: vec![],
        issued_at: now,
        expires_at: Some(now + 86_400_000),
    };
    let fixture = SourceGraphFixture {
        graph,
        writer_grant,
        parent_snapshot,
        head_snapshot,
        ref_update,
        import_proof,
    };
    validate_source_graph_fixture(&fixture)?;
    Ok(fixture)
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

pub fn build_ref_update(options: SourceRefUpdateOptions) -> SourceRefUpdate {
    let blocked_reasons = match options.state.as_str() {
        "blocked" => vec!["source.policy.fastForwardRequired".to_string()],
        "rejected" => vec!["source.policy.reviewRequired".to_string()],
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

pub fn build_status() -> Result<SourceGraphStatus> {
    let fixture = build_source_graph_fixture(DEFAULT_NOW)?;
    Ok(SourceGraphStatus {
        source_graph_ref: fixture.graph.source_graph_ref,
        head_snapshot_ref: fixture.graph.head_snapshot_ref,
        default_branch_ref: fixture.graph.default_branch_ref,
        allowed_operations: fixture.graph.policy.allowed_operations,
        storage_backend_ref: fixture.graph.storage_backend_ref,
        state: fixture.graph.state,
    })
}

pub fn validate_source_graph_fixture(fixture: &SourceGraphFixture) -> Result<()> {
    validate_source_version_graph(&fixture.graph)?;
    validate_source_writer_grant(&fixture.writer_grant)?;
    validate_source_snapshot(&fixture.parent_snapshot)?;
    validate_source_snapshot(&fixture.head_snapshot)?;
    validate_source_ref_update(&fixture.ref_update)?;
    validate_source_import_proof(&fixture.import_proof)?;
    if fixture.graph.source_graph_ref != fixture.head_snapshot.source_graph_ref {
        return Err(anyhow!("graph and head snapshot sourceGraphRef diverge"));
    }
    if fixture.graph.head_snapshot_ref != fixture.head_snapshot.snapshot_ref {
        return Err(anyhow!("graph headSnapshotRef must match head snapshot"));
    }
    Ok(())
}

pub fn default_now() -> u64 {
    DEFAULT_NOW
}
