use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::super::stable_hash;
use super::checkpoint::ensure_schema;
use super::observation_packet::ObservationPacketV2;
use super::selection::SnapshotSelectionResultV2;
use super::task_snapshot::TaskSnapshotV2;

const PERFORMANCE_SAMPLE_SCHEMA: &str = "smalltalk.mfti_04.performance_sample.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PerformanceReviewSummaryV1 {
    pub(crate) schema: String,
    pub(crate) sample_id: String,
    pub(crate) sample_complete: bool,
    pub(crate) provider_outcome: String,
    pub(crate) capture_to_packet_ms: i64,
    pub(crate) request_build_ms: i64,
    pub(crate) provider_ms: i64,
    pub(crate) verification_persistence_ms: i64,
    pub(crate) total_manual_continue_ms: i64,
    pub(crate) estimated_cost_usd: f64,
    pub(crate) second_pass_ran: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PerformanceStageTimingsV1 {
    pub(crate) capture_to_packet_ms: i64,
    pub(crate) verification_persistence_ms: i64,
    pub(crate) total_manual_continue_ms: i64,
}

fn ensure_performance_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_truth_v2_performance_samples (
           sample_id TEXT PRIMARY KEY,
           schema_version TEXT NOT NULL,
           decision_id_hash TEXT NOT NULL,
           observed_at_ms INTEGER NOT NULL,
           sample_complete INTEGER NOT NULL,
           capture_to_packet_ms INTEGER NOT NULL,
           request_build_ms INTEGER NOT NULL,
           provider_ms INTEGER NOT NULL,
           verification_persistence_ms INTEGER NOT NULL,
           total_manual_continue_ms INTEGER NOT NULL,
           image_count INTEGER NOT NULL,
           image_bytes INTEGER NOT NULL,
           structured_bytes INTEGER NOT NULL,
           input_tokens INTEGER NOT NULL,
           output_tokens INTEGER NOT NULL,
           estimated_cost_usd REAL NOT NULL,
           provider_outcome TEXT NOT NULL,
           provider_attempt_count INTEGER NOT NULL,
           second_pass_ran INTEGER NOT NULL,
           second_pass_cost_usd REAL NOT NULL,
           privacy_excluded_frame_count INTEGER NOT NULL,
           transported_frame_count INTEGER NOT NULL,
           privacy_blocked_before_transport INTEGER NOT NULL,
           background_multimodal_requests INTEGER NOT NULL,
           created_at_ms INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_tt2_performance_samples_time
           ON task_truth_v2_performance_samples(observed_at_ms DESC);",
    )
    .map_err(|error| error.to_string())
}

fn diagnostic_label(attempt: &super::model::ResolverAttemptV1) -> String {
    serde_json::to_value(attempt.diagnostic_status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Persists only bounded numeric/categorical metadata. In particular, this table
/// has no columns capable of storing OCR, Accessibility text, model hypotheses,
/// URLs, file paths, image handles, or raw provider responses.
pub(crate) fn persist_performance_sample(
    conn: &Connection,
    decision_id: &str,
    packet: &ObservationPacketV2,
    timings: &PerformanceStageTimingsV1,
    multimodal: &super::MultimodalShadowAuditV1,
) -> Result<(), String> {
    ensure_performance_schema(conn)?;
    let attempts = std::iter::once(&multimodal.first_pass_attempt)
        .chain(multimodal.second_pass_attempt.iter())
        .collect::<Vec<_>>();
    let request_build_ms = attempts
        .iter()
        .map(|attempt| attempt.request_build_latency_ms.max(0))
        .sum::<i64>();
    let provider_ms = attempts
        .iter()
        .map(|attempt| attempt.provider_latency_ms.max(0))
        .sum::<i64>();
    let image_count = attempts
        .iter()
        .filter_map(|attempt| attempt.request_audit.as_ref())
        .map(|audit| audit.image_count as i64)
        .sum::<i64>();
    let image_bytes = attempts
        .iter()
        .filter_map(|attempt| attempt.request_audit.as_ref())
        .map(|audit| audit.image_bytes as i64)
        .sum::<i64>();
    let structured_bytes = attempts
        .iter()
        .filter_map(|attempt| attempt.request_audit.as_ref())
        .map(|audit| audit.structured_bytes as i64)
        .sum::<i64>();
    let input_tokens = attempts
        .iter()
        .map(|attempt| {
            attempt.usage.input_tokens.unwrap_or_else(|| {
                attempt
                    .request_audit
                    .as_ref()
                    .map(|audit| audit.estimated_tokens as i64)
                    .unwrap_or(0)
            })
        })
        .map(|value| value.max(0))
        .sum::<i64>();
    let output_tokens = attempts
        .iter()
        .map(|attempt| attempt.usage.output_tokens.unwrap_or(0).max(0))
        .sum::<i64>();
    let privacy_excluded_frame_count = packet
        .semantic_keyframes
        .iter()
        .filter(|frame| {
            frame.partition == super::observation_packet::EvidencePartitionV2::Background
                || super::observation_packet::is_private_status(Some(&frame.privacy_status))
        })
        .count() as i64;
    let provider_attempt_count = attempts
        .iter()
        .map(|attempt| {
            attempt.provider_attempts.len().max(
                if attempt.request_audit.is_some() && attempt.provider_latency_ms > 0 {
                    1
                } else {
                    0
                },
            )
        })
        .sum::<usize>() as i64;
    let attempt_outcome = attempts
        .iter()
        .map(|attempt| diagnostic_label(attempt))
        .find(|status| status != "success")
        .unwrap_or_else(|| "success".to_string());
    let provider_outcome = match multimodal.verification.status {
        super::model::ResolutionStatusV1::InvalidResponse => "invalid_response".to_string(),
        super::model::ResolutionStatusV1::VerificationRejected => {
            "verification_rejected".to_string()
        }
        _ => attempt_outcome,
    };
    let stages_valid = timings.capture_to_packet_ms >= 0
        && timings.verification_persistence_ms >= 0
        && timings.total_manual_continue_ms >= 0
        && request_build_ms >= 0
        && provider_ms >= 0;
    let created_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(packet.observed_at_ms);
    let decision_id_hash = stable_hash(decision_id.as_bytes());
    let sample_id = format!(
        "mfti04-perf-{}",
        stable_hash(
            format!(
                "{decision_id_hash}:{}:{created_at_ms}",
                packet.observed_at_ms
            )
            .as_bytes()
        )
    );
    conn.execute(
        "INSERT INTO task_truth_v2_performance_samples (
           sample_id, schema_version, decision_id_hash, observed_at_ms, sample_complete,
           capture_to_packet_ms, request_build_ms, provider_ms,
           verification_persistence_ms, total_manual_continue_ms,
           image_count, image_bytes, structured_bytes, input_tokens, output_tokens,
           estimated_cost_usd, provider_outcome, provider_attempt_count,
           second_pass_ran, second_pass_cost_usd, privacy_excluded_frame_count,
           transported_frame_count, privacy_blocked_before_transport,
           background_multimodal_requests, created_at_ms
         ) VALUES (
           ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
           ?19,?20,?21,?22,?23,0,?24
         )",
        params![
            sample_id,
            PERFORMANCE_SAMPLE_SCHEMA,
            decision_id_hash,
            packet.observed_at_ms,
            0_i64,
            timings.capture_to_packet_ms,
            request_build_ms,
            provider_ms,
            timings.verification_persistence_ms,
            timings.total_manual_continue_ms,
            image_count,
            image_bytes,
            structured_bytes,
            input_tokens,
            output_tokens,
            multimodal
                .estimated_request_cost_usd
                .unwrap_or(0.0)
                .max(0.0),
            provider_outcome,
            provider_attempt_count,
            if multimodal.second_pass_ran {
                1_i64
            } else {
                0_i64
            },
            multimodal
                .estimated_second_pass_cost_usd
                .unwrap_or(0.0)
                .max(0.0),
            privacy_excluded_frame_count,
            image_count,
            1_i64,
            created_at_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    if !stages_valid {
        return Err("performance_sample_contains_negative_stage".to_string());
    }
    Ok(())
}

pub(crate) fn finalize_performance_total(
    conn: &Connection,
    decision_id: &str,
    total_manual_continue_ms: i64,
) -> Result<(), String> {
    if total_manual_continue_ms < 0 {
        return Err("performance_total_is_negative".to_string());
    }
    ensure_performance_schema(conn)?;
    let decision_id_hash = stable_hash(decision_id.as_bytes());
    conn.execute(
        "UPDATE task_truth_v2_performance_samples
         SET total_manual_continue_ms=?1, sample_complete=1
         WHERE sample_id=(
           SELECT sample_id FROM task_truth_v2_performance_samples
           WHERE decision_id_hash=?2 AND schema_version=?3 AND sample_complete=0
           ORDER BY created_at_ms DESC LIMIT 1
         )
           AND capture_to_packet_ms>=0 AND request_build_ms>=0 AND provider_ms>=0
           AND verification_persistence_ms>=0",
        params![
            total_manual_continue_ms,
            decision_id_hash,
            PERFORMANCE_SAMPLE_SCHEMA
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn performance_review_summary(
    conn: &Connection,
    decision_id: &str,
) -> Result<Option<PerformanceReviewSummaryV1>, String> {
    ensure_performance_schema(conn)?;
    let decision_id_hash = stable_hash(decision_id.as_bytes());
    conn.query_row(
        "SELECT sample_id, sample_complete, provider_outcome, capture_to_packet_ms,
                request_build_ms, provider_ms, verification_persistence_ms,
                total_manual_continue_ms, estimated_cost_usd, second_pass_ran
         FROM task_truth_v2_performance_samples
         WHERE decision_id_hash=?1 AND schema_version=?2
         ORDER BY created_at_ms DESC LIMIT 1",
        params![decision_id_hash, PERFORMANCE_SAMPLE_SCHEMA],
        |row| {
            Ok(PerformanceReviewSummaryV1 {
                schema: "smalltalk.mfti_04.performance_review_summary.v1".to_string(),
                sample_id: row.get(0)?,
                sample_complete: row.get::<_, i64>(1)? == 1,
                provider_outcome: row.get(2)?,
                capture_to_packet_ms: row.get(3)?,
                request_build_ms: row.get(4)?,
                provider_ms: row.get(5)?,
                verification_persistence_ms: row.get(6)?,
                total_manual_continue_ms: row.get(7)?,
                estimated_cost_usd: row.get(8)?,
                second_pass_ran: row.get::<_, i64>(9)? == 1,
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
}

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
        "evidence_window": {
            "start_ms": packet.semantic_keyframes.iter().map(|frame| frame.observed_at_ms).min().unwrap_or(packet.observed_at_ms),
            "end_ms": packet.semantic_keyframes.iter().map(|frame| frame.observed_at_ms).max().unwrap_or(packet.observed_at_ms),
        },
        "session_id": packet.session_id,
        "evidence_watermark": packet.evidence_watermark,
        "current_frame_id": packet.current_frame.frame_id,
        "effective_session_scope": if packet.session_id.is_some() { "session_scoped" } else { "unscoped" },
        "privacy_status": packet.current_frame.privacy_status,
        "model_eligible": packet.current_frame.model_eligible,
        "current_frame_readable_visual": packet.current_frame.model_eligible,
        "current_frame_structured_evidence": packet.canonical_elements.iter().any(|element| element.frame_id == packet.current_frame.frame_id),
        "frame_capacity_accounting": packet.size.frame_accounting,
        "ownership_distribution": packet.canonical_elements.iter().fold(std::collections::BTreeMap::<String, usize>::new(), |mut counts, element| { *counts.entry(element.ownership_kind.clone().unwrap_or_else(|| "unknown".into())).or_default() += 1; counts }),
        "region_distribution": packet.canonical_elements.iter().fold(std::collections::BTreeMap::<String, usize>::new(), |mut counts, element| { *counts.entry(format!("{:?}", element.region_role).to_ascii_lowercase()).or_default() += 1; counts }),
        "semantic_deltas": packet.frame_changes,
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
                "image_source_kind": keyframe.image_source_kind,
                "image_scope": keyframe.image_scope,
                "image_dimensions": [keyframe.image_width, keyframe.image_height],
                "image_rejection_reason": keyframe.image_rejection_reason,
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
                "source_frame_id": event.source_frame_id,
                "target_frame_id": event.target_frame_id,
                "target_element_id": event.target_element_id,
                "target_region": event.target_region,
                "window_id": event.window_id,
                "grounding_confidence": event.grounding_confidence,
                "missing_evidence": event.missing_evidence,
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

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn performance_table_has_only_safe_metadata_columns() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        ensure_performance_schema(&conn).expect("create performance schema");
        let columns = conn
            .prepare("PRAGMA table_info(task_truth_v2_performance_samples)")
            .expect("prepare columns")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query columns")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect columns");
        assert!(columns.contains(&"sample_complete".to_string()));
        assert!(columns.contains(&"provider_outcome".to_string()));
        for forbidden in [
            "ocr",
            "accessibility",
            "hypothesis",
            "path",
            "url",
            "response_body",
            "raw",
        ] {
            assert!(
                columns.iter().all(|column| !column.contains(forbidden)),
                "unsafe performance column: {forbidden}"
            );
        }
    }
}
