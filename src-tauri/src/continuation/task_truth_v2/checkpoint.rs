use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::super::stable_hash;
use super::observation_packet::ObservationPacketV2;
use super::selection::{select_snapshot, SnapshotSelectionResultV2};
use super::task_snapshot::{SnapshotSelectionStatusV2, TaskSnapshotV2};

const RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;
const MAX_RETAINED_CHECKPOINTS: i64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SemanticCheckpointV2 {
    pub(crate) checkpoint_id: String,
    pub(crate) boundary_kind: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) packet_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) prior_checkpoint_id: Option<String>,
    pub(crate) supersedes_checkpoint_id: Option<String>,
    pub(crate) semantic_fingerprint: String,
    pub(crate) unresolved: bool,
    pub(crate) continuity_relation: String,
    pub(crate) confidence_decay: f64,
    pub(crate) write_status: String,
}

pub(crate) fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_truth_v2_observation_packets (
           packet_id TEXT PRIMARY KEY,
           schema_version TEXT NOT NULL,
           observed_at_ms INTEGER NOT NULL,
           session_id TEXT,
           evidence_watermark TEXT NOT NULL,
           current_frame_id TEXT NOT NULL,
           privacy_status TEXT NOT NULL,
           model_eligible INTEGER NOT NULL DEFAULT 0,
           serialized_bytes INTEGER NOT NULL,
           estimated_tokens INTEGER NOT NULL,
           packet_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_packets_session_time
           ON task_truth_v2_observation_packets(session_id, observed_at_ms DESC);

         CREATE TABLE IF NOT EXISTS task_truth_v2_snapshots (
           snapshot_id TEXT PRIMARY KEY,
           schema_version TEXT NOT NULL,
           revision INTEGER NOT NULL,
           observed_at_ms INTEGER NOT NULL,
           evidence_watermark TEXT NOT NULL,
           packet_id TEXT NOT NULL,
           legacy_task_turn_id TEXT,
           execution_state TEXT NOT NULL,
           relation_to_prior TEXT NOT NULL,
           selection_status TEXT NOT NULL,
           semantic_fingerprint TEXT NOT NULL,
           snapshot_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           FOREIGN KEY(packet_id) REFERENCES task_truth_v2_observation_packets(packet_id)
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_snapshots_time
           ON task_truth_v2_snapshots(observed_at_ms DESC, revision DESC);
         CREATE INDEX IF NOT EXISTS idx_tt2_snapshots_turn
           ON task_truth_v2_snapshots(legacy_task_turn_id, revision DESC);

         CREATE TABLE IF NOT EXISTS task_truth_v2_checkpoints (
           checkpoint_id TEXT PRIMARY KEY,
           boundary_kind TEXT NOT NULL,
           observed_at_ms INTEGER NOT NULL,
           session_id TEXT,
           packet_id TEXT NOT NULL,
           snapshot_id TEXT NOT NULL,
           prior_checkpoint_id TEXT,
           supersedes_checkpoint_id TEXT,
           semantic_fingerprint TEXT NOT NULL,
           unresolved INTEGER NOT NULL DEFAULT 0,
           continuity_relation TEXT NOT NULL,
           confidence_decay REAL NOT NULL DEFAULT 0.0,
           checkpoint_json TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           FOREIGN KEY(packet_id) REFERENCES task_truth_v2_observation_packets(packet_id),
           FOREIGN KEY(snapshot_id) REFERENCES task_truth_v2_snapshots(snapshot_id)
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_tt2_checkpoint_semantic_dedupe
           ON task_truth_v2_checkpoints(session_id, semantic_fingerprint);
         CREATE INDEX IF NOT EXISTS idx_tt2_checkpoints_session_time
           ON task_truth_v2_checkpoints(session_id, observed_at_ms DESC);

         CREATE TABLE IF NOT EXISTS task_truth_v2_shadow_audits (
           audit_id TEXT PRIMARY KEY,
           decision_id TEXT NOT NULL,
           observed_at_ms INTEGER NOT NULL,
           packet_id TEXT,
           selected_snapshot_id TEXT,
           legacy_task_turn_id TEXT,
           first_divergence TEXT,
           packet_summary_json TEXT NOT NULL,
           keyframe_reasons_json TEXT NOT NULL,
           canonical_conflicts_json TEXT NOT NULL,
           causal_edges_json TEXT NOT NULL,
           snapshot_hypotheses_json TEXT NOT NULL,
           selection_json TEXT NOT NULL,
           legacy_comparison_json TEXT NOT NULL,
           latency_ms INTEGER NOT NULL,
           serialized_bytes INTEGER NOT NULL,
           estimated_tokens INTEGER NOT NULL,
           created_at_ms INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_audits_decision
           ON task_truth_v2_shadow_audits(decision_id, observed_at_ms DESC);

         CREATE TABLE IF NOT EXISTS task_truth_v2_authority_audits (
           audit_id TEXT PRIMARY KEY,
           observed_at_ms INTEGER NOT NULL,
           requested_state TEXT NOT NULL,
           effective_state TEXT NOT NULL,
           release_gate_passed INTEGER NOT NULL,
           policy_version TEXT NOT NULL,
           cache_fingerprint TEXT NOT NULL,
           reason_codes_json TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_authority_audits_time
           ON task_truth_v2_authority_audits(observed_at_ms DESC);

         CREATE TABLE IF NOT EXISTS task_truth_v2_feedback_events (
           feedback_id TEXT PRIMARY KEY,
           task_thread_id TEXT,
           task_thread_revision INTEGER,
           task_snapshot_id TEXT NOT NULL,
           task_snapshot_revision INTEGER NOT NULL,
           corrected_snapshot_id TEXT,
           corrected_snapshot_revision INTEGER,
           affected_field TEXT NOT NULL,
           hypothesis_id TEXT,
           evidence_watermark TEXT,
           correction_value TEXT,
           feedback_kind TEXT NOT NULL,
           decision_id TEXT,
           observed_at_ms INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_feedback_snapshot
           ON task_truth_v2_feedback_events(task_snapshot_id, task_snapshot_revision);

         CREATE TABLE IF NOT EXISTS task_truth_v2_decision_contracts (
           decision_id TEXT PRIMARY KEY,
           effective_state TEXT NOT NULL,
           release_gate_passed INTEGER NOT NULL,
           snapshot_id TEXT,
           snapshot_revision INTEGER,
           task_thread_id TEXT,
           task_thread_revision INTEGER,
           selected_hypothesis_id TEXT,
           model_request_id TEXT,
           model_response_id TEXT,
           provider_attempt_count INTEGER NOT NULL DEFAULT 0,
           observation_packet_id TEXT,
           evidence_watermark TEXT,
           correction_fingerprint TEXT,
           current_frame_id TEXT,
           packet_policy_version TEXT,
           response_schema_version TEXT,
           admission_version TEXT,
           admitted_result_id TEXT,
           correction_watermark TEXT,
           target_status TEXT,
           target_identity TEXT,
           answer_contract_json TEXT,
           return_target_artifact_id TEXT,
           created_at_ms INTEGER NOT NULL
         );",
    )
    .map_err(|error| error.to_string())?;
    for (table, column, sql_type) in [
        ("task_truth_v2_feedback_events", "task_thread_id", "TEXT"),
        (
            "task_truth_v2_feedback_events",
            "task_thread_revision",
            "INTEGER",
        ),
        (
            "task_truth_v2_feedback_events",
            "corrected_snapshot_id",
            "TEXT",
        ),
        (
            "task_truth_v2_feedback_events",
            "corrected_snapshot_revision",
            "INTEGER",
        ),
        (
            "task_truth_v2_feedback_events",
            "evidence_watermark",
            "TEXT",
        ),
        ("task_truth_v2_feedback_events", "correction_value", "TEXT"),
        ("task_truth_v2_decision_contracts", "task_thread_id", "TEXT"),
        (
            "task_truth_v2_decision_contracts",
            "task_thread_revision",
            "INTEGER",
        ),
        (
            "task_truth_v2_decision_contracts",
            "selected_hypothesis_id",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "model_request_id",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "model_response_id",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "provider_attempt_count",
            "INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "task_truth_v2_decision_contracts",
            "observation_packet_id",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "evidence_watermark",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "correction_fingerprint",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "current_frame_id",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "packet_policy_version",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "response_schema_version",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "admission_version",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "admitted_result_id",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "correction_watermark",
            "TEXT",
        ),
        ("task_truth_v2_decision_contracts", "target_status", "TEXT"),
        (
            "task_truth_v2_decision_contracts",
            "target_identity",
            "TEXT",
        ),
        (
            "task_truth_v2_decision_contracts",
            "answer_contract_json",
            "TEXT",
        ),
    ] {
        let exists = {
            let mut statement = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .map_err(|error| error.to_string())?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            columns.iter().any(|existing| existing == column)
        };
        if !exists {
            conn.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN {column} {sql_type}"
            ))
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn snapshot_fingerprint(snapshot: &TaskSnapshotV2) -> String {
    let material = serde_json::json!({
        "task_summary": snapshot.task_summary,
        "task_kind": snapshot.task_kind,
        "task_object": snapshot.task_object,
        "user_goal": snapshot.user_goal,
        "surface": snapshot.surface_identity_hash,
        "execution_state": snapshot.execution_state,
        "current_actor": snapshot.current_actor,
        "waiting_on": snapshot.waiting_on,
        "last_progress": snapshot.last_meaningful_progress,
        "unfinished_step": snapshot.unfinished_step,
        "relation": snapshot.relation_to_prior,
        "contradictions": snapshot.contradictions,
    });
    stable_hash(material.to_string().as_bytes())
}

pub(crate) fn load_latest_snapshot(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Option<TaskSnapshotV2>, String> {
    ensure_schema(conn)?;
    let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let raw = conn
        .query_row(
            "SELECT s.snapshot_json
             FROM task_truth_v2_snapshots s
             JOIN task_truth_v2_observation_packets p ON p.packet_id=s.packet_id
             WHERE p.session_id=?1
             ORDER BY s.observed_at_ms DESC, s.revision DESC LIMIT 1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(|error| error.to_string()))
        .transpose()
}

fn boundary_kind(packet: &ObservationPacketV2) -> String {
    let reasons = packet
        .semantic_keyframes
        .iter()
        .flat_map(|keyframe| keyframe.selection_reasons.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    for (reason, kind) in [
        ("manual_continue_boundary", "manual_continue"),
        ("visible_error_boundary", "visible_error_or_blocker"),
        ("surface_switch_boundary", "surface_switch"),
        ("command_boundary", "command_execution"),
        ("submit_boundary", "submit"),
        ("committed_typing_boundary", "submit"),
        ("idle_after_progress_boundary", "idle_after_progress"),
        ("material_change_boundary", "material_change"),
        ("event_transition_boundary", "application_output_completion"),
    ] {
        if reasons.contains(&reason) {
            return kind.into();
        }
    }
    "observation_update".into()
}

pub(crate) fn persist_checkpoint(
    conn: &Connection,
    packet: &ObservationPacketV2,
    snapshot: &TaskSnapshotV2,
) -> Result<SemanticCheckpointV2, String> {
    ensure_schema(conn)?;
    let packet_json = serde_json::to_string(packet).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO task_truth_v2_observation_packets (
           packet_id, schema_version, observed_at_ms, session_id, evidence_watermark,
           current_frame_id, privacy_status, model_eligible, serialized_bytes,
           estimated_tokens, packet_json, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?3)",
        params![
            packet.packet_id,
            packet.schema,
            packet.observed_at_ms,
            packet.session_id,
            packet.evidence_watermark,
            packet.current_frame.frame_id,
            packet.current_frame.privacy_status,
            i64::from(packet.current_frame.model_eligible),
            packet.size.serialized_bytes as i64,
            packet.size.estimated_tokens as i64,
            packet_json,
        ],
    )
    .map_err(|error| error.to_string())?;

    let semantic_fingerprint = snapshot_fingerprint(snapshot);
    let snapshot_json = serde_json::to_string(snapshot).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO task_truth_v2_snapshots (
           snapshot_id, schema_version, revision, observed_at_ms, evidence_watermark,
           packet_id, legacy_task_turn_id, execution_state, relation_to_prior,
           selection_status, semantic_fingerprint, snapshot_json, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?4)",
        params![
            snapshot.snapshot_id,
            snapshot.schema,
            snapshot.revision,
            snapshot.observed_at_ms,
            snapshot.evidence_watermark,
            snapshot.packet_id,
            snapshot.legacy_task_turn_id,
            snapshot.execution_state,
            snapshot.relation_to_prior,
            format!("{:?}", snapshot.selection_status).to_ascii_lowercase(),
            semantic_fingerprint,
            snapshot_json,
        ],
    )
    .map_err(|error| error.to_string())?;

    let prior = conn
        .query_row(
            "SELECT checkpoint_id FROM task_truth_v2_checkpoints
             WHERE (?1 IS NULL OR session_id=?1)
             ORDER BY observed_at_ms DESC, checkpoint_id DESC LIMIT 1",
            params![packet.session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let boundary_kind = boundary_kind(packet);
    let checkpoint_seed = format!(
        "{}:{}:{}",
        packet.session_id.as_deref().unwrap_or("no_session"),
        semantic_fingerprint,
        boundary_kind
    );
    let checkpoint_id = format!("checkpoint-{}", stable_hash(checkpoint_seed.as_bytes()));
    let mut checkpoint = SemanticCheckpointV2 {
        checkpoint_id: checkpoint_id.clone(),
        boundary_kind,
        observed_at_ms: packet.observed_at_ms,
        packet_id: packet.packet_id.clone(),
        snapshot_id: snapshot.snapshot_id.clone(),
        prior_checkpoint_id: prior.clone(),
        supersedes_checkpoint_id: prior,
        semantic_fingerprint: semantic_fingerprint.clone(),
        unresolved: snapshot.selection_status == SnapshotSelectionStatusV2::Unresolved,
        continuity_relation: snapshot.relation_to_prior.clone(),
        confidence_decay: snapshot.continuity_confidence_decay,
        write_status: "persisted".into(),
    };
    let checkpoint_json = serde_json::to_string(&checkpoint).map_err(|error| error.to_string())?;
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO task_truth_v2_checkpoints (
               checkpoint_id, boundary_kind, observed_at_ms, session_id, packet_id,
               snapshot_id, prior_checkpoint_id, supersedes_checkpoint_id,
               semantic_fingerprint, unresolved, continuity_relation, confidence_decay,
               checkpoint_json, created_at_ms
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?3)",
            params![
                checkpoint.checkpoint_id,
                checkpoint.boundary_kind,
                checkpoint.observed_at_ms,
                packet.session_id,
                checkpoint.packet_id,
                checkpoint.snapshot_id,
                checkpoint.prior_checkpoint_id,
                checkpoint.supersedes_checkpoint_id,
                checkpoint.semantic_fingerprint,
                i64::from(checkpoint.unresolved),
                checkpoint.continuity_relation,
                checkpoint.confidence_decay,
                checkpoint_json,
            ],
        )
        .map_err(|error| error.to_string())?;
    if inserted == 0 {
        checkpoint.write_status = "deduplicated_semantically_unchanged".into();
    }
    prune(conn, packet.observed_at_ms)?;
    Ok(checkpoint)
}

pub(crate) fn load_recent_snapshots(
    conn: &Connection,
    session_id: Option<&str>,
    limit: usize,
) -> Result<Vec<TaskSnapshotV2>, String> {
    ensure_schema(conn)?;
    let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn
        .prepare(
            "SELECT s.snapshot_json
             FROM task_truth_v2_snapshots s
             JOIN task_truth_v2_observation_packets p ON p.packet_id=s.packet_id
             WHERE p.session_id=?1
             ORDER BY s.observed_at_ms DESC, s.revision DESC LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let raw_rows = stmt
        .query_map(params![session_id, limit.clamp(1, 100) as i64], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    raw_rows
        .into_iter()
        .map(|raw| serde_json::from_str(&raw).map_err(|error| error.to_string()))
        .collect()
}

pub(crate) fn select_recent(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<SnapshotSelectionResultV2, String> {
    Ok(select_snapshot(&load_recent_snapshots(
        conn, session_id, 24,
    )?))
}

fn prune(conn: &Connection, now_ms: i64) -> Result<(), String> {
    let cutoff = now_ms.saturating_sub(RETENTION_MS);
    conn.execute(
        "DELETE FROM task_truth_v2_checkpoints
         WHERE observed_at_ms < ?1
           AND checkpoint_id NOT IN (
             SELECT checkpoint_id FROM task_truth_v2_checkpoints
             ORDER BY observed_at_ms DESC LIMIT ?2
           )",
        params![cutoff, MAX_RETAINED_CHECKPOINTS],
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "DELETE FROM task_truth_v2_snapshots
         WHERE observed_at_ms < ?1
           AND snapshot_id NOT IN (
             SELECT snapshot_id FROM task_truth_v2_checkpoints
           )
           AND snapshot_id NOT IN (
             SELECT snapshot_id FROM task_truth_v2_snapshots
             ORDER BY observed_at_ms DESC, revision DESC LIMIT ?2
           )",
        params![cutoff, MAX_RETAINED_CHECKPOINTS],
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "DELETE FROM task_truth_v2_observation_packets
         WHERE observed_at_ms < ?1
           AND packet_id NOT IN (SELECT packet_id FROM task_truth_v2_checkpoints)
           AND packet_id NOT IN (SELECT packet_id FROM task_truth_v2_snapshots)",
        params![cutoff],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unscoped_snapshot_reads_never_fall_back_to_global_rows() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO task_truth_v2_observation_packets (
               packet_id, schema_version, observed_at_ms, session_id, evidence_watermark,
               current_frame_id, privacy_status, model_eligible, serialized_bytes,
               estimated_tokens, packet_json, created_at_ms
             ) VALUES ('packet-a','packet.v2',1,'session-a','watermark-a',
                       'frame-a','allowed',1,1,1,'{}',1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO task_truth_v2_snapshots (
               snapshot_id, schema_version, revision, observed_at_ms, evidence_watermark,
               packet_id, execution_state, relation_to_prior, selection_status,
               semantic_fingerprint, snapshot_json, created_at_ms
             ) VALUES ('snapshot-a','snapshot.v2',1,1,'watermark-a','packet-a',
                       'active','new_task','selected','fingerprint-a','not-json',1)",
            [],
        )
        .unwrap();

        assert!(load_latest_snapshot(&conn, None).unwrap().is_none());
        assert!(load_latest_snapshot(&conn, Some(" ")).unwrap().is_none());
        assert!(load_recent_snapshots(&conn, None, 24).unwrap().is_empty());
        assert!(load_recent_snapshots(&conn, Some(""), 24)
            .unwrap()
            .is_empty());
        assert!(load_latest_snapshot(&conn, Some("session-a")).is_err());
    }
}
