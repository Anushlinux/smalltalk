use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::super::stable_hash;
use super::checkpoint;
use super::model::ModelTaskHypothesisV1;
use super::observation_packet::{EvidenceHandleV2, ObservationPacketV2};
use super::task_snapshot::{ClaimEvidenceV2, SnapshotSelectionStatusV2, TaskSnapshotV2};

pub(crate) const TASK_THREAD_SCHEMA_V1: &str = "smalltalk.task_thread.v1";
pub(crate) const TASK_THREAD_UPDATE_POLICY_V1: &str = "smalltalk.task_thread_update_policy.v1";
pub(crate) const MAX_CONTEXT_THREADS: usize = 6;
pub(crate) const MAX_BOUNDARY_HYPOTHESES: usize = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskThreadStatusV1 {
    Active,
    Background,
    Interrupted,
    Completed,
    Superseded,
    Unresolved,
}

impl TaskThreadStatusV1 {
    fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Background => "background",
            Self::Interrupted => "interrupted",
            Self::Completed => "completed",
            Self::Superseded => "superseded",
            Self::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ThreadActivityV1 {
    pub(crate) observed_surface: Option<String>,
    pub(crate) immediate_user_operation: Option<String>,
    pub(crate) current_subtask: Option<String>,
    pub(crate) relationship: String,
    pub(crate) observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskThreadHeadV1 {
    pub(crate) schema: String,
    pub(crate) task_thread_id: String,
    pub(crate) identity_token: String,
    pub(crate) revision: i64,
    pub(crate) status: TaskThreadStatusV1,
    pub(crate) head_snapshot_id: String,
    pub(crate) selected_hypothesis_id: String,
    pub(crate) model_response_id: String,
    pub(crate) observation_packet_id: String,
    pub(crate) evidence_watermark: String,
    pub(crate) primary_task_summary: Option<String>,
    pub(crate) task_object: Option<String>,
    pub(crate) first_supported_at_ms: i64,
    pub(crate) last_supported_at_ms: i64,
    pub(crate) current_session_id: String,
    pub(crate) session_lineage: Vec<String>,
    pub(crate) execution_state: String,
    pub(crate) last_meaningful_progress: Option<String>,
    pub(crate) unfinished_state: Option<String>,
    pub(crate) current_activity: ThreadActivityV1,
    pub(crate) superseded_by_thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskThreadRevisionV1 {
    pub(crate) schema: String,
    pub(crate) task_thread_id: String,
    pub(crate) revision: i64,
    pub(crate) prior_revision: Option<i64>,
    pub(crate) status: TaskThreadStatusV1,
    pub(crate) session_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) hypothesis_id: String,
    pub(crate) model_response_id: String,
    pub(crate) packet_id: String,
    pub(crate) evidence_watermark: String,
    pub(crate) relationship: String,
    pub(crate) semantic_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskThreadEdgeV1 {
    pub(crate) edge_id: String,
    pub(crate) from_thread_id: Option<String>,
    pub(crate) from_revision: Option<i64>,
    pub(crate) to_thread_id: String,
    pub(crate) to_revision: i64,
    pub(crate) relationship: String,
    pub(crate) packet_id: String,
    pub(crate) model_response_id: String,
    pub(crate) hypothesis_id: String,
    pub(crate) evidence_refs: Vec<EvidenceHandleV2>,
    pub(crate) semantic_source: String,
    pub(crate) observed_at_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ThreadRejectionCodeV1 {
    SessionMismatch,
    MissingContinuityEdge,
    SupersededThread,
    StaleUnsupportedSnapshot,
    TaskObjectMismatch,
    ConflictingCurrentEvidence,
    CompletedThreadCannotReactivate,
    OutdatedThreadRevision,
}

impl ThreadRejectionCodeV1 {
    fn label(self) -> &'static str {
        match self {
            Self::SessionMismatch => "session_mismatch",
            Self::MissingContinuityEdge => "missing_continuity_edge",
            Self::SupersededThread => "superseded_thread",
            Self::StaleUnsupportedSnapshot => "stale_unsupported_snapshot",
            Self::TaskObjectMismatch => "task_object_mismatch",
            Self::ConflictingCurrentEvidence => "conflicting_current_evidence",
            Self::CompletedThreadCannotReactivate => "completed_thread_cannot_reactivate",
            Self::OutdatedThreadRevision => "outdated_thread_revision",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ThreadBoundaryPersistResultV1 {
    pub(crate) policy_version: String,
    pub(crate) snapshot: TaskSnapshotV2,
    pub(crate) checkpoint: checkpoint::SemanticCheckpointV2,
    pub(crate) selected_thread_id: Option<String>,
    pub(crate) selected_thread_revision: Option<i64>,
    pub(crate) rejection_reason_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct HumanCorrectionPersistenceV1 {
    pub(crate) feedback_id: String,
    pub(crate) decision_id: Option<String>,
    pub(crate) affected_field: String,
    pub(crate) hypothesis_id: Option<String>,
    pub(crate) feedback_kind: String,
    pub(crate) correction_value: Option<String>,
    pub(crate) observed_at_ms: i64,
}

#[derive(Debug)]
struct PlannedThreadUpdate {
    head: Option<TaskThreadHeadV1>,
    revision: Option<TaskThreadRevisionV1>,
    edge: Option<TaskThreadEdgeV1>,
    superseded_head: Option<TaskThreadHeadV1>,
    superseded_revision: Option<TaskThreadRevisionV1>,
    supersession_edge: Option<TaskThreadEdgeV1>,
    rejected: Vec<ThreadRejectionCodeV1>,
}

pub(crate) fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_truth_v2_task_threads (
           task_thread_id TEXT PRIMARY KEY,
           schema_version TEXT NOT NULL,
           identity_token TEXT NOT NULL,
           head_revision INTEGER NOT NULL,
           status TEXT NOT NULL,
           head_snapshot_id TEXT NOT NULL,
           current_session_id TEXT NOT NULL,
           first_supported_at_ms INTEGER NOT NULL,
           last_supported_at_ms INTEGER NOT NULL,
           superseded_by_thread_id TEXT,
           thread_json TEXT NOT NULL,
           updated_at_ms INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_threads_status_time
           ON task_truth_v2_task_threads(status, last_supported_at_ms DESC);
         CREATE INDEX IF NOT EXISTS idx_tt2_threads_session_time
           ON task_truth_v2_task_threads(current_session_id, last_supported_at_ms DESC);

         CREATE TABLE IF NOT EXISTS task_truth_v2_task_thread_revisions (
           task_thread_id TEXT NOT NULL,
           revision INTEGER NOT NULL,
           session_id TEXT NOT NULL,
           snapshot_id TEXT NOT NULL,
           hypothesis_id TEXT NOT NULL,
           model_response_id TEXT NOT NULL,
           packet_id TEXT NOT NULL,
           evidence_watermark TEXT NOT NULL,
           prior_revision INTEGER,
           relationship TEXT NOT NULL,
           status TEXT NOT NULL,
           revision_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           PRIMARY KEY(task_thread_id, revision)
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_thread_revisions_snapshot
           ON task_truth_v2_task_thread_revisions(snapshot_id);

         CREATE TABLE IF NOT EXISTS task_truth_v2_task_thread_edges (
           edge_id TEXT PRIMARY KEY,
           from_thread_id TEXT,
           from_revision INTEGER,
           to_thread_id TEXT NOT NULL,
           to_revision INTEGER NOT NULL,
           relationship TEXT NOT NULL,
           packet_id TEXT NOT NULL,
           model_response_id TEXT NOT NULL,
           hypothesis_id TEXT NOT NULL,
           evidence_json TEXT NOT NULL,
           semantic_source TEXT NOT NULL,
           observed_at_ms INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_thread_edges_to
           ON task_truth_v2_task_thread_edges(to_thread_id, to_revision);

         CREATE TABLE IF NOT EXISTS task_truth_v2_boundary_hypotheses (
           packet_id TEXT NOT NULL,
           hypothesis_id TEXT NOT NULL,
           candidate_thread_id TEXT,
           candidate_thread_revision INTEGER,
           disposition TEXT NOT NULL,
           reason_codes_json TEXT NOT NULL,
           hypothesis_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           PRIMARY KEY(packet_id, hypothesis_id)
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_boundary_hypotheses_thread
           ON task_truth_v2_boundary_hypotheses(candidate_thread_id, candidate_thread_revision);",
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn load_context_heads(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Vec<TaskThreadHeadV1>, String> {
    ensure_schema(conn)?;
    let mut statement = conn
        .prepare(
            "SELECT thread_json FROM task_truth_v2_task_threads
             ORDER BY
               CASE WHEN current_session_id=?1 THEN 0 ELSE 1 END,
               CASE status
                 WHEN 'active' THEN 0 WHEN 'background' THEN 1
                 WHEN 'interrupted' THEN 2 WHEN 'unresolved' THEN 3
                 WHEN 'completed' THEN 4 ELSE 5 END,
               last_supported_at_ms DESC, task_thread_id ASC
             LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![session_id.unwrap_or(""), MAX_CONTEXT_THREADS as i64],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| error.to_string())?
        .map(|row| {
            let raw = row.map_err(|error| error.to_string())?;
            serde_json::from_str(&raw).map_err(|error| error.to_string())
        })
        .collect();
    rows
}

pub(crate) fn load_prior_thread_contexts(
    conn: &Connection,
    session_id: Option<&str>,
    limit: usize,
) -> Result<Vec<super::model::PriorTaskThreadContextV1>, String> {
    let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    Ok(load_context_heads(conn, Some(session_id))?
        .into_iter()
        .take(limit.clamp(1, MAX_CONTEXT_THREADS))
        .map(|head| super::model::PriorTaskThreadContextV1 {
            task_thread_id: head.task_thread_id,
            identity_token: head.identity_token,
            revision: head.revision,
            status: head.status.label().into(),
            current_session_id: head.current_session_id,
            session_lineage: head.session_lineage,
            head_snapshot_id: head.head_snapshot_id,
            task_summary: head.primary_task_summary,
            task_object: head.task_object,
            execution_state: head.execution_state,
            last_meaningful_progress: head.last_meaningful_progress,
            unfinished_state: head.unfinished_state,
            last_supported_at_ms: head.last_supported_at_ms,
        })
        .collect())
}

fn load_head(tx: &Transaction<'_>, thread_id: &str) -> Result<Option<TaskThreadHeadV1>, String> {
    let raw = tx
        .query_row(
            "SELECT thread_json FROM task_truth_v2_task_threads WHERE task_thread_id=?1",
            params![thread_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    raw.map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

fn normalized(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn relationship_evidence_supported(snapshot: &TaskSnapshotV2, prior: &TaskThreadHeadV1) -> bool {
    if snapshot.semantic_source == "human_correction" {
        return true;
    }
    let expected_prior_record = format!("{}:{}", prior.task_thread_id, prior.revision);
    snapshot
        .claim_evidence
        .iter()
        .find(|claim| claim.claim == "relationship_to_prior")
        .is_some_and(|claim| {
            let prior_revision = claim.evidence_refs.iter().any(|reference| {
                reference.source_kind == "prior_thread_revision"
                    && reference.record_id == expected_prior_record
            });
            let current = claim.evidence_refs.iter().any(|reference| {
                !matches!(
                    reference.source_kind.as_str(),
                    "prior_snapshot" | "prior_thread_revision"
                )
            });
            prior_revision && current
        })
}

fn completion_supported(snapshot: &TaskSnapshotV2) -> bool {
    snapshot.semantic_source == "human_correction"
        || snapshot.claim_evidence.iter().any(|claim| {
            claim.claim == "execution_state"
                && !claim.evidence_refs.is_empty()
                && claim.evidence_refs.iter().any(|reference| {
                    !matches!(
                        reference.source_kind.as_str(),
                        "prior_snapshot" | "prior_thread_revision"
                    )
                })
        })
}

fn thread_id(snapshot: &TaskSnapshotV2, session_id: &str) -> String {
    let material = format!(
        "{}:{}:{}:{}",
        session_id,
        snapshot
            .provider_response_id
            .as_deref()
            .unwrap_or("no_response"),
        snapshot
            .selected_hypothesis_id
            .as_deref()
            .unwrap_or("no_hypothesis"),
        snapshot.snapshot_id
    );
    format!("task-thread-{}", stable_hash(material.as_bytes()))
}

fn relationship_evidence(snapshot: &TaskSnapshotV2) -> Vec<EvidenceHandleV2> {
    snapshot
        .claim_evidence
        .iter()
        .find(|claim| claim.claim == "relationship_to_prior")
        .map(|claim| claim.evidence_refs.clone())
        .unwrap_or_default()
}

fn plan_update(
    tx: &Transaction<'_>,
    packet: &ObservationPacketV2,
    snapshot: &mut TaskSnapshotV2,
) -> Result<PlannedThreadUpdate, String> {
    let session_id = packet
        .session_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "task thread update requires session scope".to_string())?;
    snapshot.session_id = Some(session_id.to_string());
    snapshot.packet_id = packet.packet_id.clone();
    snapshot.observed_at_ms = packet.observed_at_ms;
    snapshot.evidence_watermark = packet.evidence_watermark.clone();
    let is_selected_task_rejection = snapshot.semantic_source == "human_correction"
        && snapshot.relation_to_prior == "rejected_selected_task"
        && snapshot.continuity_thread_id.is_some();
    if !is_selected_task_rejection
        && (snapshot.selection_status == SnapshotSelectionStatusV2::Unresolved
            || snapshot.task_summary.is_none()
            || snapshot.semantic_source == "unresolved")
    {
        snapshot.task_thread_id = None;
        snapshot.task_thread_revision = None;
        snapshot.thread_status = TaskThreadStatusV1::Unresolved.label().into();
        return Ok(PlannedThreadUpdate {
            head: None,
            revision: None,
            edge: None,
            superseded_head: None,
            superseded_revision: None,
            supersession_edge: None,
            rejected: vec![ThreadRejectionCodeV1::StaleUnsupportedSnapshot],
        });
    }

    let relationship = snapshot.relation_to_prior.as_str();
    let creates_new = relationship == "new_task";
    let requires_continuity = matches!(
        relationship,
        "continuation"
            | "supporting_research"
            | "verification"
            | "temporary_detour"
            | "interruption"
            | "return_to_prior_task"
            | "rejected_selected_task"
    ) || (relationship == "unrelated_or_unknown"
        && snapshot.semantic_source == "human_correction");
    let mut rejected = Vec::new();
    let prior_head = snapshot
        .continuity_thread_id
        .as_deref()
        .map(|thread_id| load_head(tx, thread_id))
        .transpose()?
        .flatten();

    if requires_continuity && prior_head.is_none() {
        rejected.push(ThreadRejectionCodeV1::MissingContinuityEdge);
    }
    if let Some(prior) = prior_head.as_ref() {
        if snapshot.continuity_thread_revision != Some(prior.revision) {
            rejected.push(ThreadRejectionCodeV1::OutdatedThreadRevision);
        }
        if snapshot.continuity_identity_token.as_deref() != Some(prior.identity_token.as_str()) {
            rejected.push(ThreadRejectionCodeV1::MissingContinuityEdge);
        }
        if prior.status == TaskThreadStatusV1::Superseded {
            rejected.push(ThreadRejectionCodeV1::SupersededThread);
        }
        if prior.status == TaskThreadStatusV1::Completed
            && relationship != "return_to_prior_task"
            && snapshot.semantic_source != "human_correction"
        {
            rejected.push(ThreadRejectionCodeV1::CompletedThreadCannotReactivate);
        }
        if prior.current_session_id != session_id
            && !relationship_evidence_supported(snapshot, prior)
        {
            rejected.push(ThreadRejectionCodeV1::SessionMismatch);
        }
        if normalized(prior.task_object.as_deref()) != normalized(snapshot.task_object.as_deref())
            && prior.task_object.is_some()
            && snapshot.task_object.is_some()
        {
            rejected.push(ThreadRejectionCodeV1::TaskObjectMismatch);
        }
    }
    if requires_continuity
        && prior_head
            .as_ref()
            .is_some_and(|prior| !relationship_evidence_supported(snapshot, prior))
    {
        rejected.push(ThreadRejectionCodeV1::MissingContinuityEdge);
    }
    let superseded_head = if creates_new {
        snapshot
            .supersedes_thread_id
            .as_deref()
            .map(|thread_id| load_head(tx, thread_id))
            .transpose()?
            .flatten()
    } else {
        None
    };
    if creates_new && snapshot.supersedes_thread_id.is_some() {
        match superseded_head.as_ref() {
            None => rejected.push(ThreadRejectionCodeV1::MissingContinuityEdge),
            Some(prior) if prior.status == TaskThreadStatusV1::Superseded => {
                rejected.push(ThreadRejectionCodeV1::SupersededThread);
            }
            Some(prior) if !relationship_evidence_supported(snapshot, prior) => {
                rejected.push(ThreadRejectionCodeV1::MissingContinuityEdge);
            }
            Some(_) => {}
        }
    }
    if !creates_new && !requires_continuity {
        rejected.push(ThreadRejectionCodeV1::ConflictingCurrentEvidence);
    }
    rejected.sort_by_key(|reason| reason.label());
    rejected.dedup();
    if !rejected.is_empty() {
        snapshot.selection_status = SnapshotSelectionStatusV2::Unresolved;
        snapshot.task_thread_id = None;
        snapshot.task_thread_revision = None;
        snapshot.thread_status = TaskThreadStatusV1::Unresolved.label().into();
        return Ok(PlannedThreadUpdate {
            head: None,
            revision: None,
            edge: None,
            superseded_head: None,
            superseded_revision: None,
            supersession_edge: None,
            rejected,
        });
    }

    let task_thread_id = if creates_new {
        thread_id(snapshot, session_id)
    } else {
        prior_head
            .as_ref()
            .expect("continuity head validated")
            .task_thread_id
            .clone()
    };
    let revision = prior_head
        .as_ref()
        .map(|head| head.revision + 1)
        .unwrap_or(1);
    let mut status = match relationship {
        "interruption" | "unrelated_or_unknown" => TaskThreadStatusV1::Interrupted,
        "rejected_selected_task" => TaskThreadStatusV1::Unresolved,
        _ => TaskThreadStatusV1::Active,
    };
    if snapshot.execution_state == "completed" {
        if completion_supported(snapshot) {
            status = TaskThreadStatusV1::Completed;
        } else {
            snapshot.selection_status = SnapshotSelectionStatusV2::Unresolved;
            snapshot.task_thread_id = None;
            snapshot.task_thread_revision = None;
            snapshot.thread_status = TaskThreadStatusV1::Unresolved.label().into();
            return Ok(PlannedThreadUpdate {
                head: None,
                revision: None,
                edge: None,
                superseded_head: None,
                superseded_revision: None,
                supersession_edge: None,
                rejected: vec![ThreadRejectionCodeV1::ConflictingCurrentEvidence],
            });
        }
    }
    snapshot.task_thread_id = Some(task_thread_id.clone());
    snapshot.task_thread_revision = Some(revision);
    snapshot.thread_status = status.label().into();
    if matches!(
        relationship,
        "supporting_research" | "verification" | "temporary_detour" | "interruption"
    ) {
        if let Some(prior) = prior_head.as_ref() {
            // The model-declared relationship says this is activity within the
            // existing task. Keep the prior thread's primary identity atomic;
            // the new surface/operation/subtask is stored as current activity.
            snapshot.task_summary = prior.primary_task_summary.clone();
            snapshot.task_object = prior.task_object.clone();
        }
    }
    let selected_hypothesis_id = snapshot.selected_hypothesis_id.clone().unwrap_or_else(|| {
        format!(
            "hypothesis-{}",
            stable_hash(snapshot.snapshot_id.as_bytes())
        )
    });
    let model_response_id = snapshot
        .provider_response_id
        .clone()
        .unwrap_or_else(|| "human_or_unavailable_response".into());
    let mut session_lineage = prior_head
        .as_ref()
        .map(|head| head.session_lineage.clone())
        .unwrap_or_default();
    if !session_lineage.iter().any(|value| value == session_id) {
        session_lineage.push(session_id.to_string());
    }
    let head = TaskThreadHeadV1 {
        schema: TASK_THREAD_SCHEMA_V1.into(),
        task_thread_id: task_thread_id.clone(),
        identity_token: prior_head
            .as_ref()
            .map(|head| head.identity_token.clone())
            .unwrap_or_else(|| {
                format!(
                    "thread-identity-{}",
                    stable_hash(format!("{}:{}", task_thread_id, snapshot.snapshot_id).as_bytes())
                )
            }),
        revision,
        status,
        head_snapshot_id: snapshot.snapshot_id.clone(),
        selected_hypothesis_id: selected_hypothesis_id.clone(),
        model_response_id: model_response_id.clone(),
        observation_packet_id: packet.packet_id.clone(),
        evidence_watermark: packet.evidence_watermark.clone(),
        primary_task_summary: snapshot.task_summary.clone(),
        task_object: snapshot.task_object.clone(),
        first_supported_at_ms: prior_head
            .as_ref()
            .map(|head| head.first_supported_at_ms)
            .unwrap_or(packet.observed_at_ms),
        last_supported_at_ms: packet.observed_at_ms,
        current_session_id: session_id.to_string(),
        session_lineage,
        execution_state: snapshot.execution_state.clone(),
        last_meaningful_progress: snapshot.last_meaningful_progress.clone(),
        unfinished_state: snapshot.unfinished_step.clone(),
        current_activity: ThreadActivityV1 {
            observed_surface: snapshot.observed_surface.clone(),
            immediate_user_operation: snapshot.immediate_user_operation.clone(),
            current_subtask: snapshot.current_subtask.clone(),
            relationship: relationship.into(),
            observed_at_ms: packet.observed_at_ms,
        },
        superseded_by_thread_id: None,
    };
    let revision_record = TaskThreadRevisionV1 {
        schema: "smalltalk.task_thread_revision.v1".into(),
        task_thread_id: task_thread_id.clone(),
        revision,
        prior_revision: prior_head.as_ref().map(|head| head.revision),
        status,
        session_id: session_id.to_string(),
        snapshot_id: snapshot.snapshot_id.clone(),
        hypothesis_id: selected_hypothesis_id.clone(),
        model_response_id: model_response_id.clone(),
        packet_id: packet.packet_id.clone(),
        evidence_watermark: packet.evidence_watermark.clone(),
        relationship: relationship.into(),
        semantic_source: snapshot.semantic_source.clone(),
    };
    let edge = prior_head.as_ref().map(|prior| {
        let seed = format!(
            "{}:{}:{}:{}:{}",
            prior.task_thread_id, prior.revision, task_thread_id, revision, packet.packet_id
        );
        TaskThreadEdgeV1 {
            edge_id: format!("task-thread-edge-{}", stable_hash(seed.as_bytes())),
            from_thread_id: Some(prior.task_thread_id.clone()),
            from_revision: Some(prior.revision),
            to_thread_id: task_thread_id.clone(),
            to_revision: revision,
            relationship: relationship.into(),
            packet_id: packet.packet_id.clone(),
            model_response_id: model_response_id.clone(),
            hypothesis_id: selected_hypothesis_id.clone(),
            evidence_refs: relationship_evidence(snapshot),
            semantic_source: snapshot.semantic_source.clone(),
            observed_at_ms: packet.observed_at_ms,
        }
    });
    let (superseded_head, superseded_revision, supersession_edge) =
        if let Some(mut superseded) = superseded_head {
            let superseded_revision_number = superseded.revision + 1;
            let prior_revision = superseded.revision;
            let prior_snapshot_id = superseded.head_snapshot_id.clone();
            superseded.revision = superseded_revision_number;
            superseded.status = TaskThreadStatusV1::Superseded;
            superseded.selected_hypothesis_id = selected_hypothesis_id.clone();
            superseded.model_response_id = model_response_id.clone();
            superseded.observation_packet_id = packet.packet_id.clone();
            superseded.evidence_watermark = packet.evidence_watermark.clone();
            superseded.last_supported_at_ms = packet.observed_at_ms;
            superseded.superseded_by_thread_id = Some(task_thread_id.clone());
            if !superseded
                .session_lineage
                .iter()
                .any(|value| value == session_id)
            {
                superseded.session_lineage.push(session_id.to_string());
            }
            let status_revision = TaskThreadRevisionV1 {
                schema: "smalltalk.task_thread_revision.v1".into(),
                task_thread_id: superseded.task_thread_id.clone(),
                revision: superseded_revision_number,
                prior_revision: Some(prior_revision),
                status: TaskThreadStatusV1::Superseded,
                session_id: session_id.to_string(),
                // The old thread keeps its last coherent semantic snapshot.
                // The current packet/response and the supersession edge below
                // carry the evidence for this status-only revision.
                snapshot_id: prior_snapshot_id,
                hypothesis_id: selected_hypothesis_id.clone(),
                model_response_id: model_response_id.clone(),
                packet_id: packet.packet_id.clone(),
                evidence_watermark: packet.evidence_watermark.clone(),
                relationship: "superseded_by_new_task".into(),
                semantic_source: snapshot.semantic_source.clone(),
            };
            let seed = format!(
                "{}:{}:{}:{}:{}:supersedes",
                superseded.task_thread_id,
                superseded_revision_number,
                task_thread_id,
                revision,
                packet.packet_id
            );
            let supersession_edge = TaskThreadEdgeV1 {
                edge_id: format!("task-thread-edge-{}", stable_hash(seed.as_bytes())),
                from_thread_id: Some(superseded.task_thread_id.clone()),
                from_revision: Some(superseded_revision_number),
                to_thread_id: task_thread_id.clone(),
                to_revision: revision,
                relationship: "superseded_by_new_task".into(),
                packet_id: packet.packet_id.clone(),
                model_response_id: model_response_id.clone(),
                hypothesis_id: selected_hypothesis_id.clone(),
                evidence_refs: relationship_evidence(snapshot),
                semantic_source: snapshot.semantic_source.clone(),
                observed_at_ms: packet.observed_at_ms,
            };
            (
                Some(superseded),
                Some(status_revision),
                Some(supersession_edge),
            )
        } else {
            (None, None, None)
        };
    Ok(PlannedThreadUpdate {
        head: Some(head),
        revision: Some(revision_record),
        edge,
        superseded_head,
        superseded_revision,
        supersession_edge,
        rejected,
    })
}

fn persist_head(tx: &Transaction<'_>, head: &TaskThreadHeadV1) -> Result<(), String> {
    tx.execute(
        "INSERT INTO task_truth_v2_task_threads (
           task_thread_id, schema_version, identity_token, head_revision, status, head_snapshot_id,
           current_session_id, first_supported_at_ms, last_supported_at_ms,
           superseded_by_thread_id, thread_json, updated_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?9)
         ON CONFLICT(task_thread_id) DO UPDATE SET
           head_revision=excluded.head_revision, status=excluded.status,
           head_snapshot_id=excluded.head_snapshot_id,
           current_session_id=excluded.current_session_id,
           first_supported_at_ms=excluded.first_supported_at_ms,
           last_supported_at_ms=excluded.last_supported_at_ms,
           superseded_by_thread_id=excluded.superseded_by_thread_id,
           thread_json=excluded.thread_json, updated_at_ms=excluded.updated_at_ms",
        params![
            head.task_thread_id,
            head.schema,
            head.identity_token,
            head.revision,
            head.status.label(),
            head.head_snapshot_id,
            head.current_session_id,
            head.first_supported_at_ms,
            head.last_supported_at_ms,
            head.superseded_by_thread_id,
            serde_json::to_string(head).map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn persist_revision(tx: &Transaction<'_>, revision: &TaskThreadRevisionV1) -> Result<(), String> {
    tx.execute(
        "INSERT INTO task_truth_v2_task_thread_revisions (
           task_thread_id, revision, session_id, snapshot_id, hypothesis_id,
           model_response_id, packet_id, evidence_watermark, prior_revision,
           relationship, status, revision_json, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,
                   CAST(strftime('%s','now') AS INTEGER) * 1000)",
        params![
            revision.task_thread_id,
            revision.revision,
            revision.session_id,
            revision.snapshot_id,
            revision.hypothesis_id,
            revision.model_response_id,
            revision.packet_id,
            revision.evidence_watermark,
            revision.prior_revision,
            revision.relationship,
            revision.status.label(),
            serde_json::to_string(revision).map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn persist_edge(tx: &Transaction<'_>, edge: &TaskThreadEdgeV1) -> Result<(), String> {
    tx.execute(
        "INSERT INTO task_truth_v2_task_thread_edges (
           edge_id, from_thread_id, from_revision, to_thread_id, to_revision,
           relationship, packet_id, model_response_id, hypothesis_id,
           evidence_json, semantic_source, observed_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            edge.edge_id,
            edge.from_thread_id,
            edge.from_revision,
            edge.to_thread_id,
            edge.to_revision,
            edge.relationship,
            edge.packet_id,
            edge.model_response_id,
            edge.hypothesis_id,
            serde_json::to_string(&edge.evidence_refs).map_err(|error| error.to_string())?,
            edge.semantic_source,
            edge.observed_at_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn demote_other_active_threads(
    tx: &Transaction<'_>,
    session_id: &str,
    selected_thread_id: &str,
    now_ms: i64,
) -> Result<(), String> {
    let mut statement = tx
        .prepare(
            "SELECT thread_json FROM task_truth_v2_task_threads
             WHERE current_session_id=?1 AND status='active' AND task_thread_id<>?2",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![session_id, selected_thread_id], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for raw in rows {
        let mut head: TaskThreadHeadV1 =
            serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        head.status = TaskThreadStatusV1::Background;
        head.last_supported_at_ms = head.last_supported_at_ms.min(now_ms);
        persist_head(tx, &head)?;
    }
    Ok(())
}

fn persist_boundary_hypotheses(
    tx: &Transaction<'_>,
    packet: &ObservationPacketV2,
    snapshot: &TaskSnapshotV2,
    reason_codes: &[String],
) -> Result<(), String> {
    let selected_id = snapshot.selected_hypothesis_id.clone().unwrap_or_else(|| {
        format!(
            "hypothesis-{}",
            stable_hash(snapshot.snapshot_id.as_bytes())
        )
    });
    tx.execute(
        "INSERT OR REPLACE INTO task_truth_v2_boundary_hypotheses (
           packet_id, hypothesis_id, candidate_thread_id, candidate_thread_revision,
           disposition, reason_codes_json, hypothesis_json, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            packet.packet_id,
            selected_id,
            snapshot.task_thread_id,
            snapshot.task_thread_revision,
            if snapshot.selection_status == SnapshotSelectionStatusV2::Unresolved {
                "unresolved"
            } else {
                "selected"
            },
            serde_json::to_string(reason_codes).map_err(|error| error.to_string())?,
            serde_json::to_string(snapshot).map_err(|error| error.to_string())?,
            packet.observed_at_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    for hypothesis in snapshot
        .alternative_hypotheses
        .iter()
        .take(MAX_BOUNDARY_HYPOTHESES.saturating_sub(1))
    {
        tx.execute(
            "INSERT OR REPLACE INTO task_truth_v2_boundary_hypotheses (
               packet_id, hypothesis_id, candidate_thread_id, candidate_thread_revision,
               disposition, reason_codes_json, hypothesis_json, created_at_ms
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                packet.packet_id,
                hypothesis.hypothesis_id,
                hypothesis.task_thread_id,
                hypothesis.task_thread_revision,
                "retained_alternative",
                serde_json::to_string(&hypothesis.reason_codes)
                    .map_err(|error| error.to_string())?,
                serde_json::to_string(hypothesis).map_err(|error| error.to_string())?,
                packet.observed_at_ms,
            ],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn persist_boundary_in_transaction(
    tx: &Transaction<'_>,
    packet: &ObservationPacketV2,
    mut snapshot: TaskSnapshotV2,
) -> Result<ThreadBoundaryPersistResultV1, String> {
    let plan = plan_update(tx, packet, &mut snapshot)?;
    let reason_codes = plan
        .rejected
        .iter()
        .map(|reason| reason.label().to_string())
        .collect::<Vec<_>>();

    if let Some(head) = plan.head.as_ref() {
        demote_other_active_threads(
            tx,
            &head.current_session_id,
            &head.task_thread_id,
            packet.observed_at_ms,
        )?;
        persist_head(tx, head)?;
    }
    if let Some(revision) = plan.revision.as_ref() {
        persist_revision(tx, revision)?;
    }
    if let Some(edge) = plan.edge.as_ref() {
        persist_edge(tx, edge)?;
    }
    if let Some(head) = plan.superseded_head.as_ref() {
        persist_head(tx, head)?;
    }
    if let Some(revision) = plan.superseded_revision.as_ref() {
        persist_revision(tx, revision)?;
    }
    if let Some(edge) = plan.supersession_edge.as_ref() {
        persist_edge(tx, edge)?;
    }
    persist_boundary_hypotheses(tx, packet, &snapshot, &reason_codes)?;
    let semantic_checkpoint = checkpoint::persist_checkpoint(tx, packet, &snapshot)?;
    Ok(ThreadBoundaryPersistResultV1 {
        policy_version: TASK_THREAD_UPDATE_POLICY_V1.into(),
        selected_thread_id: snapshot.task_thread_id.clone(),
        selected_thread_revision: snapshot.task_thread_revision,
        snapshot,
        checkpoint: semantic_checkpoint,
        rejection_reason_codes: reason_codes,
    })
}

pub(crate) fn persist_boundary_atomic(
    conn: &Connection,
    packet: &ObservationPacketV2,
    snapshot: TaskSnapshotV2,
) -> Result<ThreadBoundaryPersistResultV1, String> {
    checkpoint::ensure_schema(conn)?;
    ensure_schema(conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let result = persist_boundary_in_transaction(&tx, packet, snapshot)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

fn apply_model_hypothesis(snapshot: &mut TaskSnapshotV2, model: ModelTaskHypothesisV1) {
    snapshot.claim_evidence = model
        .claim_evidence
        .iter()
        .filter_map(|(field, claim)| claim.as_ref().map(|claim| (field, claim)))
        .map(|(field, claim)| ClaimEvidenceV2 {
            claim: field.clone(),
            evidence_refs: claim.evidence_refs.clone(),
            confidence: claim.confidence,
            source_confidence: std::collections::BTreeMap::from([(
                "multimodal_model".into(),
                claim.confidence,
            )]),
        })
        .collect();
    snapshot.confidence_by_field = model.confidence_by_field.clone();
    snapshot
        .confidence_by_source
        .insert("human_selected_model_hypothesis".into(), model.confidence);
    snapshot.observed_surface = model.observed_surface;
    snapshot.immediate_user_operation = model.immediate_user_operation;
    snapshot.semantic_effect_of_operation = model.semantic_effect_of_operation;
    snapshot.current_subtask = model.current_subtask;
    snapshot.task_summary = model.likely_primary_task;
    snapshot.task_object = model.task_object;
    snapshot.app_identity = model.app_identity;
    snapshot.surface_identity_hash = model.surface_identity_hash;
    snapshot.document_or_thread_identity_hash = model.document_or_thread_identity_hash;
    snapshot.execution_state = model.execution_state.unwrap_or_else(|| "unclear".into());
    snapshot.current_actor = model.current_actor.unwrap_or_else(|| "unknown".into());
    snapshot.waiting_on = model.waiting_on.unwrap_or_else(|| "unknown".into());
    snapshot.last_meaningful_progress = model.last_meaningful_progress;
    snapshot.unfinished_step = model.unfinished_state;
    snapshot.next_action = model.possible_next_action;
    snapshot.relation_to_prior = model.relationship_to_prior.label().into();
    snapshot.continuity_thread_id = model.continuity_thread_id;
    snapshot.continuity_thread_revision = model.continuity_thread_revision;
    snapshot.continuity_identity_token = model.continuity_identity_token;
    snapshot.supersedes_thread_id = model.supersedes_thread_id;
}

fn attach_human_correction_claim(snapshot: &mut TaskSnapshotV2, field: &str, feedback_id: &str) {
    snapshot.claim_evidence.retain(|claim| claim.claim != field);
    snapshot.claim_evidence.push(ClaimEvidenceV2 {
        claim: field.into(),
        evidence_refs: vec![EvidenceHandleV2 {
            source_kind: "human_correction".into(),
            record_id: feedback_id.into(),
            frame_id: None,
            content_hash: None,
        }],
        confidence: 1.0,
        source_confidence: std::collections::BTreeMap::from([("human_correction".into(), 1.0)]),
    });
    snapshot.confidence_by_field.insert(field.into(), 1.0);
    snapshot
        .confidence_by_source
        .insert("human_correction".into(), 1.0);
}

fn corrected_snapshot(
    original: &TaskSnapshotV2,
    correction: &HumanCorrectionPersistenceV1,
) -> Result<TaskSnapshotV2, String> {
    let mut corrected = original.clone();
    corrected.snapshot_id = format!(
        "snapshot-human-{}",
        stable_hash(
            format!(
                "{}:{}:{}",
                original.snapshot_id, original.revision, correction.feedback_id
            )
            .as_bytes()
        )
    );
    corrected.revision = original.revision + 1;
    corrected.prior_snapshot_id = Some(original.snapshot_id.clone());
    corrected.supersedes_snapshot_id = Some(original.snapshot_id.clone());
    corrected.observed_at_ms = correction.observed_at_ms;
    corrected.semantic_source = "human_correction".into();
    corrected.wording_source = "deterministic".into();
    corrected.inference_status = "human_corrected".into();
    corrected.selection_status = SnapshotSelectionStatusV2::Selected;
    corrected.continuity_thread_id = original.task_thread_id.clone();
    corrected.continuity_thread_revision = original.task_thread_revision;

    if original.task_thread_id.is_none() {
        return Err("human correction requires a thread-owned snapshot".into());
    }
    if correction.affected_field == "hypothesis" && correction.feedback_kind == "corrected" {
        let hypothesis_id = correction
            .hypothesis_id
            .as_deref()
            .ok_or_else(|| "hypothesis correction requires a hypothesis id".to_string())?;
        let selected = original
            .alternative_hypotheses
            .iter()
            .find(|hypothesis| hypothesis.hypothesis_id == hypothesis_id)
            .ok_or_else(|| "corrected hypothesis is not retained on the snapshot".to_string())?;
        let payload = selected.semantic_payload.clone().ok_or_else(|| {
            "corrected hypothesis is missing its atomic semantic payload".to_string()
        })?;
        let model: ModelTaskHypothesisV1 =
            serde_json::from_value(payload).map_err(|error| error.to_string())?;
        apply_model_hypothesis(&mut corrected, model);
        corrected.selected_hypothesis_id = Some(hypothesis_id.to_string());
        corrected
            .alternative_hypotheses
            .retain(|hypothesis| hypothesis.hypothesis_id != hypothesis_id);
    } else if correction.affected_field == "relationship" {
        corrected.relation_to_prior = correction
            .correction_value
            .clone()
            .ok_or_else(|| "relationship correction requires a value".to_string())?;
        attach_human_correction_claim(
            &mut corrected,
            "relationship_to_prior",
            &correction.feedback_id,
        );
    } else if correction.affected_field == "task_status" {
        match correction.correction_value.as_deref() {
            Some("completed") => {
                corrected.execution_state = "completed".into();
                corrected.relation_to_prior = "continuation".into();
                corrected.unfinished_step = None;
                corrected.next_action = None;
            }
            Some("reactivated") => {
                corrected.execution_state = "active".into();
                corrected.relation_to_prior = "return_to_prior_task".into();
            }
            _ => return Err("task-status correction requires a supported value".into()),
        }
        attach_human_correction_claim(&mut corrected, "execution_state", &correction.feedback_id);
    } else if correction.affected_field == "task_summary" && correction.feedback_kind == "rejected"
    {
        let selected_hypothesis_id = original
            .selected_hypothesis_id
            .as_deref()
            .ok_or_else(|| "selected-task rejection requires a selected hypothesis".to_string())?;
        if correction.hypothesis_id.as_deref() != Some(selected_hypothesis_id) {
            return Err(
                "selected-task rejection must name the exact selected hypothesis".to_string(),
            );
        }
        corrected.selection_status = SnapshotSelectionStatusV2::Unresolved;
        corrected.thread_status = TaskThreadStatusV1::Unresolved.label().into();
        corrected.relation_to_prior = "rejected_selected_task".into();
        corrected.task_summary = None;
        corrected.task_object = None;
        corrected.user_goal = None;
        corrected.last_meaningful_progress = None;
        corrected.unfinished_step = None;
        corrected.next_action = None;
        corrected.return_anchor_candidate_id = None;
        corrected.return_anchor_status = super::task_snapshot::ReturnAnchorStatusV2::Unresolved;
        corrected.inference_status = "human_rejected_selected_task".into();
        attach_human_correction_claim(&mut corrected, "task_summary", &correction.feedback_id);
    } else {
        return Err("feedback does not create a semantic task-thread correction".into());
    }
    corrected
        .provenance
        .push(format!("human_correction:{}", correction.feedback_id));
    Ok(corrected)
}

pub(crate) fn persist_human_correction_atomic(
    conn: &Connection,
    original: &TaskSnapshotV2,
    correction: &HumanCorrectionPersistenceV1,
) -> Result<ThreadBoundaryPersistResultV1, String> {
    checkpoint::ensure_schema(conn)?;
    ensure_schema(conn)?;
    let packet_raw = conn
        .query_row(
            "SELECT packet_json FROM task_truth_v2_observation_packets WHERE packet_id=?1",
            params![original.packet_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "human correction source packet is missing".to_string())?;
    let mut packet: ObservationPacketV2 =
        serde_json::from_str(&packet_raw).map_err(|error| error.to_string())?;
    packet.packet_id = format!(
        "packet-human-{}",
        stable_hash(format!("{}:{}", original.packet_id, correction.feedback_id).as_bytes())
    );
    packet.observed_at_ms = correction.observed_at_ms;
    packet.evidence_watermark = stable_hash(
        format!(
            "{}:{}:{}",
            original.evidence_watermark, correction.feedback_id, correction.observed_at_ms
        )
        .as_bytes(),
    );
    packet.previous_valid_snapshot_id = Some(original.snapshot_id.clone());
    packet.evidence_quality = "human_correction".into();
    let mut snapshot = corrected_snapshot(original, correction)?;
    if let Some(thread_id) = snapshot.continuity_thread_id.as_deref() {
        let head_raw = conn
            .query_row(
                "SELECT thread_json FROM task_truth_v2_task_threads WHERE task_thread_id=?1",
                params![thread_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "human correction task-thread head is missing".to_string())?;
        let head: TaskThreadHeadV1 =
            serde_json::from_str(&head_raw).map_err(|error| error.to_string())?;
        snapshot.continuity_identity_token = Some(head.identity_token);
    }
    snapshot.packet_id = packet.packet_id.clone();
    snapshot.evidence_watermark = packet.evidence_watermark.clone();

    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let result = persist_boundary_in_transaction(&tx, &packet, snapshot)?;
    let thread_id = result
        .selected_thread_id
        .as_deref()
        .ok_or_else(|| "human correction did not produce a selected task thread".to_string())?;
    let thread_revision = result
        .selected_thread_revision
        .ok_or_else(|| "human correction did not produce a task thread revision".to_string())?;
    tx.execute(
        "INSERT INTO task_truth_v2_feedback_events (
           feedback_id, task_thread_id, task_thread_revision,
           task_snapshot_id, task_snapshot_revision,
           corrected_snapshot_id, corrected_snapshot_revision,
           affected_field, hypothesis_id, evidence_watermark, correction_value,
           feedback_kind, decision_id, observed_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            correction.feedback_id,
            thread_id,
            thread_revision,
            original.snapshot_id,
            original.revision,
            result.snapshot.snapshot_id,
            result.snapshot.revision,
            correction.affected_field,
            correction.hypothesis_id,
            original.evidence_watermark,
            correction.correction_value,
            correction.feedback_kind,
            correction.decision_id,
            correction.observed_at_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use super::super::model::{ModelClaimEvidenceV1, TaskRelationshipV1, MODEL_SEMANTIC_FIELDS};
    use super::super::observation_packet::{
        ActiveSurfaceIdentityV2, EvidencePartitionV2, KeyframeReferenceV2, PacketSizeAccountingV2,
    };
    use super::super::task_snapshot::{ReturnAnchorStatusV2, SnapshotHypothesisV2};

    fn packet(session: &str, at: i64) -> ObservationPacketV2 {
        let frame = KeyframeReferenceV2 {
            frame_id: format!("frame-{at}"),
            observed_at_ms: at,
            partition: EvidencePartitionV2::Current,
            surface_identity: ActiveSurfaceIdentityV2 {
                app_name: Some("Test".into()),
                app_bundle_id: Some("com.test".into()),
                window_title_hash: None,
                window_id: Some(1),
                browser_url_hash: None,
                document_path_hash: None,
            },
            surface_ownership_confidence: 1.0,
            privacy_status: "allowed".into(),
            model_eligible: true,
            image_source_kind: "native_active_window".into(),
            image_scope: "active_window".into(),
            image_width: Some(100),
            image_height: Some(100),
            image_rejection_reason: None,
            crop_pixels: None,
            local_image_handle_hash: Some(format!("image-{at}")),
            ephemeral_local_image_path: None,
            selection_reasons: vec!["manual_continue_boundary".into()],
        };
        ObservationPacketV2 {
            schema: "smalltalk.observation_packet.v2".into(),
            packet_id: format!("packet-{session}-{at}"),
            observed_at_ms: at,
            session_id: Some(session.into()),
            evidence_watermark: format!("watermark-{session}-{at}"),
            active_surface: frame.surface_identity.clone(),
            current_frame: frame.clone(),
            semantic_keyframes: vec![frame],
            surface_timeline: Vec::new(),
            canonical_elements: Vec::new(),
            focused_element_ids: Vec::new(),
            editable_element_ids: Vec::new(),
            selected_element_ids: Vec::new(),
            causal_events: Vec::new(),
            frame_changes: Vec::new(),
            capture_trigger_ids: Vec::new(),
            transition_ids: Vec::new(),
            return_anchor_facts: Vec::new(),
            previous_valid_snapshot_id: None,
            evidence_quality: "bounded_multisource".into(),
            missing_source_notes: Vec::new(),
            conflicting_observations: Vec::new(),
            partitions: BTreeMap::from([(
                EvidencePartitionV2::Current,
                vec![format!("frame-{at}")],
            )]),
            size: PacketSizeAccountingV2 {
                frame_count: 1,
                keyframe_count: 1,
                canonical_element_count: 0,
                causal_event_count: 0,
                serialized_bytes: 100,
                estimated_tokens: 25,
                truncated: false,
                frame_accounting: Vec::new(),
            },
        }
    }

    fn snapshot(id: &str, relation: &str, object: &str) -> TaskSnapshotV2 {
        TaskSnapshotV2 {
            schema: "smalltalk.task_snapshot.v2".into(),
            snapshot_id: id.into(),
            revision: 1,
            prior_snapshot_id: None,
            supersedes_snapshot_id: None,
            observed_at_ms: 1,
            evidence_watermark: "watermark".into(),
            packet_id: "packet".into(),
            session_id: None,
            task_thread_id: None,
            task_thread_revision: None,
            thread_status: "unresolved".into(),
            continuity_thread_id: None,
            continuity_thread_revision: None,
            continuity_identity_token: None,
            supersedes_thread_id: None,
            legacy_task_turn_id: None,
            task_basis: "explicit_goal".into(),
            observed_surface: Some("support surface".into()),
            immediate_user_operation: Some("reviewed evidence".into()),
            semantic_effect_of_operation: None,
            current_subtask: Some("supporting research".into()),
            task_summary: Some("Implement task memory".into()),
            task_kind: "model_inferred".into(),
            task_object: Some(object.into()),
            user_goal: None,
            app_identity: Some("Test".into()),
            surface_identity_hash: None,
            document_or_thread_identity_hash: None,
            execution_state: "active".into(),
            current_actor: "user".into(),
            waiting_on: "nothing".into(),
            last_meaningful_progress: Some("implemented schema".into()),
            unfinished_step: Some("wire selection".into()),
            next_action: None,
            relation_to_prior: relation.into(),
            selection_status: SnapshotSelectionStatusV2::Selected,
            claim_evidence: Vec::new(),
            alternative_hypotheses: Vec::<SnapshotHypothesisV2>::new(),
            contradictions: Vec::new(),
            confidence_by_field: BTreeMap::new(),
            confidence_by_source: BTreeMap::new(),
            return_anchor_candidate_id: None,
            return_anchor_status: ReturnAnchorStatusV2::Unresolved,
            resolver_version: "test".into(),
            provenance: Vec::new(),
            continuity_confidence_decay: 0.0,
            semantic_source: "cloud_multimodal_model".into(),
            provider_name: Some("test".into()),
            provider_model: Some("test".into()),
            provider_request_id: Some(format!("request-{id}")),
            provider_response_id: Some(format!("response-{id}")),
            selected_hypothesis_id: Some(format!("hypothesis-{id}")),
            wording_source: "deterministic".into(),
            inference_status: "verified".into(),
        }
    }

    fn persist_new(
        conn: &Connection,
        session: &str,
        id: &str,
        at: i64,
    ) -> ThreadBoundaryPersistResultV1 {
        persist_boundary_atomic(
            conn,
            &packet(session, at),
            snapshot(id, "new_task", "task-memory"),
        )
        .unwrap()
    }

    fn attach_continuity_identity(
        conn: &Connection,
        snapshot: &mut TaskSnapshotV2,
        result: &ThreadBoundaryPersistResultV1,
    ) -> TaskThreadHeadV1 {
        let thread_id = result.selected_thread_id.clone().unwrap();
        let head = load_context_heads(conn, result.snapshot.session_id.as_deref())
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == thread_id)
            .unwrap();
        snapshot.continuity_thread_id = Some(thread_id);
        snapshot.continuity_thread_revision = result.selected_thread_revision;
        snapshot.continuity_identity_token = Some(head.identity_token.clone());
        head
    }

    fn relationship_claim(prior: &TaskThreadHeadV1) -> ClaimEvidenceV2 {
        ClaimEvidenceV2 {
            claim: "relationship_to_prior".into(),
            evidence_refs: vec![
                EvidenceHandleV2 {
                    source_kind: "prior_thread_revision".into(),
                    record_id: format!("{}:{}", prior.task_thread_id, prior.revision),
                    frame_id: None,
                    content_hash: None,
                },
                EvidenceHandleV2 {
                    source_kind: "keyframe".into(),
                    record_id: "current-frame".into(),
                    frame_id: Some("current-frame".into()),
                    content_hash: None,
                },
            ],
            confidence: 0.9,
            source_confidence: BTreeMap::from([("cloud_multimodal_model".into(), 0.9)]),
        }
    }

    fn attach_continuity(
        conn: &Connection,
        snapshot: &mut TaskSnapshotV2,
        result: &ThreadBoundaryPersistResultV1,
    ) {
        let prior = attach_continuity_identity(conn, snapshot, result);
        snapshot.claim_evidence.push(relationship_claim(&prior));
    }

    #[test]
    fn supporting_research_preserves_the_primary_thread() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut support = snapshot("snapshot-2", "supporting_research", "task-memory");
        attach_continuity(&conn, &mut support, &first);
        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), support).unwrap();
        assert_eq!(result.selected_thread_id, first.selected_thread_id);
        assert_eq!(result.selected_thread_revision, Some(2));
        assert_eq!(result.snapshot.task_summary, first.snapshot.task_summary);
    }

    #[test]
    fn genuine_new_task_creates_a_new_thread_and_backgrounds_the_old_one() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let second = persist_new(&conn, "session-a", "snapshot-2", 2);
        assert_ne!(first.selected_thread_id, second.selected_thread_id);
        let old = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| Some(&head.task_thread_id) == first.selected_thread_id.as_ref())
            .unwrap();
        assert_eq!(old.status, TaskThreadStatusV1::Background);
    }

    #[test]
    fn temporary_detour_does_not_replace_the_thread_identity() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut detour = snapshot("snapshot-2", "temporary_detour", "task-memory");
        attach_continuity(&conn, &mut detour, &first);
        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), detour).unwrap();
        assert_eq!(result.selected_thread_id, first.selected_thread_id);
        assert_eq!(result.snapshot.thread_status, "active");
    }

    #[test]
    fn cross_session_reuse_without_a_verified_edge_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut continuation = snapshot("snapshot-2", "continuation", "task-memory");
        attach_continuity_identity(&conn, &mut continuation, &first);
        let result = persist_boundary_atomic(&conn, &packet("session-b", 2), continuation).unwrap();
        assert!(result.selected_thread_id.is_none());
        assert!(result
            .rejection_reason_codes
            .contains(&"session_mismatch".into()));
        assert!(result
            .rejection_reason_codes
            .contains(&"missing_continuity_edge".into()));
    }

    #[test]
    fn cross_session_continuity_with_exact_revision_and_current_evidence_reuses_thread() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut continuation = snapshot("snapshot-2", "continuation", "task-memory");
        attach_continuity(&conn, &mut continuation, &first);

        let result = persist_boundary_atomic(&conn, &packet("session-b", 2), continuation).unwrap();
        assert_eq!(result.selected_thread_id, first.selected_thread_id);
        assert_eq!(result.selected_thread_revision, Some(2));
        let head = load_context_heads(&conn, Some("session-b"))
            .unwrap()
            .into_iter()
            .find(|head| Some(&head.task_thread_id) == result.selected_thread_id.as_ref())
            .unwrap();
        assert_eq!(head.session_lineage, vec!["session-a", "session-b"]);
        assert_eq!(head.current_session_id, "session-b");
    }

    #[test]
    fn same_session_continuity_requires_exact_prior_revision_and_current_evidence() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut continuation = snapshot("snapshot-2", "continuation", "task-memory");
        let prior = attach_continuity_identity(&conn, &mut continuation, &first);
        continuation.claim_evidence.push(ClaimEvidenceV2 {
            claim: "relationship_to_prior".into(),
            evidence_refs: vec![EvidenceHandleV2 {
                source_kind: "prior_snapshot".into(),
                record_id: prior.head_snapshot_id,
                frame_id: None,
                content_hash: None,
            }],
            confidence: 0.9,
            source_confidence: BTreeMap::new(),
        });

        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), continuation).unwrap();
        assert!(result.selected_thread_id.is_none());
        assert!(result
            .rejection_reason_codes
            .contains(&"missing_continuity_edge".into()));
    }

    #[test]
    fn interruption_preserves_thread_identity_and_marks_it_interrupted() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut interruption = snapshot("snapshot-2", "interruption", "task-memory");
        interruption.current_subtask = Some("handle a brief interruption".into());
        attach_continuity(&conn, &mut interruption, &first);

        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), interruption).unwrap();
        assert_eq!(result.selected_thread_id, first.selected_thread_id);
        assert_eq!(result.snapshot.thread_status, "interrupted");
        assert_eq!(result.snapshot.task_summary, first.snapshot.task_summary);
        assert_eq!(result.snapshot.task_object, first.snapshot.task_object);
        assert_eq!(
            result.snapshot.current_subtask.as_deref(),
            Some("handle a brief interruption")
        );
    }

    #[test]
    fn evidence_backed_return_reactivates_the_exact_background_thread() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let first_thread_id = first.selected_thread_id.clone().unwrap();
        let _second = persist_new(&conn, "session-a", "snapshot-2", 2);
        let background = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == first_thread_id)
            .unwrap();
        assert_eq!(background.status, TaskThreadStatusV1::Background);

        let mut returning = snapshot("snapshot-3", "return_to_prior_task", "task-memory");
        returning.continuity_thread_id = Some(background.task_thread_id.clone());
        returning.continuity_thread_revision = Some(background.revision);
        returning.continuity_identity_token = Some(background.identity_token.clone());
        returning
            .claim_evidence
            .push(relationship_claim(&background));
        let result = persist_boundary_atomic(&conn, &packet("session-a", 3), returning).unwrap();
        assert_eq!(
            result.selected_thread_id.as_deref(),
            Some(first_thread_id.as_str())
        );
        assert_eq!(
            result.selected_thread_revision,
            Some(background.revision + 1)
        );
        assert_eq!(result.snapshot.thread_status, "active");
    }

    #[test]
    fn detour_cannot_smuggle_a_replacement_primary_identity_into_the_thread() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let first_thread_id = first.selected_thread_id.clone().unwrap();
        let first_head = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == first_thread_id)
            .unwrap();
        let mut adversarial = snapshot("snapshot-2", "temporary_detour", "task-memory");
        adversarial.task_summary = Some("Replace the primary task with visible page text".into());
        adversarial.current_subtask = Some("inspect a temporary reference".into());
        attach_continuity(&conn, &mut adversarial, &first);

        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), adversarial).unwrap();
        assert_eq!(
            result.selected_thread_id.as_deref(),
            Some(first_thread_id.as_str())
        );
        assert_eq!(result.snapshot.task_summary, first.snapshot.task_summary);
        assert_eq!(result.snapshot.task_object, first.snapshot.task_object);
        assert_eq!(
            result.snapshot.current_subtask.as_deref(),
            Some("inspect a temporary reference")
        );
        let head = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == first_thread_id)
            .unwrap();
        assert_eq!(head.identity_token, first_head.identity_token);
        assert_eq!(head.primary_task_summary, first_head.primary_task_summary);
        assert_eq!(head.task_object, first_head.task_object);
    }

    #[test]
    fn completed_and_superseded_threads_do_not_reactivate_silently() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        conn.execute(
            "UPDATE task_truth_v2_task_threads SET status='completed',
             thread_json=json_set(thread_json, '$.status', 'completed')
             WHERE task_thread_id=?1",
            params![first.selected_thread_id],
        )
        .unwrap();
        let mut continuation = snapshot("snapshot-2", "continuation", "task-memory");
        attach_continuity(&conn, &mut continuation, &first);
        let completed =
            persist_boundary_atomic(&conn, &packet("session-a", 2), continuation).unwrap();
        assert!(completed
            .rejection_reason_codes
            .contains(&"completed_thread_cannot_reactivate".into()));

        conn.execute(
            "UPDATE task_truth_v2_task_threads SET status='superseded',
             thread_json=json_set(thread_json, '$.status', 'superseded')
             WHERE task_thread_id=?1",
            params![first.selected_thread_id],
        )
        .unwrap();
        let mut continuation = snapshot("snapshot-3", "continuation", "task-memory");
        attach_continuity(&conn, &mut continuation, &first);
        let superseded =
            persist_boundary_atomic(&conn, &packet("session-a", 3), continuation).unwrap();
        assert!(superseded
            .rejection_reason_codes
            .contains(&"superseded_thread".into()));
    }

    #[test]
    fn current_unresolved_boundary_defeats_stale_supported_truth() {
        let conn = Connection::open_in_memory().unwrap();
        let _first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut unresolved = snapshot("snapshot-2", "unrelated_or_unknown", "unknown");
        unresolved.selection_status = SnapshotSelectionStatusV2::Unresolved;
        unresolved.task_summary = None;
        unresolved.semantic_source = "unresolved".into();
        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), unresolved).unwrap();
        assert!(result.selected_thread_id.is_none());
        assert!(result
            .rejection_reason_codes
            .contains(&"stale_unsupported_snapshot".into()));
    }

    #[test]
    fn completion_requires_current_execution_state_evidence() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let mut unsupported = snapshot("snapshot-2", "continuation", "task-memory");
        attach_continuity(&conn, &mut unsupported, &first);
        unsupported.execution_state = "completed".into();
        unsupported.claim_evidence.push(ClaimEvidenceV2 {
            claim: "last_meaningful_progress".into(),
            evidence_refs: vec![EvidenceHandleV2 {
                source_kind: "keyframe".into(),
                record_id: "progress-frame".into(),
                frame_id: Some("progress-frame".into()),
                content_hash: None,
            }],
            confidence: 0.9,
            source_confidence: BTreeMap::new(),
        });
        let rejected =
            persist_boundary_atomic(&conn, &packet("session-a", 2), unsupported).unwrap();
        assert!(rejected.selected_thread_id.is_none());
        assert!(rejected
            .rejection_reason_codes
            .contains(&"conflicting_current_evidence".into()));

        let mut supported = snapshot("snapshot-3", "continuation", "task-memory");
        attach_continuity(&conn, &mut supported, &first);
        supported.execution_state = "completed".into();
        supported.claim_evidence.push(ClaimEvidenceV2 {
            claim: "execution_state".into(),
            evidence_refs: vec![EvidenceHandleV2 {
                source_kind: "keyframe".into(),
                record_id: "completion-frame".into(),
                frame_id: Some("completion-frame".into()),
                content_hash: None,
            }],
            confidence: 0.9,
            source_confidence: BTreeMap::new(),
        });
        let completed = persist_boundary_atomic(&conn, &packet("session-a", 3), supported).unwrap();
        assert_eq!(completed.selected_thread_id, first.selected_thread_id);
        assert_eq!(completed.snapshot.thread_status, "completed");
    }

    #[test]
    fn supersession_records_evidence_linked_old_status_revision_and_edge() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let old_thread_id = first.selected_thread_id.clone().unwrap();
        let old_head = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == old_thread_id)
            .unwrap();
        let mut replacement = snapshot("snapshot-2", "new_task", "replacement-task");
        replacement.task_summary = Some("Start replacement task".into());
        replacement.supersedes_thread_id = Some(old_thread_id.clone());
        replacement
            .claim_evidence
            .push(relationship_claim(&old_head));

        let result = persist_boundary_atomic(&conn, &packet("session-a", 2), replacement).unwrap();
        let new_thread_id = result.selected_thread_id.clone().unwrap();
        assert_ne!(new_thread_id, old_thread_id);
        let old_after = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == old_thread_id)
            .unwrap();
        assert_eq!(old_after.status, TaskThreadStatusV1::Superseded);
        assert_eq!(old_after.revision, 2);
        assert_eq!(
            old_after.superseded_by_thread_id.as_deref(),
            Some(new_thread_id.as_str())
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM task_truth_v2_task_thread_revisions
                 WHERE task_thread_id=?1",
                params![old_thread_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            2
        );
        assert_eq!(
            conn.query_row(
                "SELECT snapshot_id FROM task_truth_v2_task_thread_revisions
                 WHERE task_thread_id=?1 AND revision=2",
                params![old_after.task_thread_id],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            first.snapshot.snapshot_id
        );
        let edge: (String, i64, String, i64, String) = conn
            .query_row(
                "SELECT from_thread_id, from_revision, to_thread_id, to_revision, evidence_json
                 FROM task_truth_v2_task_thread_edges
                 WHERE relationship='superseded_by_new_task'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(edge.0, old_after.task_thread_id);
        assert_eq!(edge.1, old_after.revision);
        assert_eq!(edge.2, new_thread_id);
        assert_eq!(edge.3, 1);
        assert!(edge.4.contains("prior_thread_revision"));
        assert!(edge.4.contains("keyframe"));
    }

    #[test]
    fn human_hypothesis_choice_persists_an_atomic_corrected_thread_revision() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_boundary_atomic(
            &conn,
            &packet("session-a", 1),
            snapshot("snapshot-1", "new_task", "task-memory"),
        )
        .unwrap();
        let thread_id = first.selected_thread_id.clone().unwrap();
        let thread_revision = first.selected_thread_revision.unwrap();
        let head = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == thread_id)
            .unwrap();
        let evidence = EvidenceHandleV2 {
            source_kind: "keyframe".into(),
            record_id: "frame-1".into(),
            frame_id: Some("frame-1".into()),
            content_hash: Some("image-1".into()),
        };
        let payload = ModelTaskHypothesisV1 {
            hypothesis_id: "hypothesis-alternative".into(),
            observed_surface: Some("corrected surface".into()),
            immediate_user_operation: Some("corrected operation".into()),
            semantic_effect_of_operation: Some("corrected effect".into()),
            current_subtask: Some("corrected subtask".into()),
            likely_primary_task: Some("Corrected primary task".into()),
            task_object: Some("task-memory".into()),
            app_identity: Some("Corrected App".into()),
            surface_identity_hash: None,
            document_or_thread_identity_hash: None,
            execution_state: Some("waiting".into()),
            current_actor: Some("application".into()),
            waiting_on: Some("application".into()),
            last_meaningful_progress: Some("corrected progress".into()),
            unfinished_state: Some("corrected unfinished state".into()),
            possible_next_action: Some("corrected next action".into()),
            relationship_to_prior: TaskRelationshipV1::Continuation,
            continuity_thread_id: Some(thread_id.clone()),
            continuity_thread_revision: Some(thread_revision),
            continuity_identity_token: Some(head.identity_token.clone()),
            supersedes_thread_id: None,
            return_anchor_record_id: None,
            claim_evidence: MODEL_SEMANTIC_FIELDS
                .iter()
                .map(|field| {
                    (
                        (*field).into(),
                        Some(ModelClaimEvidenceV1 {
                            claim: format!("corrected {field}"),
                            evidence_refs: vec![evidence.clone()],
                            confidence: 0.8,
                        }),
                    )
                })
                .collect(),
            contradictions: Vec::new(),
            confidence_by_field: MODEL_SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), 0.8))
                .collect(),
            confidence: 0.8,
        };
        let mut stored = first.snapshot.clone();
        stored.alternative_hypotheses = vec![SnapshotHypothesisV2 {
            hypothesis_id: "hypothesis-alternative".into(),
            summary: "Corrected primary task".into(),
            relation: "continuation".into(),
            confidence: 0.72,
            evidence_refs: Vec::new(),
            contradicting_evidence_refs: Vec::new(),
            task_thread_id: Some(thread_id.clone()),
            task_thread_revision: Some(thread_revision),
            last_supported_at_ms: Some(1),
            disposition: "retained_close_alternative".into(),
            reason_codes: vec!["close_model_hypothesis".into()],
            semantic_payload: Some(serde_json::to_value(payload).unwrap()),
        }];
        conn.execute(
            "UPDATE task_truth_v2_snapshots SET snapshot_json=?1 WHERE snapshot_id=?2",
            params![serde_json::to_string(&stored).unwrap(), stored.snapshot_id],
        )
        .unwrap();

        let correction = HumanCorrectionPersistenceV1 {
            feedback_id: "feedback-human-choice".into(),
            decision_id: Some("decision-1".into()),
            affected_field: "hypothesis".into(),
            hypothesis_id: Some("hypothesis-alternative".into()),
            feedback_kind: "corrected".into(),
            correction_value: None,
            observed_at_ms: 2,
        };
        let corrected = persist_human_correction_atomic(&conn, &stored, &correction).unwrap();

        assert_eq!(
            corrected.selected_thread_id.as_deref(),
            Some(thread_id.as_str())
        );
        assert_eq!(corrected.selected_thread_revision, Some(2));
        assert_eq!(
            corrected.snapshot.task_summary.as_deref(),
            Some("Corrected primary task")
        );
        assert_eq!(corrected.snapshot.semantic_source, "human_correction");
        assert_eq!(
            corrected.snapshot.last_meaningful_progress.as_deref(),
            Some("corrected progress")
        );
        assert_eq!(
            corrected.snapshot.unfinished_step.as_deref(),
            Some("corrected unfinished state")
        );
        assert_eq!(corrected.snapshot.execution_state, "waiting");
        assert_eq!(
            corrected.snapshot.selected_hypothesis_id.as_deref(),
            Some("hypothesis-alternative")
        );
        let feedback_identity: (String, i64, String) = conn
            .query_row(
                "SELECT task_thread_id, task_thread_revision, corrected_snapshot_id
                 FROM task_truth_v2_feedback_events WHERE feedback_id=?1",
                params![correction.feedback_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(feedback_identity.0, thread_id);
        assert_eq!(feedback_identity.1, 2);
        assert_eq!(feedback_identity.2, corrected.snapshot.snapshot_id);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM task_truth_v2_task_thread_revisions
                 WHERE task_thread_id=?1",
                params![feedback_identity.0],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn human_completion_updates_the_existing_thread_instead_of_creating_a_new_one() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let correction = HumanCorrectionPersistenceV1 {
            feedback_id: "feedback-completed".into(),
            decision_id: Some("decision-1".into()),
            affected_field: "task_status".into(),
            hypothesis_id: None,
            feedback_kind: "completed".into(),
            correction_value: Some("completed".into()),
            observed_at_ms: 2,
        };

        let corrected =
            persist_human_correction_atomic(&conn, &first.snapshot, &correction).unwrap();
        assert_eq!(corrected.selected_thread_id, first.selected_thread_id);
        assert_eq!(corrected.selected_thread_revision, Some(2));
        assert_eq!(corrected.snapshot.thread_status, "completed");
        assert_eq!(corrected.snapshot.execution_state, "completed");
        assert!(corrected.snapshot.unfinished_step.is_none());
    }

    #[test]
    fn human_rejection_demotes_only_the_exact_selected_thread_revision_to_unresolved() {
        let conn = Connection::open_in_memory().unwrap();
        let first = persist_new(&conn, "session-a", "snapshot-1", 1);
        let thread_id = first.selected_thread_id.clone().unwrap();
        let selected_hypothesis_id = first.snapshot.selected_hypothesis_id.clone().unwrap();
        let original_watermark = first.snapshot.evidence_watermark.clone();
        let correction = HumanCorrectionPersistenceV1 {
            feedback_id: "feedback-reject-selected".into(),
            decision_id: Some("decision-reject-selected".into()),
            affected_field: "task_summary".into(),
            hypothesis_id: Some(selected_hypothesis_id.clone()),
            feedback_kind: "rejected".into(),
            correction_value: Some("unresolved".into()),
            observed_at_ms: 2,
        };

        let corrected =
            persist_human_correction_atomic(&conn, &first.snapshot, &correction).unwrap();
        assert_eq!(
            corrected.selected_thread_id.as_deref(),
            Some(thread_id.as_str())
        );
        assert_eq!(corrected.selected_thread_revision, Some(2));
        assert_eq!(
            corrected.snapshot.selection_status,
            SnapshotSelectionStatusV2::Unresolved
        );
        assert_eq!(corrected.snapshot.thread_status, "unresolved");
        assert!(corrected.snapshot.task_summary.is_none());
        assert!(corrected.snapshot.task_object.is_none());
        assert!(corrected.snapshot.last_meaningful_progress.is_none());
        assert!(corrected.snapshot.unfinished_step.is_none());
        assert!(corrected.snapshot.next_action.is_none());
        assert_eq!(
            corrected.snapshot.selected_hypothesis_id.as_deref(),
            Some(selected_hypothesis_id.as_str())
        );

        let head = load_context_heads(&conn, Some("session-a"))
            .unwrap()
            .into_iter()
            .find(|head| head.task_thread_id == thread_id)
            .unwrap();
        assert_eq!(head.revision, 2);
        assert_eq!(head.status, TaskThreadStatusV1::Unresolved);
        assert!(head.primary_task_summary.is_none());
        assert!(head.task_object.is_none());

        let feedback: (String, i64, String, i64, String, String, String) = conn
            .query_row(
                "SELECT task_thread_id, task_thread_revision,
                        task_snapshot_id, task_snapshot_revision,
                        hypothesis_id, evidence_watermark, feedback_kind
                 FROM task_truth_v2_feedback_events WHERE feedback_id=?1",
                params![correction.feedback_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(feedback.0, thread_id);
        assert_eq!(feedback.1, 2);
        assert_eq!(feedback.2, first.snapshot.snapshot_id);
        assert_eq!(feedback.3, first.snapshot.revision);
        assert_eq!(feedback.4, selected_hypothesis_id);
        assert_eq!(feedback.5, original_watermark);
        assert_eq!(feedback.6, "rejected");

        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM task_truth_v2_feedback_events
                 WHERE feedback_id<>?1",
                params![correction.feedback_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
    }
}
