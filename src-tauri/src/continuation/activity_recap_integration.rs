use super::activity_recap::{
    sanitize_public_text, ActivityConfidence, ActivityEvidenceAnchorType,
    ActivityEvidenceConfidence, ActivityEvidenceSource, ActivityEvidenceSpan,
    ActivityRecapValidationStatus, ContinueActivityRecap, ACTIVITY_RECAP_SCHEMA,
};
use super::activity_recap_detours::DetourRecapResult;
use super::activity_recap_inputs::{ActivityRecapInputs, MemoryFact};
use super::activity_recap_model::ActivityRecapSynthesisAudit;
use super::activity_recap_objective::ActivityWorkLabelResult;
use super::activity_recap_open_loop::LastStateRecapResult;
use super::activity_recap_segments::StitchedActivityTimeline;
use super::{
    memory_keywords, rebuild_continue_memory_edges, stable_hash, ContinueEvidenceWatermark,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub(crate) const ACTIVITY_RECAP_PROOF_SCHEMA: &str = "smalltalk.activity_recap_decision_proof.v1";
pub(crate) const ACTIVITY_RECAP_PIPELINE_VERSION: &str = "p5.08.v1";

const MEMORY_TYPE_WORKSTREAM_SUMMARY: &str = "activity_workstream_summary";
const MEMORY_TYPE_PRIMARY_LABEL: &str = "activity_primary_label";
const MEMORY_TYPE_LAST_GOOD_RECAP: &str = "activity_last_good_recap";
const MEMORY_TYPE_REJECTED: &str = "activity_recap_rejected";
const MEMORY_CREATED_BY: &str = "p5_activity_recap";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ActivityRecapInputSummary {
    pub schema: String,
    pub evidence_watermark: Option<String>,
    pub current_surface_id: Option<String>,
    pub selected_workstream_id: Option<String>,
    pub selected_candidate_id: Option<String>,
    pub return_target_artifact_id: Option<String>,
    pub segment_count: usize,
    pub action_count: usize,
    pub semantic_moment_count: usize,
    pub open_loop_count: usize,
    pub branch_context_count: usize,
    pub surface_snapshot_count: usize,
    pub memory_fact_count: usize,
    pub support_evidence_count: usize,
    pub input_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ActivityRecapDecisionProof {
    pub schema: String,
    pub pipeline_version: String,
    pub input_summary: ActivityRecapInputSummary,
    pub stitched_timeline: StitchedActivityTimeline,
    pub work_labels: ActivityWorkLabelResult,
    pub detours: DetourRecapResult,
    pub last_state: LastStateRecapResult,
    pub model_status: Value,
    pub validation_failures: Vec<String>,
    pub final_recap: ContinueActivityRecap,
}

impl Default for ActivityRecapDecisionProof {
    fn default() -> Self {
        Self {
            schema: ACTIVITY_RECAP_PROOF_SCHEMA.to_string(),
            pipeline_version: ACTIVITY_RECAP_PIPELINE_VERSION.to_string(),
            input_summary: ActivityRecapInputSummary {
                schema: "smalltalk.activity_recap_input_summary.v1".to_string(),
                ..ActivityRecapInputSummary::default()
            },
            stitched_timeline: StitchedActivityTimeline::default(),
            work_labels: ActivityWorkLabelResult::default(),
            detours: DetourRecapResult::default(),
            last_state: LastStateRecapResult::default(),
            model_status: json!({"attempted": false, "result": "not_available"}),
            validation_failures: Vec::new(),
            final_recap: ContinueActivityRecap::default(),
        }
    }
}

impl ActivityRecapDecisionProof {
    pub(crate) fn from_recap(recap: ContinueActivityRecap) -> Self {
        Self {
            final_recap: recap,
            ..Self::default()
        }
    }
}

pub(crate) fn activity_recap_policy_fingerprint(
    model_enabled: bool,
    model_override: Option<&str>,
) -> String {
    stable_hash(
        format!(
            "{}:{}:{}:{}",
            ACTIVITY_RECAP_PIPELINE_VERSION,
            ACTIVITY_RECAP_SCHEMA,
            model_enabled,
            model_override.unwrap_or("default")
        )
        .as_bytes(),
    )
}

pub(crate) fn activity_recap_watermark_hash(
    watermark: &ContinueEvidenceWatermark,
    policy_fingerprint: &str,
) -> String {
    stable_hash(
        format!(
            "{}:{}:{}",
            ACTIVITY_RECAP_PIPELINE_VERSION, policy_fingerprint, watermark.hash
        )
        .as_bytes(),
    )
}

pub(crate) fn build_activity_recap_proof(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    work_labels: &ActivityWorkLabelResult,
    detours: &DetourRecapResult,
    last_state: &LastStateRecapResult,
    synthesis_audit: &ActivityRecapSynthesisAudit,
    recap: &ContinueActivityRecap,
) -> ActivityRecapDecisionProof {
    let validation_failures = synthesis_audit
        .validation
        .get("failures")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .take(12)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    ActivityRecapDecisionProof {
        schema: ACTIVITY_RECAP_PROOF_SCHEMA.to_string(),
        pipeline_version: ACTIVITY_RECAP_PIPELINE_VERSION.to_string(),
        input_summary: ActivityRecapInputSummary {
            schema: "smalltalk.activity_recap_input_summary.v1".to_string(),
            evidence_watermark: inputs.decision_context.evidence_watermark.clone(),
            current_surface_id: inputs
                .current_surface
                .as_ref()
                .map(|surface| surface.surface_id.clone()),
            selected_workstream_id: inputs
                .selected_workstream
                .as_ref()
                .map(|workstream| workstream.workstream_id.clone()),
            selected_candidate_id: inputs
                .selected_candidate
                .as_ref()
                .map(|candidate| candidate.candidate_id.clone()),
            return_target_artifact_id: inputs
                .return_target
                .as_ref()
                .map(|target| target.artifact_id.clone()),
            segment_count: inputs.recent_segments.len(),
            action_count: inputs.recent_actions.len(),
            semantic_moment_count: inputs.recent_moments.len(),
            open_loop_count: inputs.open_loops.len(),
            branch_context_count: inputs.branch_contexts.len(),
            surface_snapshot_count: inputs.surface_snapshots.len(),
            memory_fact_count: inputs.memory_facts.len(),
            support_evidence_count: inputs.support_evidence.len(),
            input_warnings: inputs.input_warnings.iter().take(12).cloned().collect(),
        },
        stitched_timeline: timeline.clone(),
        work_labels: work_labels.clone(),
        detours: detours.clone(),
        last_state: last_state.clone(),
        model_status: json!({
            "validation": synthesis_audit.validation,
            "fallback": synthesis_audit.fallback,
        }),
        validation_failures,
        final_recap: recap.clone(),
    }
}

/// Uses narrative memory only to explain a thin same-artifact/workstream decision.
/// It deliberately cannot create or modify a target, candidate, or openability fact.
pub(crate) fn apply_prior_activity_memory(
    mut recap: ContinueActivityRecap,
    inputs: &ActivityRecapInputs,
) -> ContinueActivityRecap {
    if matches!(
        recap.activity_confidence,
        ActivityConfidence::Medium | ActivityConfidence::High
    ) {
        return recap;
    }
    let current_artifact_id = inputs
        .current_surface
        .as_ref()
        .and_then(|surface| surface.artifact_id.as_deref());
    let selected_workstream_id = inputs
        .selected_workstream
        .as_ref()
        .map(|workstream| workstream.workstream_id.as_str());
    if related_memory_contradiction(
        &inputs.memory_facts,
        current_artifact_id,
        selected_workstream_id,
    ) {
        return recap;
    }

    let prior = inputs
        .memory_facts
        .iter()
        .filter(|memory| {
            memory.relation == "support"
                && memory.memory_type == MEMORY_TYPE_LAST_GOOD_RECAP
                && memory.confidence >= 0.60
                && memory.feedback_score >= 0.0
                && memory_matches(memory, current_artifact_id, selected_workstream_id)
        })
        .max_by(|left, right| {
            left.retrieval_score
                .partial_cmp(&right.retrieval_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.last_seen_at_ms.cmp(&right.last_seen_at_ms))
        });
    let Some(prior) = prior else {
        return recap;
    };
    let Some(summary) = prior
        .summary
        .as_deref()
        .and_then(|summary| sanitize_public_text(format!("Prior context: {summary}"), 280))
    else {
        return recap;
    };

    let label = inputs
        .memory_facts
        .iter()
        .filter(|memory| {
            memory.relation == "support"
                && memory.memory_type == MEMORY_TYPE_PRIMARY_LABEL
                && memory.confidence >= 0.60
                && memory.feedback_score >= 0.0
                && memory_matches(memory, current_artifact_id, selected_workstream_id)
        })
        .max_by_key(|memory| memory.last_seen_at_ms)
        .and_then(|memory| memory.summary.clone());

    recap.primary_work_summary = Some(summary.clone());
    recap.primary_work_label = recap.primary_work_label.or(label);
    recap.activity_confidence = ActivityConfidence::Low;
    recap.validation_status = ActivityRecapValidationStatus::Thin;
    recap.evidence_spans.push(ActivityEvidenceSpan {
        claim_key: "primary_work_summary".to_string(),
        claim_text: summary,
        anchor_type: ActivityEvidenceAnchorType::MemoryCell,
        anchor_ids: vec![prior.memory_id.clone()],
        confidence: ActivityEvidenceConfidence::Low,
        source: ActivityEvidenceSource::Local,
    });
    push_unique(
        &mut recap.missing_evidence,
        "The activity description comes from prior context; fresh task detail is thin.",
    );
    push_unique(&mut recap.warnings, "activity_recap:prior_context_only");
    recap.sanitized()
}

pub(crate) fn promote_validated_activity_recap_memory(
    conn: &Connection,
    decision_id: &str,
    selected_candidate_id: Option<&str>,
    workstream_id: Option<&str>,
    artifact_id: Option<&str>,
    recap: &ContinueActivityRecap,
    now_ms: i64,
) -> Result<usize, String> {
    if recap.validation_status != ActivityRecapValidationStatus::Valid
        || !matches!(
            recap.activity_confidence,
            ActivityConfidence::Medium | ActivityConfidence::High
        )
        || (workstream_id.is_none() && artifact_id.is_none())
        || !has_stable_non_memory_anchor(recap)
    {
        return Ok(0);
    }
    if !activity_memory_scope_is_safe(conn, artifact_id)? {
        return Ok(0);
    }
    let confidence = match recap.activity_confidence {
        ActivityConfidence::High => 0.88,
        ActivityConfidence::Medium => 0.72,
        _ => return Ok(0),
    };
    let evidence_handles = recap
        .evidence_spans
        .iter()
        .filter(|span| span.anchor_type != ActivityEvidenceAnchorType::MemoryCell)
        .flat_map(|span| span.anchor_ids.iter().cloned())
        .take(16)
        .collect::<Vec<_>>();
    let source_anchor = json!({
        "kind": "activity_recap",
        "schema": recap.schema,
        "decision_id": decision_id,
        "selected_candidate_id": selected_candidate_id,
        "workstream_id": workstream_id,
        "artifact_id": artifact_id,
        "evidence_handles": evidence_handles,
    });
    let values = [
        (
            MEMORY_TYPE_WORKSTREAM_SUMMARY,
            recap.primary_work_summary.as_deref(),
            0.84,
        ),
        (
            MEMORY_TYPE_PRIMARY_LABEL,
            recap.primary_work_label.as_deref(),
            0.76,
        ),
        (
            MEMORY_TYPE_LAST_GOOD_RECAP,
            recap.primary_work_summary.as_deref(),
            0.90,
        ),
    ];
    let mut changed = 0;
    for (memory_type, summary, importance) in values {
        let Some(summary) = summary.and_then(|value| sanitize_public_text(value.to_string(), 240))
        else {
            continue;
        };
        let id = recap_memory_id(memory_type, workstream_id, artifact_id);
        let content_hash =
            stable_hash(format!("{}:{}:{}", memory_type, summary, source_anchor).as_bytes());
        let existing = conn
            .query_row(
                "SELECT content_hash, feedback_score FROM continue_memory_cells WHERE id = ?1",
                params![id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, f64>(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if existing
            .as_ref()
            .is_some_and(|(_, feedback_score)| *feedback_score < 0.0)
            || existing
                .as_ref()
                .is_some_and(|(hash, _)| hash.as_deref() == Some(content_hash.as_str()))
        {
            continue;
        }
        let keywords = memory_keywords(&[
            workstream_id,
            artifact_id,
            Some(memory_type),
            Some(summary.as_str()),
        ]);
        super::upsert_continue_memory_cell(
            conn,
            &id,
            workstream_id,
            artifact_id,
            None,
            None,
            memory_type,
            &summary,
            &keywords,
            &["activity_recap".to_string(), "narrative_only".to_string()],
            &source_anchor,
            now_ms,
            now_ms,
            confidence,
            importance,
            None,
            "summary_only",
            0.0,
            now_ms,
            MEMORY_CREATED_BY,
        )?;
        changed += 1;
    }
    if changed > 0 {
        rebuild_continue_memory_edges(conn, now_ms)?;
    }
    Ok(changed)
}

pub(crate) fn apply_activity_recap_feedback_to_memory(
    conn: &Connection,
    feedback_kind: &str,
    workstream_id: Option<&str>,
    target_artifact_id: Option<&str>,
    chosen_artifact_id: Option<&str>,
    now_ms: i64,
) -> Result<usize, String> {
    if feedback_kind == "user_next_step_note" {
        return Ok(0);
    }
    if feedback_kind == "accepted" {
        let accepted_workstream = target_artifact_id
            .is_none()
            .then_some(workstream_id)
            .flatten();
        return update_activity_recap_memory_feedback(
            conn,
            accepted_workstream,
            target_artifact_id,
            false,
            now_ms,
        );
    }
    if !matches!(
        feedback_kind,
        "rejected" | "ignored" | "corrected" | "artifact_only_evidence" | "ignored_workstream"
    ) {
        return Ok(0);
    }
    let scoped_workstream = (feedback_kind == "ignored_workstream")
        .then_some(workstream_id)
        .flatten();
    let scoped_artifact = if feedback_kind == "ignored_workstream" {
        None
    } else {
        target_artifact_id
    };
    let mut changed = update_activity_recap_memory_feedback(
        conn,
        scoped_workstream,
        scoped_artifact,
        true,
        now_ms,
    )?;
    if feedback_kind == "corrected" && chosen_artifact_id.is_some() {
        changed +=
            update_activity_recap_memory_feedback(conn, None, chosen_artifact_id, false, now_ms)?;
    }
    Ok(changed)
}

fn update_activity_recap_memory_feedback(
    conn: &Connection,
    workstream_id: Option<&str>,
    artifact_id: Option<&str>,
    negative: bool,
    now_ms: i64,
) -> Result<usize, String> {
    if workstream_id.is_none() && artifact_id.is_none() {
        return Ok(0);
    }
    let sql = if negative {
        "UPDATE continue_memory_cells
         SET memory_type = ?1,
             confidence = MAX(0.05, confidence * 0.35),
             importance = MAX(0.05, importance * 0.50),
             feedback_score = MIN(feedback_score, -0.55),
             rejected_count = rejected_count + 1,
             updated_at_ms = ?2
         WHERE created_by = ?3
           AND ((?4 IS NOT NULL AND workstream_id = ?4)
             OR (?5 IS NOT NULL AND artifact_id = ?5))"
    } else {
        "UPDATE continue_memory_cells
         SET confidence = MIN(1.0, confidence + 0.04),
             feedback_score = MAX(feedback_score, 0.20),
             accepted_count = accepted_count + 1,
             last_reinforced_at_ms = ?2,
             updated_at_ms = ?2
         WHERE created_by = ?3
           AND memory_type <> ?1
           AND ((?4 IS NOT NULL AND workstream_id = ?4)
             OR (?5 IS NOT NULL AND artifact_id = ?5))"
    };
    conn.execute(
        sql,
        params![
            MEMORY_TYPE_REJECTED,
            now_ms,
            MEMORY_CREATED_BY,
            workstream_id,
            artifact_id,
        ],
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn narrative_memory_type(memory_type: &str) -> bool {
    matches!(
        memory_type,
        MEMORY_TYPE_WORKSTREAM_SUMMARY
            | MEMORY_TYPE_PRIMARY_LABEL
            | MEMORY_TYPE_LAST_GOOD_RECAP
            | MEMORY_TYPE_REJECTED
    )
}

pub(crate) fn rejected_narrative_memory_type(memory_type: &str) -> bool {
    memory_type == MEMORY_TYPE_REJECTED
}

fn recap_memory_id(
    memory_type: &str,
    workstream_id: Option<&str>,
    artifact_id: Option<&str>,
) -> String {
    format!(
        "continue-memory-{}",
        stable_hash(
            format!(
                "activity_recap:{}:{}:{}",
                memory_type,
                workstream_id.unwrap_or("none"),
                artifact_id.unwrap_or("none")
            )
            .as_bytes(),
        )
    )
}

fn activity_memory_scope_is_safe(
    conn: &Connection,
    artifact_id: Option<&str>,
) -> Result<bool, String> {
    let Some(artifact_id) = artifact_id else {
        return Ok(true);
    };
    let artifact = conn
        .query_row(
            "SELECT display_title, browser_url, document_path, privacy_status
             FROM continue_artifacts WHERE id = ?1",
            params![artifact_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(artifact.is_some_and(|(title, url, path, privacy)| {
        !super::memory_surface_excluded(
            title.as_deref(),
            url.as_deref(),
            path.as_deref(),
            privacy.as_deref(),
        )
    }))
}

fn has_stable_non_memory_anchor(recap: &ContinueActivityRecap) -> bool {
    recap.evidence_spans.iter().any(|span| {
        span.anchor_type != ActivityEvidenceAnchorType::MemoryCell && !span.anchor_ids.is_empty()
    })
}

fn memory_matches(
    memory: &MemoryFact,
    artifact_id: Option<&str>,
    workstream_id: Option<&str>,
) -> bool {
    artifact_id.is_some_and(|id| memory.artifact_id.as_deref() == Some(id))
        || workstream_id.is_some_and(|id| memory.workstream_id.as_deref() == Some(id))
}

fn related_memory_contradiction(
    memories: &[MemoryFact],
    artifact_id: Option<&str>,
    workstream_id: Option<&str>,
) -> bool {
    memories.iter().any(|memory| {
        memory_matches(memory, artifact_id, workstream_id)
            && (memory.relation.contains("contradict")
                || memory.feedback_score < 0.0
                || rejected_narrative_memory_type(&memory.memory_type))
    })
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::activity_recap_inputs::{
        ActivityRecapDecisionContext, CurrentSurfaceFact, ExistingQualityFacts,
    };

    fn thin_inputs(memory_facts: Vec<MemoryFact>) -> ActivityRecapInputs {
        ActivityRecapInputs {
            schema: "smalltalk.activity_recap_inputs.v1".to_string(),
            decision_context: ActivityRecapDecisionContext {
                decision_id_seed: None,
                mode: "normal".to_string(),
                lookback_ms: 60_000,
                evidence_watermark: Some("watermark".to_string()),
                output_mode: Some("no_clear_continuation".to_string()),
            },
            current_surface: Some(CurrentSurfaceFact {
                surface_id: "surface-current".to_string(),
                artifact_id: Some("artifact-same".to_string()),
                app_name: Some("Codex".to_string()),
                display_title: None,
                domain: None,
                activity_state: None,
                task_state: None,
                observed_at_ms: 2_000,
                evidence_quality: "thin".to_string(),
                openability: "app_focus_only".to_string(),
                focus_confidence: 0.45,
                identity_confidence: Some(0.35),
                snapshot_id: None,
                evidence_ids: vec!["frame-current".to_string()],
                missing_evidence: vec!["missing_task_identity".to_string()],
                claim_eligible: false,
            }),
            selected_workstream: None,
            selected_candidate: None,
            return_target: None,
            resume_work_target: None,
            recent_segments: Vec::new(),
            recent_actions: Vec::new(),
            recent_moments: Vec::new(),
            open_loops: Vec::new(),
            workstream_states: Vec::new(),
            branch_contexts: Vec::new(),
            surface_snapshots: Vec::new(),
            support_evidence: Vec::new(),
            memory_facts,
            existing_quality: ExistingQualityFacts {
                p0_quality_signals: None,
                current_surface_resolution: None,
                evidence_freshness_ledger: None,
                app_activity_summary: None,
                quality_gate: None,
            },
            input_warnings: Vec::new(),
        }
    }

    fn prior_memory(memory_type: &str, relation: &str, feedback_score: f64) -> MemoryFact {
        MemoryFact {
            memory_id: "memory-prior".to_string(),
            workstream_id: Some("workstream-prior".to_string()),
            artifact_id: Some("artifact-same".to_string()),
            episode_id: None,
            action_id: None,
            memory_type: memory_type.to_string(),
            relation: relation.to_string(),
            summary: Some("Implementing the P5 decision integration".to_string()),
            last_seen_at_ms: 1_000,
            confidence: 0.82,
            importance: 0.85,
            retrieval_score: 0.9,
            feedback_score,
            retrieval_reasons: vec!["artifact_match".to_string()],
            supports_candidate_ids: Vec::new(),
            contradicts_candidate_ids: Vec::new(),
        }
    }

    #[test]
    fn prior_activity_memory_supports_thin_recap_without_creating_target_claims() {
        let recap = apply_prior_activity_memory(
            ContinueActivityRecap::default(),
            &thin_inputs(vec![
                prior_memory(MEMORY_TYPE_LAST_GOOD_RECAP, "support", 0.0),
                prior_memory(MEMORY_TYPE_PRIMARY_LABEL, "support", 0.0),
            ]),
        );
        assert_eq!(recap.activity_confidence, ActivityConfidence::Low);
        assert!(recap
            .primary_work_summary
            .as_deref()
            .is_some_and(|summary| summary.starts_with("Prior context:")));
        assert_eq!(recap.target_confidence, ActivityConfidence::None);
        assert!(recap.why_this_target.is_none());
        assert!(recap.why_no_safe_target.is_none());
        assert!(recap.evidence_spans.iter().any(|span| {
            span.anchor_type == ActivityEvidenceAnchorType::MemoryCell
                && span.anchor_ids == vec!["memory-prior".to_string()]
        }));
    }

    #[test]
    fn rejected_activity_memory_never_overrides_fresh_thin_state() {
        let recap = apply_prior_activity_memory(
            ContinueActivityRecap::default(),
            &thin_inputs(vec![prior_memory(
                MEMORY_TYPE_REJECTED,
                "contradiction",
                -0.55,
            )]),
        );
        assert!(recap.primary_work_summary.is_none());
        assert_eq!(recap.activity_confidence, ActivityConfidence::None);
    }
}
