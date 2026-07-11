use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::super::stable_hash;
use super::checkpoint::ensure_schema;
use super::observation_packet::ObservationPacketV2;
use super::selection::SnapshotSelectionResultV2;
use super::task_snapshot::TaskSnapshotV2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ShadowAuditSummaryV2 {
    pub(crate) audit_id: String,
    pub(crate) decision_id: String,
    pub(crate) packet_id: String,
    pub(crate) selected_snapshot_id: Option<String>,
    pub(crate) first_divergence: Option<String>,
    pub(crate) latency_ms: i64,
    pub(crate) serialized_bytes: usize,
    pub(crate) estimated_tokens: usize,
    pub(crate) p50_latency_ms: i64,
    pub(crate) p95_latency_ms: i64,
    pub(crate) resolution_status: Option<String>,
    pub(crate) estimated_request_cost_usd: Option<f64>,
}

fn percentile(sorted: &[i64], percentile: f64) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (((sorted.len() - 1) as f64) * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn recent_latency_percentiles(conn: &Connection) -> Result<(i64, i64), String> {
    let mut statement = conn
        .prepare(
            "SELECT latency_ms FROM task_truth_v2_shadow_audits
             ORDER BY observed_at_ms DESC LIMIT 200",
        )
        .map_err(|error| error.to_string())?;
    let mut values = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    values.sort_unstable();
    Ok((percentile(&values, 0.50), percentile(&values, 0.95)))
}

pub(crate) fn persist_shadow_audit(
    conn: &Connection,
    decision_id: &str,
    packet: &ObservationPacketV2,
    snapshots: &[TaskSnapshotV2],
    selection: &SnapshotSelectionResultV2,
    legacy_task_turn_id: Option<&str>,
    legacy_selected_candidate_id: Option<&str>,
    build_latency_ms: i64,
    multimodal: Option<&super::MultimodalShadowAuditV1>,
) -> Result<ShadowAuditSummaryV2, String> {
    ensure_schema(conn)?;
    let selected = selection.selected_snapshot_id.as_deref();
    let first_divergence = match (selected, legacy_task_turn_id) {
        (None, Some(_)) => Some("task_selection_shadow_unresolved".to_string()),
        (Some(selected), Some(legacy))
            if snapshots
                .iter()
                .find(|snapshot| snapshot.snapshot_id == selected)
                .and_then(|snapshot| snapshot.legacy_task_turn_id.as_deref())
                != Some(legacy) =>
        {
            Some("task_selection".into())
        }
        _ if legacy_selected_candidate_id.is_some() && selected.is_none() => {
            Some("legacy_candidate_without_selected_snapshot".into())
        }
        _ => None,
    };
    let created_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(packet.observed_at_ms);
    let audit_id = format!(
        "tt2-audit-{}",
        stable_hash(format!("{decision_id}:{}:{created_at_ms}", packet.packet_id).as_bytes())
    );
    let packet_summary = serde_json::json!({
        "schema": packet.schema,
        "packet_id": packet.packet_id,
        "observed_at_ms": packet.observed_at_ms,
        "session_id": packet.session_id,
        "evidence_watermark": packet.evidence_watermark,
        "current_frame_id": packet.current_frame.frame_id,
        "privacy_status": packet.current_frame.privacy_status,
        "model_eligible": packet.current_frame.model_eligible,
        "evidence_quality": packet.evidence_quality,
        "missing_source_notes": packet.missing_source_notes,
        "size": packet.size,
        "multimodal": multimodal,
    });
    let keyframe_reasons = packet
        .semantic_keyframes
        .iter()
        .map(|keyframe| {
            serde_json::json!({
                "frame_id": keyframe.frame_id,
                "partition": keyframe.partition,
                "reasons": keyframe.selection_reasons,
                "model_eligible": keyframe.model_eligible,
            })
        })
        .collect::<Vec<_>>();
    let conflicts = packet
        .canonical_elements
        .iter()
        .filter(|element| !element.source_conflicts.is_empty())
        .map(|element| {
            serde_json::json!({
                "element_id": element.element_id,
                "sources": element.source_votes,
                "conflicts": element.source_conflicts,
                "task_eligible": element.task_eligible,
            })
        })
        .collect::<Vec<_>>();
    let causal_edges = packet
        .causal_events
        .iter()
        .map(|event| {
            serde_json::json!({
                "event_id": event.event_id,
                "event_kind": event.event_kind,
                "frame_id": event.frame_id,
                "parents": event.causal_parent_ids,
                "committed": event.committed,
            })
        })
        .collect::<Vec<_>>();
    let hypotheses = snapshots
        .iter()
        .map(|snapshot| {
            serde_json::json!({
                "snapshot_id": snapshot.snapshot_id,
                "revision": snapshot.revision,
                "selection_status": snapshot.selection_status,
                "task_summary_hash": snapshot.task_summary.as_deref().map(|value| stable_hash(value.as_bytes())),
                "execution_state": snapshot.execution_state,
                "current_actor": snapshot.current_actor,
                "waiting_on": snapshot.waiting_on,
                "field_confidence": snapshot.confidence_by_field,
                "contradictions": snapshot.contradictions,
            })
        })
        .collect::<Vec<_>>();
    let legacy_comparison = serde_json::json!({
        "legacy_task_turn_id": legacy_task_turn_id,
        "legacy_selected_candidate_id": legacy_selected_candidate_id,
        "selected_snapshot_id": selected,
        "first_divergence": first_divergence,
        "target_features_used_for_snapshot_selection": false,
    });
    conn.execute(
        "INSERT INTO task_truth_v2_shadow_audits (
           audit_id, decision_id, observed_at_ms, packet_id, selected_snapshot_id,
           legacy_task_turn_id, first_divergence, packet_summary_json,
           keyframe_reasons_json, canonical_conflicts_json, causal_edges_json,
           snapshot_hypotheses_json, selection_json, legacy_comparison_json,
           latency_ms, serialized_bytes, estimated_tokens, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            audit_id,
            decision_id,
            packet.observed_at_ms,
            packet.packet_id,
            selected,
            legacy_task_turn_id,
            first_divergence,
            packet_summary.to_string(),
            serde_json::to_string(&keyframe_reasons).map_err(|error| error.to_string())?,
            serde_json::to_string(&conflicts).map_err(|error| error.to_string())?,
            serde_json::to_string(&causal_edges).map_err(|error| error.to_string())?,
            serde_json::to_string(&hypotheses).map_err(|error| error.to_string())?,
            serde_json::to_string(selection).map_err(|error| error.to_string())?,
            legacy_comparison.to_string(),
            build_latency_ms,
            packet.size.serialized_bytes as i64,
            packet.size.estimated_tokens as i64,
            created_at_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    let (p50_latency_ms, p95_latency_ms) = recent_latency_percentiles(conn)?;
    Ok(ShadowAuditSummaryV2 {
        audit_id,
        decision_id: decision_id.into(),
        packet_id: packet.packet_id.clone(),
        selected_snapshot_id: selected.map(str::to_string),
        first_divergence,
        latency_ms: build_latency_ms,
        serialized_bytes: packet.size.serialized_bytes,
        estimated_tokens: packet.size.estimated_tokens,
        p50_latency_ms,
        p95_latency_ms,
        resolution_status: multimodal.and_then(|audit| {
            serde_json::to_value(audit.verification.status)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
        }),
        estimated_request_cost_usd: multimodal.and_then(|audit| audit.estimated_request_cost_usd),
    })
}
