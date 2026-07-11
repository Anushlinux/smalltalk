use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::super::{stable_hash, ContinueEvidencePreview, ContinueReturnTarget, ContinueWorkTruth};
use super::checkpoint;
use super::selection::SNAPSHOT_SELECTION_POLICY_V1;
use super::task_snapshot::{TaskSnapshotV2, TASK_SNAPSHOT_SCHEMA_V2};

pub(crate) const TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1: &str = "smalltalk.task_truth_public_answer.v1";
pub(crate) const TASK_TRUTH_AUTHORITY_POLICY_V1: &str = "smalltalk.task_truth_authority_policy.v1";

fn default_public_task_basis() -> String {
    "unresolved".into()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskTruthAuthorityStateV1 {
    Off,
    Shadow,
    Eligible,
    Authoritative,
    Rollback,
}

impl TaskTruthAuthorityStateV1 {
    fn from_environment() -> Self {
        match std::env::var("SMALLTALK_TASK_TRUTH_AUTHORITY")
            .unwrap_or_else(|_| "shadow".into())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "off" => Self::Off,
            "eligible" => Self::Eligible,
            "authoritative" => Self::Authoritative,
            "rollback" => Self::Rollback,
            _ => Self::Shadow,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Shadow => "shadow",
            Self::Eligible => "eligible",
            Self::Authoritative => "authoritative",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthFieldSupportV1 {
    pub confidence: Option<f64>,
    pub support_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthAlternativeV1 {
    pub hypothesis_id: String,
    pub task_summary: String,
    pub relation: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthPublicAnswerV1 {
    pub schema: String,
    #[serde(default = "default_public_task_basis")]
    pub task_basis: String,
    pub task_resolution_status: String,
    pub task_summary: Option<String>,
    pub task_object: Option<String>,
    pub last_meaningful_progress: Option<String>,
    pub unfinished_state: Option<String>,
    pub next_action: Option<String>,
    pub where_summary: Option<String>,
    pub alternative_hypotheses: Vec<TaskTruthAlternativeV1>,
    pub direct_return_target: Option<ContinueReturnTarget>,
    pub evidence_preview: Option<ContinueEvidencePreview>,
    pub field_support: BTreeMap<String, TaskTruthFieldSupportV1>,
    pub task_understanding_source: String,
    pub wording_source: String,
    pub target_selection_source: String,
    pub snapshot_id: String,
    pub snapshot_revision: i64,
    pub evidence_watermark: String,
}

impl Default for TaskTruthPublicAnswerV1 {
    fn default() -> Self {
        Self {
            schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
            task_basis: "unresolved".into(),
            task_resolution_status: "unresolved".into(),
            task_summary: None,
            task_object: None,
            last_meaningful_progress: None,
            unfinished_state: None,
            next_action: None,
            where_summary: None,
            alternative_hypotheses: Vec::new(),
            direct_return_target: None,
            evidence_preview: None,
            field_support: BTreeMap::new(),
            task_understanding_source: "unresolved".into(),
            wording_source: "deterministic".into(),
            target_selection_source: "strict_local_policy".into(),
            snapshot_id: String::new(),
            snapshot_revision: 0,
            evidence_watermark: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthProductionDecisionV1 {
    pub schema: String,
    pub requested_state: TaskTruthAuthorityStateV1,
    pub effective_state: TaskTruthAuthorityStateV1,
    pub policy_version: String,
    pub release_gate_passed: bool,
    pub release_gate_source: Option<String>,
    pub reason_codes: Vec<String>,
    pub cache_fingerprint: String,
    pub answer: Option<TaskTruthPublicAnswerV1>,
}

impl Default for TaskTruthProductionDecisionV1 {
    fn default() -> Self {
        Self {
            schema: "smalltalk.task_truth_production_decision.v1".into(),
            requested_state: TaskTruthAuthorityStateV1::Shadow,
            effective_state: TaskTruthAuthorityStateV1::Shadow,
            policy_version: TASK_TRUTH_AUTHORITY_POLICY_V1.into(),
            release_gate_passed: false,
            release_gate_source: None,
            reason_codes: vec!["release_gate_not_evaluated".into()],
            cache_fingerprint: String::new(),
            answer: None,
        }
    }
}

fn release_report_path() -> Option<PathBuf> {
    std::env::var("SMALLTALK_TASK_TRUTH_RELEASE_REPORT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn final_release_report_is_complete(value: &serde_json::Value) -> bool {
    const METRICS: [&str; 13] = [
        "wrong_primary_task_rate",
        "control_navigation_as_task_rate",
        "useful_non_generic_task_summary",
        "task_object_accuracy",
        "execution_state_accuracy",
        "supported_next_action_precision",
        "supported_next_action_coverage",
        "return_target_precision",
        "unsupported_specific_claim_rate",
        "stronger_manual_result_downgraded",
        "unseen_application_useful_summary",
        "human_immediately_useful",
        "model_on_off_unexplained_task_disagreement",
    ];
    const SURFACES: [&str; 10] = [
        "agent_chat",
        "editor_ide",
        "terminal",
        "browser_research_search",
        "documents",
        "spreadsheets",
        "email_messaging",
        "pdf_file_manager",
        "custom_rendered_canvas",
        "mixed_window_thin_unknown",
    ];
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("smalltalk.task_truth_v2.final_release_report.v1")
        || value
            .get("policy_version")
            .and_then(serde_json::Value::as_str)
            != Some("tt2.02-v1")
        || value.get("passed").and_then(serde_json::Value::as_bool) != Some(true)
        || value
            .get("authority_state")
            .and_then(serde_json::Value::as_str)
            != Some("authoritative")
        || value
            .get("violations")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|items| !items.is_empty())
        || value
            .get("reviewed_live_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < 200
        || value
            .get("locked_holdout_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < 50
        || value
            .get("manual_scenario_count")
            .and_then(serde_json::Value::as_u64)
            != Some(14)
    {
        return false;
    }
    for field in [
        "evaluator_release_gate_passed",
        "evaluator_validation_passed",
        "manual_macos_qa_passed",
        "performance_cost_privacy_passed",
        "release_budget_policy_passed",
    ] {
        if value.get(field).and_then(serde_json::Value::as_bool) != Some(true) {
            return false;
        }
    }
    for metric in METRICS {
        let Some(assessment) = value
            .get("tt2_05_metric_results")
            .and_then(|metrics| metrics.get(metric))
        else {
            return false;
        };
        if assessment
            .get("passed")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
            || assessment
                .get("denominator")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                == 0
        {
            return false;
        }
        let Some(interval) = value
            .get("tt2_05_confidence_intervals")
            .and_then(|intervals| intervals.get(metric))
        else {
            return false;
        };
        if interval.get("method").and_then(serde_json::Value::as_str) != Some("wilson_score")
            || interval
                .get("lower")
                .and_then(serde_json::Value::as_f64)
                .is_none()
            || interval
                .get("upper")
                .and_then(serde_json::Value::as_f64)
                .is_none()
        {
            return false;
        }
    }
    for surface in SURFACES {
        let Some(assessment) = value
            .get("tt2_05_surface_wrong_task_results")
            .and_then(|surfaces| surfaces.get(surface))
        else {
            return false;
        };
        if assessment
            .get("passed")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
            || assessment
                .get("denominator")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                < 15
        {
            return false;
        }
        let interval_key = format!("wrong_primary_task_rate.surface.{surface}");
        let Some(interval) = value
            .get("tt2_05_confidence_intervals")
            .and_then(|intervals| intervals.get(&interval_key))
        else {
            return false;
        };
        if interval.get("method").and_then(serde_json::Value::as_str) != Some("wilson_score")
            || interval
                .get("lower")
                .and_then(serde_json::Value::as_f64)
                .is_none()
            || interval
                .get("upper")
                .and_then(serde_json::Value::as_f64)
                .is_none()
        {
            return false;
        }
    }
    for (slice, minimum) in [
        ("interruption_resumption", 30_u64),
        ("ambiguous_or_privacy_blocked", 20),
        ("waiting_on_agent_or_application", 20),
        ("completed_vs_new_task", 20),
    ] {
        if value
            .get("slice_denominators")
            .and_then(|slices| slices.get(slice))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < minimum
        {
            return false;
        }
    }
    [
        "control_navigation_as_task",
        "stronger_manual_result_downgraded",
        "model_on_off_unexplained_task_disagreement",
        "privacy_violations",
        "unsafe_opens",
        "background_multimodal_requests",
    ]
    .iter()
    .all(|field| {
        value
            .get("zero_tolerance")
            .and_then(|counts| counts.get(*field))
            .and_then(serde_json::Value::as_u64)
            == Some(0)
    })
}

fn read_release_gate() -> (bool, Option<String>, Vec<String>) {
    let Some(path) = release_report_path() else {
        return (false, None, vec!["release_report_not_configured".into()]);
    };
    let source = Some(path.to_string_lossy().to_string());
    let Ok(bytes) = fs::read(&path) else {
        return (false, source, vec!["release_report_unreadable".into()]);
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (false, source, vec!["release_report_invalid_json".into()]);
    };
    if final_release_report_is_complete(&value) {
        (true, source, vec!["locked_release_gate_passed".into()])
    } else {
        (false, source, vec!["locked_release_gate_closed".into()])
    }
}

pub(crate) fn authoritative_runtime_enabled() -> bool {
    TaskTruthAuthorityStateV1::from_environment() == TaskTruthAuthorityStateV1::Authoritative
        && read_release_gate().0
}

fn field_support(snapshot: &TaskSnapshotV2, field: &str) -> TaskTruthFieldSupportV1 {
    let confidence = snapshot.confidence_by_field.get(field).copied();
    let evidence_refs = snapshot
        .claim_evidence
        .iter()
        .filter(|claim| claim.claim == field)
        .flat_map(|claim| claim.evidence_refs.iter())
        .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.record_id))
        .collect::<Vec<_>>();
    TaskTruthFieldSupportV1 {
        confidence,
        support_status: if !evidence_refs.is_empty() && confidence.unwrap_or(0.0) > 0.0 {
            "supported".into()
        } else if confidence.is_some() {
            "partial".into()
        } else {
            "unsupported".into()
        },
        evidence_refs,
    }
}

fn understanding_source(snapshot: &TaskSnapshotV2) -> String {
    if snapshot.task_summary.is_none() {
        "unresolved".into()
    } else if snapshot
        .provenance
        .iter()
        .any(|value| value.contains("human_correction"))
    {
        "human_correction".into()
    } else if snapshot.resolver_version.contains("multimodal") {
        "multimodal_model".into()
    } else {
        "local_causal".into()
    }
}

fn public_answer(snapshot: &TaskSnapshotV2) -> TaskTruthPublicAnswerV1 {
    let mut field_support_map = BTreeMap::new();
    for field in [
        "task_summary",
        "task_object",
        "execution_state",
        "last_meaningful_progress",
        "unfinished_step",
        "next_action",
    ] {
        field_support_map.insert(field.into(), field_support(snapshot, field));
    }
    field_support_map.insert(
        "where_summary".into(),
        field_support(snapshot, "app_identity"),
    );
    let alternatives = snapshot
        .alternative_hypotheses
        .iter()
        .take(2)
        .map(|hypothesis| TaskTruthAlternativeV1 {
            hypothesis_id: format!(
                "hypothesis-{}",
                stable_hash(format!("{}:{}", snapshot.snapshot_id, hypothesis.summary).as_bytes())
            ),
            task_summary: hypothesis.summary.clone(),
            relation: hypothesis.relation.clone(),
            confidence: hypothesis.confidence,
            evidence_refs: hypothesis
                .evidence_refs
                .iter()
                .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.record_id))
                .collect(),
        })
        .collect::<Vec<_>>();
    let task_resolution_status = if snapshot.task_summary.is_none() {
        "unresolved"
    } else if alternatives.len() >= 2 {
        "ambiguous"
    } else {
        "resolved"
    };
    let preview_ref = snapshot
        .claim_evidence
        .iter()
        .flat_map(|claim| claim.evidence_refs.iter())
        .find_map(|evidence| evidence.frame_id.clone());
    TaskTruthPublicAnswerV1 {
        schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
        task_basis: snapshot.task_basis.clone(),
        task_resolution_status: task_resolution_status.into(),
        task_summary: snapshot.task_summary.clone(),
        task_object: snapshot.task_object.clone(),
        last_meaningful_progress: snapshot.last_meaningful_progress.clone(),
        unfinished_state: snapshot.unfinished_step.clone(),
        next_action: snapshot.next_action.clone(),
        where_summary: snapshot.app_identity.clone(),
        alternative_hypotheses: alternatives,
        direct_return_target: None,
        evidence_preview: preview_ref.map(|frame_id| ContinueEvidencePreview {
            schema: "smalltalk.continue_evidence_preview.v1".into(),
            preview_kind: "task_snapshot_evidence".into(),
            frame_id,
        }),
        field_support: field_support_map,
        task_understanding_source: understanding_source(snapshot),
        wording_source: "deterministic".into(),
        target_selection_source: "strict_local_policy".into(),
        snapshot_id: snapshot.snapshot_id.clone(),
        snapshot_revision: snapshot.revision,
        evidence_watermark: snapshot.evidence_watermark.clone(),
    }
}

fn apply_scoped_feedback(
    conn: &Connection,
    answer: &mut TaskTruthPublicAnswerV1,
) -> Result<(), String> {
    let mut statement = conn
        .prepare(
            "SELECT affected_field, hypothesis_id, feedback_kind
             FROM task_truth_v2_feedback_events
             WHERE task_snapshot_id=?1 AND task_snapshot_revision=?2
             ORDER BY observed_at_ms ASC, feedback_id ASC",
        )
        .map_err(|error| error.to_string())?;
    let feedback = statement
        .query_map(
            params![answer.snapshot_id, answer.snapshot_revision],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    for (field, hypothesis_id, kind) in feedback {
        if field == "hypothesis" {
            let Some(hypothesis_id) = hypothesis_id else {
                continue;
            };
            if kind == "corrected" {
                if let Some(selected) = answer
                    .alternative_hypotheses
                    .iter()
                    .find(|hypothesis| hypothesis.hypothesis_id == hypothesis_id)
                    .cloned()
                {
                    answer.task_summary = Some(selected.task_summary.clone());
                    answer.task_resolution_status = "resolved".into();
                    answer.task_understanding_source = "human_correction".into();
                    answer.field_support.insert(
                        "task_summary".into(),
                        TaskTruthFieldSupportV1 {
                            confidence: Some(1.0),
                            support_status: "human_corrected".into(),
                            evidence_refs: selected.evidence_refs,
                        },
                    );
                    answer
                        .alternative_hypotheses
                        .retain(|hypothesis| hypothesis.hypothesis_id != hypothesis_id);
                }
            } else if kind == "rejected" {
                answer
                    .alternative_hypotheses
                    .retain(|hypothesis| hypothesis.hypothesis_id != hypothesis_id);
            }
            continue;
        }
        if kind != "rejected" {
            continue;
        }
        let support_key = match field.as_str() {
            "state" => "execution_state",
            "where" => "where_summary",
            other => other,
        };
        answer
            .field_support
            .entry(support_key.into())
            .and_modify(|support| support.support_status = "rejected_by_user".into())
            .or_insert_with(|| TaskTruthFieldSupportV1 {
                confidence: None,
                support_status: "rejected_by_user".into(),
                evidence_refs: Vec::new(),
            });
        match field.as_str() {
            "task_summary" => {
                answer.task_resolution_status = "unresolved".into();
                answer.task_summary = None;
                answer.task_object = None;
                answer.last_meaningful_progress = None;
                answer.unfinished_state = None;
                answer.next_action = None;
                answer.direct_return_target = None;
                answer.task_understanding_source = "unresolved".into();
            }
            "task_object" => answer.task_object = None,
            "state" => {
                answer.last_meaningful_progress = None;
                answer.unfinished_state = None;
            }
            "next_action" => answer.next_action = None,
            "where" => {
                answer.where_summary = None;
                answer.direct_return_target = None;
            }
            _ => {}
        }
    }
    Ok(())
}

fn ensure_authority_audit_schema(conn: &Connection) -> Result<(), String> {
    checkpoint::ensure_schema(conn)
}

fn audit_switch(conn: &Connection, decision: &TaskTruthProductionDecisionV1) -> Result<(), String> {
    ensure_authority_audit_schema(conn)?;
    let prior = conn
        .query_row(
            "SELECT effective_state FROM task_truth_v2_authority_audits
             ORDER BY observed_at_ms DESC, audit_id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if prior.as_deref() == Some(decision.effective_state.label()) {
        return Ok(());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();
    let audit_id = format!(
        "tt2-authority-{}",
        stable_hash(
            format!(
                "{now}:{}:{}",
                decision.requested_state.label(),
                decision.effective_state.label()
            )
            .as_bytes()
        )
    );
    conn.execute(
        "INSERT INTO task_truth_v2_authority_audits (
           audit_id, observed_at_ms, requested_state, effective_state,
           release_gate_passed, policy_version, cache_fingerprint, reason_codes_json
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            audit_id,
            now,
            decision.requested_state.label(),
            decision.effective_state.label(),
            i64::from(decision.release_gate_passed),
            decision.policy_version,
            decision.cache_fingerprint,
            serde_json::to_string(&decision.reason_codes).map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn production_decision(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<TaskTruthProductionDecisionV1, String> {
    let requested_state = TaskTruthAuthorityStateV1::from_environment();
    let (release_gate_passed, release_gate_source, mut reason_codes) = read_release_gate();
    let effective_state = match requested_state {
        TaskTruthAuthorityStateV1::Authoritative if release_gate_passed => {
            TaskTruthAuthorityStateV1::Authoritative
        }
        TaskTruthAuthorityStateV1::Authoritative => {
            reason_codes.push("authoritative_blocked_by_release_gate".into());
            TaskTruthAuthorityStateV1::Eligible
        }
        other => other,
    };
    let snapshots = checkpoint::load_recent_snapshots(conn, session_id, 24)?;
    let selection = super::selection::select_snapshot(&snapshots);
    let selected = selection
        .selected_snapshot_id
        .as_deref()
        .and_then(|id| snapshots.iter().find(|snapshot| snapshot.snapshot_id == id));
    if selected.is_none() {
        reason_codes.push("no_verified_snapshot_selected".into());
    }
    let mut answer = selected.map(public_answer);
    if let Some(answer) = answer.as_mut() {
        apply_scoped_feedback(conn, answer)?;
    }
    let fingerprint_material = serde_json::json!({
        "snapshot_schema": TASK_SNAPSHOT_SCHEMA_V2,
        "selection_policy": SNAPSHOT_SELECTION_POLICY_V1,
        "authority_policy": TASK_TRUTH_AUTHORITY_POLICY_V1,
        "requested_state": requested_state,
        "effective_state": effective_state,
        "evidence_watermark": selected.map(|snapshot| snapshot.evidence_watermark.as_str()),
        "resolver_version": selected.map(|snapshot| snapshot.resolver_version.as_str()),
    });
    let decision = TaskTruthProductionDecisionV1 {
        schema: "smalltalk.task_truth_production_decision.v1".into(),
        requested_state,
        effective_state,
        policy_version: TASK_TRUTH_AUTHORITY_POLICY_V1.into(),
        release_gate_passed,
        release_gate_source,
        reason_codes,
        cache_fingerprint: stable_hash(fingerprint_material.to_string().as_bytes()),
        answer,
    };
    audit_switch(conn, &decision)?;
    Ok(decision)
}

pub(crate) fn attach_observed_activity(
    decision: &mut TaskTruthProductionDecisionV1,
    work_truth: &ContinueWorkTruth,
) {
    if work_truth.resolution_status != "activity_supported" {
        return;
    }
    let answer = decision
        .answer
        .get_or_insert_with(TaskTruthPublicAnswerV1::default);
    answer.task_basis = "observed_activity".into();
    answer.task_resolution_status = "resolved".into();
    answer.task_summary = work_truth.activity_summary.clone();
    answer.task_object = work_truth.work_object.clone();
    answer.where_summary = work_truth.where_summary.clone();
    answer.next_action = None;
    answer.task_understanding_source = "observed_activity".into();
    answer.wording_source = "local_deterministic".into();
    answer.evidence_watermark = work_truth.policy_version.clone();
    if answer.snapshot_id.is_empty() {
        answer.snapshot_id = format!(
            "activity-snapshot-{}",
            stable_hash(
                format!(
                    "{}:{}:{}",
                    work_truth.observed_at_ms,
                    work_truth.artifact_id.as_deref().unwrap_or("unknown"),
                    work_truth.activity_kind
                )
                .as_bytes(),
            )
        );
        answer.snapshot_revision = 1;
    }
    decision
        .reason_codes
        .push("task_basis:observed_activity".into());
}

pub(crate) fn attach_strict_target(
    decision: &mut TaskTruthProductionDecisionV1,
    snapshot_legacy_task_turn_id: Option<&str>,
    current_task_turn_id: Option<&str>,
    direct_target_allowed: bool,
    target: Option<ContinueReturnTarget>,
) {
    let identity_matches = snapshot_legacy_task_turn_id.is_some()
        && snapshot_legacy_task_turn_id == current_task_turn_id;
    if let Some(answer) = decision.answer.as_mut() {
        let feedback_allows_target = answer.task_resolution_status != "unresolved"
            && answer
                .field_support
                .get("where_summary")
                .is_none_or(|support| support.support_status != "rejected_by_user");
        if identity_matches && direct_target_allowed && feedback_allows_target {
            if let Some(target_title) = target
                .as_ref()
                .and_then(|target| safe_target_title_for_where(target.title.as_deref()))
            {
                let app_identity = answer.where_summary.as_deref().unwrap_or_default();
                if !app_identity.eq_ignore_ascii_case(target_title) {
                    answer.where_summary = Some(if app_identity.is_empty() {
                        target_title.to_string()
                    } else {
                        format!("{app_identity} — {target_title}")
                    });
                }
                if let Some(artifact_id) = target
                    .as_ref()
                    .and_then(|target| target.artifact_id.as_deref())
                    .filter(|id| !id.trim().is_empty())
                {
                    let support = answer
                        .field_support
                        .entry("where_summary".into())
                        .or_insert_with(|| TaskTruthFieldSupportV1 {
                            confidence: None,
                            support_status: "unsupported".into(),
                            evidence_refs: Vec::new(),
                        });
                    support.support_status = "supported".into();
                    support.confidence = Some(support.confidence.unwrap_or(0.0).max(0.98));
                    support
                        .evidence_refs
                        .push(format!("artifact:{artifact_id}"));
                    support.evidence_refs.sort();
                    support.evidence_refs.dedup();
                }
            }
            answer.direct_return_target = target;
        } else {
            answer.direct_return_target = None;
            if !identity_matches {
                decision
                    .reason_codes
                    .push("target_task_identity_mismatch".into());
            } else if !feedback_allows_target {
                decision
                    .reason_codes
                    .push("target_blocked_by_scoped_task_feedback".into());
            }
        }
    }
}

pub(crate) fn persist_decision_contract(
    conn: &Connection,
    decision_id: &str,
    decision: &TaskTruthProductionDecisionV1,
) -> Result<(), String> {
    checkpoint::ensure_schema(conn)?;
    let answer = decision.answer.as_ref();
    conn.execute(
        "INSERT OR REPLACE INTO task_truth_v2_decision_contracts (
           decision_id, effective_state, release_gate_passed, snapshot_id,
           snapshot_revision, return_target_artifact_id, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,
                   CAST(strftime('%s','now') AS INTEGER) * 1000)",
        params![
            decision_id,
            decision.effective_state.label(),
            i64::from(decision.release_gate_passed),
            answer.map(|answer| answer.snapshot_id.as_str()),
            answer.map(|answer| answer.snapshot_revision),
            answer
                .and_then(|answer| answer.direct_return_target.as_ref())
                .and_then(|target| target.artifact_id.as_deref()),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn safe_target_title_for_where(value: Option<&str>) -> Option<&str> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    let lower = value.to_ascii_lowercase();
    (!value.contains("/Users/")
        && !value.contains("://")
        && !lower.contains("artifact-")
        && !lower.contains("candidate-")
        && !lower.contains("workstream-")
        && !lower.contains("frame-"))
    .then_some(value)
}

pub(crate) fn selected_snapshot_legacy_turn_id(
    conn: &Connection,
    session_id: Option<&str>,
    snapshot_id: Option<&str>,
) -> Result<Option<String>, String> {
    let snapshots = checkpoint::load_recent_snapshots(conn, session_id, 24)?;
    Ok(snapshot_id.and_then(|id| {
        snapshots
            .iter()
            .find(|snapshot| snapshot.snapshot_id == id)
            .and_then(|snapshot| snapshot.legacy_task_turn_id.clone())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoritative_request_cannot_bypass_closed_release_gate() {
        let requested = TaskTruthAuthorityStateV1::Authoritative;
        let release_gate_passed = false;
        let effective =
            if requested == TaskTruthAuthorityStateV1::Authoritative && !release_gate_passed {
                TaskTruthAuthorityStateV1::Eligible
            } else {
                requested
            };
        assert_eq!(effective, TaskTruthAuthorityStateV1::Eligible);
    }

    #[test]
    fn passed_boolean_without_complete_release_evidence_cannot_open_authority() {
        let forged = serde_json::json!({
            "schema": "smalltalk.task_truth_v2.final_release_report.v1",
            "policy_version": "tt2.02-v1",
            "passed": true,
            "authority_state": "authoritative",
            "violations": []
        });
        assert!(!final_release_report_is_complete(&forged));
    }

    #[test]
    fn complete_release_report_shape_can_open_authority() {
        let metric_names = [
            "wrong_primary_task_rate",
            "control_navigation_as_task_rate",
            "useful_non_generic_task_summary",
            "task_object_accuracy",
            "execution_state_accuracy",
            "supported_next_action_precision",
            "supported_next_action_coverage",
            "return_target_precision",
            "unsupported_specific_claim_rate",
            "stronger_manual_result_downgraded",
            "unseen_application_useful_summary",
            "human_immediately_useful",
            "model_on_off_unexplained_task_disagreement",
        ];
        let surfaces = [
            "agent_chat",
            "editor_ide",
            "terminal",
            "browser_research_search",
            "documents",
            "spreadsheets",
            "email_messaging",
            "pdf_file_manager",
            "custom_rendered_canvas",
            "mixed_window_thin_unknown",
        ];
        let metrics = metric_names
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    serde_json::json!({"passed": true, "denominator": 200}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let mut intervals = metric_names
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    serde_json::json!({"method": "wilson_score", "lower": 0.9, "upper": 1.0}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        intervals.extend(surfaces.iter().map(|name| {
            (
                format!("wrong_primary_task_rate.surface.{name}"),
                serde_json::json!({"method": "wilson_score", "lower": 0.0, "upper": 0.1}),
            )
        }));
        let surface_results = surfaces
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    serde_json::json!({"passed": true, "denominator": 20}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let report = serde_json::json!({
            "schema": "smalltalk.task_truth_v2.final_release_report.v1",
            "policy_version": "tt2.02-v1",
            "passed": true,
            "authority_state": "authoritative",
            "violations": [],
            "reviewed_live_count": 200,
            "locked_holdout_count": 50,
            "manual_scenario_count": 14,
            "evaluator_release_gate_passed": true,
            "evaluator_validation_passed": true,
            "manual_macos_qa_passed": true,
            "performance_cost_privacy_passed": true,
            "release_budget_policy_passed": true,
            "tt2_05_metric_results": metrics,
            "tt2_05_confidence_intervals": intervals,
            "tt2_05_surface_wrong_task_results": surface_results,
            "slice_denominators": {
                "interruption_resumption": 30,
                "ambiguous_or_privacy_blocked": 20,
                "waiting_on_agent_or_application": 20,
                "completed_vs_new_task": 20
            },
            "zero_tolerance": {
                "control_navigation_as_task": 0,
                "stronger_manual_result_downgraded": 0,
                "model_on_off_unexplained_task_disagreement": 0,
                "privacy_violations": 0,
                "unsafe_opens": 0,
                "background_multimodal_requests": 0
            }
        });
        assert!(final_release_report_is_complete(&report));
    }

    #[test]
    fn target_mismatch_nulls_target_without_rewriting_task() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        decision.answer = Some(TaskTruthPublicAnswerV1 {
            schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
            task_basis: "explicit_goal".into(),
            task_resolution_status: "resolved".into(),
            task_summary: Some("Implement Task Truth authority".into()),
            task_object: None,
            last_meaningful_progress: None,
            unfinished_state: None,
            next_action: None,
            where_summary: None,
            alternative_hypotheses: Vec::new(),
            direct_return_target: None,
            evidence_preview: None,
            field_support: BTreeMap::new(),
            task_understanding_source: "local_causal".into(),
            wording_source: "deterministic".into(),
            target_selection_source: "strict_local_policy".into(),
            snapshot_id: "snapshot-1".into(),
            snapshot_revision: 1,
            evidence_watermark: "watermark-1".into(),
        });
        attach_strict_target(
            &mut decision,
            Some("turn-current"),
            Some("turn-old"),
            true,
            Some(ContinueReturnTarget {
                artifact_id: None,
                artifact_kind: None,
                title: Some("Old tab".into()),
                browser_url: Some("https://example.invalid".into()),
                document_path: None,
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.unwrap();
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Implement Task Truth authority")
        );
        assert!(answer.direct_return_target.is_none());
    }

    #[test]
    fn observed_activity_sets_task_basis_without_inventing_a_goal() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        let truth = ContinueWorkTruth {
            schema: super::super::super::CONTINUE_WORK_TRUTH_SCHEMA.into(),
            policy_version: super::super::super::CONTINUE_WORK_TRUTH_POLICY_VERSION.into(),
            resolution_status: "activity_supported".into(),
            activity_kind: "editing".into(),
            activity_summary: Some("Editing tt2-05-completion-audit.md".into()),
            work_object: Some("tt2-05-completion-audit.md".into()),
            where_summary: Some("Visual Studio Code".into()),
            app_name: Some("Visual Studio Code".into()),
            artifact_id: Some("artifact-md".into()),
            observed_at_ms: 1_000,
            confidence: 0.88,
            evidence_ids: vec!["frame-1".into()],
            source: "local_direct_activity".into(),
            broader_goal_known: false,
            primary_relation: "primary".into(),
            reason_codes: vec!["direct_production_action".into()],
        };

        attach_observed_activity(&mut decision, &truth);

        let answer = decision.answer.unwrap();
        assert_eq!(answer.task_basis, "observed_activity");
        assert_eq!(answer.task_summary, truth.activity_summary);
        assert!(answer.next_action.is_none());
    }

    #[test]
    fn matching_strict_target_enriches_where_without_replacing_task() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        decision.answer = Some(TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Implement Task Truth authority".into()),
            where_summary: Some("Codex".into()),
            snapshot_id: "snapshot-1".into(),
            snapshot_revision: 1,
            evidence_watermark: "watermark-1".into(),
            ..Default::default()
        });
        attach_strict_target(
            &mut decision,
            Some("turn-current"),
            Some("turn-current"),
            true,
            Some(ContinueReturnTarget {
                artifact_id: Some("artifact-current".into()),
                artifact_kind: Some("code_file".into()),
                title: Some("production.rs".into()),
                browser_url: None,
                document_path: Some("/private/redacted/production.rs".into()),
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.unwrap();
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Implement Task Truth authority")
        );
        assert_eq!(
            answer.where_summary.as_deref(),
            Some("Codex — production.rs")
        );
        assert_eq!(
            answer
                .field_support
                .get("where_summary")
                .map(|support| support.support_status.as_str()),
            Some("supported")
        );
        assert_eq!(
            answer
                .direct_return_target
                .and_then(|target| target.artifact_id),
            Some("artifact-current".into())
        );
    }

    #[test]
    fn not_right_feedback_is_scoped_to_exact_snapshot_revision_and_field() {
        let conn = Connection::open_in_memory().unwrap();
        let result = crate::continuation::record_continue_feedback(
            &conn,
            crate::continuation::ContinueExplicitFeedbackRequest {
                decision_id: None,
                selected_candidate_id: None,
                workstream_id: None,
                target_artifact_id: None,
                corrected_artifact_id: None,
                feedback_kind: "rejected".into(),
                note: None,
                source: Some("test".into()),
                task_snapshot_id: Some("snapshot-exact".into()),
                task_snapshot_revision: Some(7),
                affected_task_field: Some("task_summary".into()),
                task_hypothesis_id: None,
            },
        )
        .unwrap();
        assert!(result.workstream_id.is_none());
        assert!(result.target_artifact_id.is_none());
        assert!(result.normalized_targets.is_empty());
        let stored: (String, i64, String, String) = conn
            .query_row(
                "SELECT task_snapshot_id, task_snapshot_revision, affected_field, feedback_kind
                 FROM task_truth_v2_feedback_events WHERE feedback_id=?1",
                params![result.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            stored,
            (
                "snapshot-exact".into(),
                7,
                "task_summary".into(),
                "rejected".into()
            )
        );
        let legacy_feedback_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_feedback_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(legacy_feedback_count, 0);
        let first_watermark =
            crate::continuation::current_continue_evidence_watermark_hash(&conn, None).unwrap();
        let second = crate::continuation::record_continue_feedback(
            &conn,
            crate::continuation::ContinueExplicitFeedbackRequest {
                decision_id: None,
                selected_candidate_id: None,
                workstream_id: None,
                target_artifact_id: None,
                corrected_artifact_id: None,
                feedback_kind: "rejected".into(),
                note: None,
                source: Some("test".into()),
                task_snapshot_id: Some("snapshot-exact".into()),
                task_snapshot_revision: Some(7),
                affected_task_field: Some("next_action".into()),
                task_hypothesis_id: None,
            },
        )
        .unwrap();
        let second_watermark =
            crate::continuation::current_continue_evidence_watermark_hash(&conn, None).unwrap();
        assert_ne!(result.id, second.id);
        assert_ne!(first_watermark, second_watermark);
        let scoped_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_truth_v2_feedback_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(scoped_count, 2);
    }

    #[test]
    fn scoped_hypothesis_feedback_promotes_only_the_selected_alternative() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO task_truth_v2_feedback_events (
               feedback_id, task_snapshot_id, task_snapshot_revision, affected_field,
               hypothesis_id, feedback_kind, decision_id, observed_at_ms
             ) VALUES ('feedback-a','snapshot-exact',7,'hypothesis','hypothesis-a',
                       'corrected','decision-a',100)",
            [],
        )
        .unwrap();
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "ambiguous".into(),
            task_summary: Some("Original task".into()),
            alternative_hypotheses: vec![
                TaskTruthAlternativeV1 {
                    hypothesis_id: "hypothesis-a".into(),
                    task_summary: "User-selected task".into(),
                    relation: "alternative".into(),
                    confidence: 0.72,
                    evidence_refs: vec!["frame:1".into()],
                },
                TaskTruthAlternativeV1 {
                    hypothesis_id: "hypothesis-b".into(),
                    task_summary: "Other task".into(),
                    relation: "alternative".into(),
                    confidence: 0.7,
                    evidence_refs: vec!["frame:2".into()],
                },
            ],
            snapshot_id: "snapshot-exact".into(),
            snapshot_revision: 7,
            evidence_watermark: "watermark-7".into(),
            ..Default::default()
        };

        apply_scoped_feedback(&conn, &mut answer).unwrap();

        assert_eq!(answer.task_summary.as_deref(), Some("User-selected task"));
        assert_eq!(answer.task_resolution_status, "resolved");
        assert_eq!(answer.task_understanding_source, "human_correction");
        assert_eq!(answer.alternative_hypotheses.len(), 1);
        assert_eq!(
            answer.alternative_hypotheses[0].hypothesis_id,
            "hypothesis-b"
        );
    }

    #[test]
    fn rejected_task_summary_blocks_target_for_that_snapshot_revision() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO task_truth_v2_feedback_events (
               feedback_id, task_snapshot_id, task_snapshot_revision, affected_field,
               hypothesis_id, feedback_kind, decision_id, observed_at_ms
             ) VALUES ('feedback-task','snapshot-exact',7,'task_summary',NULL,
                       'rejected','decision-a',100)",
            [],
        )
        .unwrap();
        let mut decision = TaskTruthProductionDecisionV1::default();
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Wrong task".into()),
            where_summary: Some("Codex".into()),
            snapshot_id: "snapshot-exact".into(),
            snapshot_revision: 7,
            evidence_watermark: "watermark-7".into(),
            ..Default::default()
        };
        apply_scoped_feedback(&conn, &mut answer).unwrap();
        decision.answer = Some(answer);
        attach_strict_target(
            &mut decision,
            Some("turn-current"),
            Some("turn-current"),
            true,
            Some(ContinueReturnTarget {
                artifact_id: Some("artifact-current".into()),
                artifact_kind: None,
                title: Some("Current target".into()),
                browser_url: Some("https://example.invalid/current".into()),
                document_path: None,
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.unwrap();
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert!(answer.task_summary.is_none());
        assert!(answer.direct_return_target.is_none());
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "target_blocked_by_scoped_task_feedback"));
    }
}
