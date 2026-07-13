use super::activity_recap::{
    sanitize_public_text, ActivityConfidence, ActivityCurrentState, ActivityDetourRole,
    ActivityEvidenceConfidence, ActivityRecapGeneratedBy, ActivityRecapValidationStatus,
    ContinueActivityRecap,
};
use super::activity_recap_inputs::ActivityRecapInputs;
use super::activity_recap_objective::ActivityWorkLabelResult;
use super::activity_recap_segments::StitchedActivityTimeline;
use super::activity_recap_truth::{
    ActivityRecapLocalGuard, ActivityRecapTaskIdentity, ActivityRecapTaskTruth,
};
use super::activity_recap_validation::validate_activity_recap_model_output;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

pub(crate) const ACTIVITY_RECAP_MODEL_PACK_SCHEMA: &str = "smalltalk.activity_recap_model_pack.v2";
const MAX_OBJECTIVE_TERMS: usize = 8;
const MAX_MISSING_EVIDENCE: usize = 8;
const MAX_TERM_BANK: usize = 64;

pub(crate) struct ActivityRecapSynthesisContext<'a> {
    pub enabled: bool,
    pub model_override: Option<&'a str>,
    pub retain_audit_payloads: bool,
    pub inputs: &'a ActivityRecapInputs,
    pub timeline: &'a StitchedActivityTimeline,
    pub work_label: &'a ActivityWorkLabelResult,
    pub local_recap: &'a ContinueActivityRecap,
    pub task_truth: &'a ActivityRecapTaskTruth,
    pub local_guard: &'a ActivityRecapLocalGuard,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivityRecapSynthesisResult {
    pub recap: ContinueActivityRecap,
    pub audit: ActivityRecapSynthesisAudit,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActivityRecapSynthesisAudit {
    pub model_pack: Value,
    pub openai_request: Value,
    pub raw_response: Value,
    pub parsed_output: Value,
    pub validation: Value,
    pub fallback: Value,
}

impl Default for ActivityRecapSynthesisAudit {
    fn default() -> Self {
        Self {
            model_pack: json!({"status": "not_built"}),
            openai_request: json!({"status": "not_requested"}),
            raw_response: json!({"status": "not_received"}),
            parsed_output: json!({"status": "not_parsed"}),
            validation: json!({
                "schema": "smalltalk.activity_recap_model_validation.v2",
                "outcome": "not_attempted",
                "failures": [],
                "repairs": []
            }),
            fallback: json!({
                "schema": "smalltalk.activity_recap_model_fallback.v1",
                "used": false,
                "reason": null
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelPack {
    pub schema: String,
    pub instructions: String,
    pub task_truth: ActivityRecapTaskTruth,
    pub current_surface: Option<ActivityRecapModelSurface>,
    pub primary_segment: Option<ActivityRecapModelPrimarySegment>,
    pub detours: Vec<ActivityRecapModelDetour>,
    pub supporting_context: Vec<ActivityRecapModelSupport>,
    pub local_seed: ActivityRecapModelLocalSeed,
    pub objective_terms: Vec<String>,
    pub safe_next_action_candidates: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub target_policy: ActivityRecapModelTargetPolicy,
    pub evidence_handles: Vec<ActivityRecapModelEvidenceHandle>,
    pub allowed_primary_terms: Vec<String>,
    pub allowed_where_terms: Vec<String>,
    pub allowed_state_terms: Vec<String>,
    pub allowed_next_action_terms: Vec<String>,
    pub allowed_target_terms: Vec<String>,
    #[serde(skip, default)]
    pub local_guard: ActivityRecapLocalGuard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelSurface {
    pub app_name: Option<String>,
    pub display_title: Option<String>,
    pub activity_state: Option<String>,
    pub task_state: Option<String>,
    pub evidence_quality: String,
    pub openability: String,
    pub claim_eligible: bool,
    pub evidence_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelPrimarySegment {
    pub app_name: Option<String>,
    pub surface_title: Option<String>,
    pub role: String,
    pub activity_kinds: Vec<String>,
    pub confidence: ActivityEvidenceConfidence,
    pub evidence_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelDetour {
    pub source_detour_id: String,
    pub surface_title: Option<String>,
    pub app_name: Option<String>,
    pub role: ActivityDetourRole,
    pub activity_label: Option<String>,
    pub local_reason: String,
    pub confidence: ActivityEvidenceConfidence,
    pub evidence_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelSupport {
    pub source_support_id: String,
    pub local_summary: String,
    pub role: String,
    pub confidence: ActivityEvidenceConfidence,
    pub evidence_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelLocalSeed {
    pub primary_work_summary: Option<String>,
    pub primary_work_label: Option<String>,
    pub primary_where_summary: Option<String>,
    pub current_state: ActivityCurrentState,
    pub last_meaningful_state: Option<String>,
    pub unfinished_state: Option<String>,
    pub next_action_summary: Option<String>,
    pub why_this_target: Option<String>,
    pub why_no_safe_target: Option<String>,
    pub activity_confidence: ActivityConfidence,
    pub target_confidence: ActivityConfidence,
    pub validation_status: ActivityRecapValidationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelTargetPolicy {
    pub has_safe_target: bool,
    pub openability: String,
    pub may_explain_target: bool,
    pub must_explain_no_safe_target: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelEvidenceHandle {
    pub handle: String,
    pub role: String,
    pub confidence: ActivityEvidenceConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelOutput {
    pub identity: ActivityRecapTaskIdentity,
    pub target_policy: ActivityRecapModelTargetPolicy,
    pub primary_work_summary: Option<String>,
    pub primary_where_summary: Option<String>,
    pub last_meaningful_state: Option<String>,
    pub unfinished_state: Option<String>,
    pub next_action_summary: Option<String>,
    pub why_this_target: Option<String>,
    pub why_no_safe_target: Option<String>,
    pub detour_summaries: Vec<ActivityRecapModelDetourOutput>,
    pub confidence: String,
    pub uncertainty_notes: Vec<String>,
    pub used_evidence_handles: Vec<String>,
    pub claim_proofs: Vec<ActivityRecapModelClaimProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelClaimProof {
    pub claim_key: String,
    pub evidence_handles: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivityRecapModelDetourOutput {
    pub source_detour_id: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
struct ActivityRecapOpenAiConfig {
    api_key: Option<String>,
    model: String,
}

pub(crate) fn maybe_run_activity_recap_model_synthesis(
    context: ActivityRecapSynthesisContext<'_>,
) -> ActivityRecapSynthesisResult {
    let pack = build_activity_recap_model_pack(
        context.inputs,
        context.timeline,
        context.work_label,
        context.local_recap,
        context.task_truth,
        context.local_guard,
    );
    let mut audit = ActivityRecapSynthesisAudit {
        model_pack: serde_json::to_value(&pack)
            .unwrap_or_else(|_| json!({"status": "serialization_failed"})),
        ..ActivityRecapSynthesisAudit::default()
    };

    if !context.enabled {
        audit.fallback = fallback_audit(false, Some("disabled"));
        return ActivityRecapSynthesisResult {
            recap: context.local_recap.clone(),
            audit,
        };
    }
    if pack.evidence_handles.is_empty() || !pack_has_rewritable_local_claim(&pack) {
        return fallback_result(
            context.local_recap,
            audit,
            "insufficient_bounded_facts",
            ActivityRecapValidationStatus::Fallback,
        );
    }

    let config = match activity_recap_openai_config(context.model_override) {
        Ok(config) => config,
        Err(_) => {
            return fallback_result(
                context.local_recap,
                audit,
                "configuration_unavailable",
                ActivityRecapValidationStatus::Fallback,
            )
        }
    };
    let Some(api_key) = config.api_key.as_deref() else {
        return fallback_result(
            context.local_recap,
            audit,
            "model_unavailable",
            ActivityRecapValidationStatus::Fallback,
        );
    };
    let request = match build_activity_recap_openai_request(&config.model, &pack) {
        Ok(request) => request,
        Err(_) => {
            return fallback_result(
                context.local_recap,
                audit,
                "request_build_failed",
                ActivityRecapValidationStatus::Fallback,
            )
        }
    };
    audit.openai_request = if context.retain_audit_payloads {
        request.clone()
    } else {
        json!({"status": "not_retained", "reason": "explicit_audit_disabled"})
    };

    let response = match super::call_openai_responses_with_timeout(api_key, &request, 25, 0) {
        Ok(response) => response,
        Err(error) => {
            return fallback_result(
                context.local_recap,
                audit,
                classify_transport_failure(&error),
                ActivityRecapValidationStatus::Fallback,
            )
        }
    };
    finish_activity_recap_synthesis(
        context.local_recap,
        &pack,
        response,
        audit,
        context.retain_audit_payloads,
    )
}

fn pack_has_rewritable_local_claim(pack: &ActivityRecapModelPack) -> bool {
    [
        pack.local_seed.primary_work_summary.as_ref(),
        pack.local_seed.primary_where_summary.as_ref(),
        pack.local_seed.last_meaningful_state.as_ref(),
        pack.local_seed.unfinished_state.as_ref(),
        pack.local_seed.next_action_summary.as_ref(),
        pack.local_seed.why_this_target.as_ref(),
        pack.local_seed.why_no_safe_target.as_ref(),
    ]
    .into_iter()
    .any(|value| value.is_some())
        || !pack.detours.is_empty()
}

fn finish_activity_recap_synthesis(
    local_recap: &ContinueActivityRecap,
    pack: &ActivityRecapModelPack,
    response: Value,
    mut audit: ActivityRecapSynthesisAudit,
    retain_audit_payloads: bool,
) -> ActivityRecapSynthesisResult {
    audit.raw_response = if retain_audit_payloads {
        response.clone()
    } else {
        json!({"status": "not_retained", "reason": "explicit_audit_disabled"})
    };
    let output = match parse_activity_recap_model_response(&response) {
        Ok(output) => output,
        Err(_) => {
            return fallback_result(
                local_recap,
                audit,
                "invalid_json",
                ActivityRecapValidationStatus::Rejected,
            )
        }
    };
    audit.parsed_output = if retain_audit_payloads {
        serde_json::to_value(&output).unwrap_or_else(|_| json!({"status": "serialization_failed"}))
    } else {
        json!({"status": "not_retained", "reason": "explicit_audit_disabled"})
    };

    let validation = validate_activity_recap_model_output(local_recap, pack, &output);
    audit.validation = serde_json::to_value(&validation.report)
        .unwrap_or_else(|_| json!({"outcome": "audit_serialization_failed"}));
    if validation.report.outcome.starts_with("rejected_") {
        audit.fallback = fallback_audit(true, Some("validation_rejected"));
    } else {
        audit.fallback = fallback_audit(false, None);
    }
    ActivityRecapSynthesisResult {
        recap: validation.recap,
        audit,
    }
}

fn fallback_result(
    local_recap: &ContinueActivityRecap,
    mut audit: ActivityRecapSynthesisAudit,
    reason: &str,
    validation_status: ActivityRecapValidationStatus,
) -> ActivityRecapSynthesisResult {
    let mut recap = local_recap.clone();
    recap.generated_by = ActivityRecapGeneratedBy::Fallback;
    recap.validation_status = validation_status;
    push_unique(
        &mut recap.warnings,
        format!("activity_recap_model_fallback:{reason}"),
    );
    audit.validation = json!({
        "schema": "smalltalk.activity_recap_model_validation.v2",
        "outcome": "fallback_local",
        "failures": [reason],
        "repairs": []
    });
    audit.fallback = fallback_audit(true, Some(reason));
    ActivityRecapSynthesisResult { recap, audit }
}

fn fallback_audit(used: bool, reason: Option<&str>) -> Value {
    json!({
        "schema": "smalltalk.activity_recap_model_fallback.v1",
        "used": used,
        "reason": reason
    })
}

pub(crate) fn build_activity_recap_model_pack(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
    work_label: &ActivityWorkLabelResult,
    local_recap: &ContinueActivityRecap,
    task_truth: &ActivityRecapTaskTruth,
    local_guard: &ActivityRecapLocalGuard,
) -> ActivityRecapModelPack {
    let recap = local_recap.clone().sanitized();
    let eligible_primary_segment = timeline.primary_segment.as_ref().filter(|segment| {
        task_truth.consistency_status != "conflicting"
            && task_truth.selected_primary_segment_id.as_deref()
                == Some(segment.segment_id.as_str())
    });
    let primary_anchor_ids = eligible_primary_segment
        .map(|segment| segment.evidence_anchor_ids.as_slice())
        .unwrap_or(&[]);
    let primary_handle = (!primary_anchor_ids.is_empty()).then(|| "e1".to_string());
    let current_surface_handle = inputs
        .current_surface
        .as_ref()
        .filter(|surface| surface.claim_eligible && !surface.evidence_ids.is_empty())
        .map(|_| "e2".to_string());

    let current_surface =
        inputs
            .current_surface
            .as_ref()
            .map(|surface| ActivityRecapModelSurface {
                app_name: clean_optional(surface.app_name.as_deref(), 80),
                display_title: clean_optional(surface.display_title.as_deref(), 160),
                activity_state: clean_optional(surface.activity_state.as_deref(), 100),
                task_state: clean_optional(surface.task_state.as_deref(), 100),
                evidence_quality: safe_enum_text(&surface.evidence_quality, "unknown"),
                openability: safe_enum_text(&surface.openability, "unknown"),
                claim_eligible: surface.claim_eligible,
                evidence_handle: current_surface_handle.clone(),
            });
    let primary_segment =
        eligible_primary_segment.map(|segment| ActivityRecapModelPrimarySegment {
            app_name: clean_optional(segment.app_name.as_deref(), 80),
            surface_title: clean_optional(segment.surface_title.as_deref(), 160),
            role: enum_name(&segment.role),
            activity_kinds: clean_list(segment.activity_kinds.iter().map(String::as_str), 8, 100),
            confidence: segment.confidence,
            evidence_handle: primary_handle.clone(),
        });

    let mut evidence_handles = Vec::new();
    if let Some(handle) = primary_handle.as_deref() {
        evidence_handles.push(ActivityRecapModelEvidenceHandle {
            handle: handle.to_string(),
            role: "primary_activity".to_string(),
            confidence: timeline
                .primary_segment
                .as_ref()
                .map(|segment| segment.confidence)
                .unwrap_or(ActivityEvidenceConfidence::Low),
        });
    }
    if let Some(handle) = current_surface_handle.as_deref() {
        evidence_handles.push(ActivityRecapModelEvidenceHandle {
            handle: handle.to_string(),
            role: "current_surface".to_string(),
            confidence: confidence_from_quality(
                inputs
                    .current_surface
                    .as_ref()
                    .map(|surface| surface.evidence_quality.as_str()),
            ),
        });
    }
    for handles in task_truth.claim_evidence_handles.values() {
        for handle in handles {
            if !evidence_handles.iter().any(|item| item.handle == *handle) {
                evidence_handles.push(ActivityRecapModelEvidenceHandle {
                    handle: handle.clone(),
                    role: "current_task_truth".to_string(),
                    confidence: confidence_from_score(
                        task_truth
                            .claim_confidence_caps
                            .get("primary_work_summary")
                            .copied()
                            .unwrap_or(0.0),
                    ),
                });
            }
        }
    }

    let mut detours = Vec::new();
    for (index, detour) in recap.recent_detours.iter().take(3).enumerate() {
        let id = format!("detour_{}", index + 1);
        let evidence_handle = format!("d{}", index + 1);
        evidence_handles.push(ActivityRecapModelEvidenceHandle {
            handle: evidence_handle.clone(),
            role: format!("detour:{id}"),
            confidence: detour.confidence,
        });
        detours.push(ActivityRecapModelDetour {
            source_detour_id: id,
            surface_title: detour.surface_title.clone(),
            app_name: detour.app_name.clone(),
            role: detour.role,
            activity_label: detour.activity_label.clone(),
            local_reason: detour.reason.clone(),
            confidence: detour.confidence,
            evidence_handle,
        });
    }
    let mut supporting_context = Vec::new();
    let remaining_support_slots = 3usize.saturating_sub(detours.len());
    for (index, support) in recap
        .supporting_context
        .iter()
        .take(remaining_support_slots)
        .enumerate()
    {
        let id = format!("support_{}", index + 1);
        let evidence_handle = format!("s{}", index + 1);
        evidence_handles.push(ActivityRecapModelEvidenceHandle {
            handle: evidence_handle.clone(),
            role: format!("support:{id}"),
            confidence: support.confidence,
        });
        supporting_context.push(ActivityRecapModelSupport {
            source_support_id: id,
            local_summary: support.summary.clone(),
            role: enum_name(&support.role),
            confidence: support.confidence,
            evidence_handle,
        });
    }

    let target = inputs
        .return_target
        .as_ref()
        .or(inputs.resume_work_target.as_ref());
    let has_safe_target = task_truth.direct_target_allowed;
    let target_openability = if has_safe_target {
        target
            .map(|target| safe_enum_text(&target.openability, "unknown"))
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "none_or_thin".to_string()
    };

    let mut objective_terms = clean_list(
        task_truth
            .latest_user_goal
            .iter()
            .chain(task_truth.task_object.iter())
            .map(String::as_str),
        MAX_OBJECTIVE_TERMS,
        100,
    );
    push_optional_clean(
        &mut objective_terms,
        work_label.object_label.as_deref(),
        100,
    );
    let safe_next_action_candidates = recap
        .next_action_summary
        .as_deref()
        .and_then(|value| sanitize_public_text(value.to_string(), 240))
        .into_iter()
        .collect::<Vec<_>>();
    let missing_evidence = clean_list(
        recap.missing_evidence.iter().map(String::as_str),
        MAX_MISSING_EVIDENCE,
        180,
    );

    let mut allowed_primary_sources = vec![
        recap.primary_work_summary.as_deref(),
        recap.primary_work_label.as_deref(),
        task_truth.latest_user_goal.as_deref(),
        task_truth.task_object.as_deref(),
        task_truth.identity.bounded_semantic_label.as_deref(),
    ];
    allowed_primary_sources.extend(objective_terms.iter().map(|value| Some(value.as_str())));
    if let Some(primary) = primary_segment.as_ref().filter(|_| {
        timeline.primary_segment.as_ref().is_some_and(|segment| {
            task_truth.selected_primary_segment_id.as_deref() == Some(segment.segment_id.as_str())
        })
    }) {
        allowed_primary_sources.push(primary.app_name.as_deref());
        allowed_primary_sources.push(primary.surface_title.as_deref());
        allowed_primary_sources.extend(
            primary
                .activity_kinds
                .iter()
                .map(|value| Some(value.as_str())),
        );
    }
    let allowed_primary_terms = term_bank(allowed_primary_sources);
    let allowed_where_terms = term_bank(vec![
        recap.primary_where_summary.as_deref(),
        work_label.where_label.as_deref(),
    ]);
    let allowed_state_terms = term_bank(vec![
        recap.last_meaningful_state.as_deref(),
        recap.unfinished_state.as_deref(),
    ]);
    let allowed_next_action_terms = term_bank(vec![recap.next_action_summary.as_deref()]);
    let allowed_target_terms = term_bank(vec![
        recap.why_this_target.as_deref(),
        recap.why_no_safe_target.as_deref(),
    ]);

    ActivityRecapModelPack {
        schema: ACTIVITY_RECAP_MODEL_PACK_SCHEMA.to_string(),
        instructions: "Rewrite only supplied recap phrasing. Copy identity and target_policy exactly. Every material claim needs an allowed evidence handle and confidence at or below its local cap. Do not select or open targets, promote detours, infer new tasks, alter lifecycle state, or add facts. Use opaque handles only in proof fields and never in user copy. Preserve uncertainty when evidence is thin.".to_string(),
        task_truth: task_truth.clone(),
        current_surface,
        primary_segment,
        detours,
        supporting_context,
        local_seed: ActivityRecapModelLocalSeed {
            primary_work_summary: recap.primary_work_summary.clone(),
            primary_work_label: recap.primary_work_label.clone(),
            primary_where_summary: recap.primary_where_summary.clone(),
            current_state: recap.current_state,
            last_meaningful_state: recap.last_meaningful_state.clone(),
            unfinished_state: recap.unfinished_state.clone(),
            next_action_summary: recap.next_action_summary.clone(),
            why_this_target: recap.why_this_target.clone(),
            why_no_safe_target: recap.why_no_safe_target.clone(),
            activity_confidence: recap.activity_confidence,
            target_confidence: recap.target_confidence,
            validation_status: recap.validation_status,
        },
        objective_terms,
        safe_next_action_candidates,
        missing_evidence,
        target_policy: ActivityRecapModelTargetPolicy {
            has_safe_target,
            openability: target_openability,
            may_explain_target: has_safe_target,
            must_explain_no_safe_target: !has_safe_target,
        },
        evidence_handles,
        allowed_primary_terms,
        allowed_where_terms,
        allowed_state_terms,
        allowed_next_action_terms,
        allowed_target_terms,
        local_guard: local_guard.clone(),
    }
}

fn activity_recap_openai_config(
    model_override: Option<&str>,
) -> Result<ActivityRecapOpenAiConfig, String> {
    let project_env = super::project_dotenv_values()?;
    let base = super::continue_openai_config(None)?;
    let process_model = std::env::var("SMALLTALK_ACTIVITY_RECAP_MODEL").ok();
    let project_model = project_env.get("SMALLTALK_ACTIVITY_RECAP_MODEL").cloned();
    let model = select_activity_recap_model(
        model_override,
        process_model.as_deref(),
        project_model.as_deref(),
        &base.model,
    );
    Ok(ActivityRecapOpenAiConfig {
        api_key: base.api_key,
        model,
    })
}

pub(crate) fn effective_activity_recap_model(
    model_override: Option<&str>,
) -> Result<String, String> {
    activity_recap_openai_config(model_override).map(|config| config.model)
}

fn select_activity_recap_model(
    model_override: Option<&str>,
    process_activity_model: Option<&str>,
    project_activity_model: Option<&str>,
    base_model: &str,
) -> String {
    model_override
        .and_then(non_empty_ref)
        .or_else(|| process_activity_model.and_then(non_empty_ref))
        .or_else(|| project_activity_model.and_then(non_empty_ref))
        .unwrap_or_else(|| base_model.to_string())
}

fn build_activity_recap_openai_request(
    model: &str,
    pack: &ActivityRecapModelPack,
) -> Result<Value, String> {
    Ok(json!({
        "model": model,
        "store": crate::continuation::openai_response_storage_enabled(),
        "max_output_tokens": 750,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "smalltalk_activity_recap_synthesis",
                "strict": true,
                "schema": activity_recap_model_schema()
            }
        },
        "input": [
            {
                "role": "system",
                "content": [{
                    "type": "input_text",
                    "text": "You phrase a local-first Smalltalk activity recap from the supplied compact fact pack only. Never invent tasks, objects, actions, URLs, paths, ids, targets, or confidence. Never promote a detour or support branch. A null field means no supported rewrite. When evidence is thin, use low confidence and include an uncertainty note. Return JSON only."
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": serde_json::to_string(pack).map_err(super::to_string)?
                }]
            }
        ]
    }))
}

fn activity_recap_model_schema() -> Value {
    let nullable_string = || json!({"type": ["string", "null"]});
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "identity",
            "target_policy",
            "primary_work_summary",
            "primary_where_summary",
            "last_meaningful_state",
            "unfinished_state",
            "next_action_summary",
            "why_this_target",
            "why_no_safe_target",
            "detour_summaries",
            "confidence",
            "uncertainty_notes",
            "used_evidence_handles",
            "claim_proofs"
        ],
        "properties": {
            "identity": {
                "type": "object",
                "additionalProperties": false,
                "required": ["task_turn_id", "task_turn_revision", "task_identity_key", "bounded_semantic_label", "execution_state", "current_actor", "waiting_on", "relation_to_prior", "workstream_id"],
                "properties": {
                    "task_turn_id": {"type": "string"},
                    "task_turn_revision": {"type": "integer"},
                    "task_identity_key": {"type": "string"},
                    "bounded_semantic_label": nullable_string(),
                    "execution_state": {"type": "string"},
                    "current_actor": {"type": "string"},
                    "waiting_on": {"type": "string"},
                    "relation_to_prior": {"type": "string"},
                    "workstream_id": nullable_string()
                }
            },
            "target_policy": {
                "type": "object",
                "additionalProperties": false,
                "required": ["has_safe_target", "openability", "may_explain_target", "must_explain_no_safe_target"],
                "properties": {
                    "has_safe_target": {"type": "boolean"},
                    "openability": {"type": "string"},
                    "may_explain_target": {"type": "boolean"},
                    "must_explain_no_safe_target": {"type": "boolean"}
                }
            },
            "primary_work_summary": nullable_string(),
            "primary_where_summary": nullable_string(),
            "last_meaningful_state": nullable_string(),
            "unfinished_state": nullable_string(),
            "next_action_summary": nullable_string(),
            "why_this_target": nullable_string(),
            "why_no_safe_target": nullable_string(),
            "detour_summaries": {
                "type": "array",
                "maxItems": 3,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["source_detour_id", "summary"],
                    "properties": {
                        "source_detour_id": {"type": "string"},
                        "summary": {"type": "string"}
                    }
                }
            },
            "confidence": {"type": "string", "enum": ["low", "medium", "high"]},
            "uncertainty_notes": {
                "type": "array",
                "maxItems": 4,
                "items": {"type": "string"}
            },
            "used_evidence_handles": {
                "type": "array",
                "maxItems": 8,
                "items": {"type": "string"}
            },
            "claim_proofs": {
                "type": "array",
                "maxItems": 10,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["claim_key", "evidence_handles", "confidence"],
                    "properties": {
                        "claim_key": {"type": "string"},
                        "evidence_handles": {"type": "array", "maxItems": 8, "items": {"type": "string"}},
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1}
                    }
                }
            }
        }
    })
}

fn parse_activity_recap_model_response(
    response: &Value,
) -> Result<ActivityRecapModelOutput, String> {
    if response.get("status").and_then(Value::as_str) == Some("incomplete") {
        return Err("incomplete_response".to_string());
    }
    let text = response
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| extract_response_content(response))
        .ok_or_else(|| "missing_output".to_string())?;
    serde_json::from_str(&text).map_err(|_| "invalid_json".to_string())
}

fn extract_response_content(response: &Value) -> Option<String> {
    let items = response.get("output")?.as_array()?;
    let mut chunks = Vec::new();
    for item in items {
        for part in item
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if part.get("type").and_then(Value::as_str) == Some("refusal") {
                return None;
            }
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                chunks.push(text.to_string());
            } else if let Some(value) = part.get("json") {
                chunks.push(value.to_string());
            }
        }
    }
    (!chunks.is_empty()).then(|| chunks.join(""))
}

fn classify_transport_failure(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") || lower.contains("status: 28") {
        "timeout"
    } else if lower.contains("401") || lower.contains("authentication") {
        "authentication"
    } else if lower.contains("429") || lower.contains("rate limit") {
        "rate_limit"
    } else if lower.contains("resolve host")
        || lower.contains("could not connect")
        || lower.contains("network")
    {
        "network"
    } else {
        "model_unavailable"
    }
}

fn clean_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.and_then(|value| sanitize_public_text(value.to_string(), max_chars))
}

fn clean_list<'a>(
    values: impl Iterator<Item = &'a str>,
    max_items: usize,
    max_chars: usize,
) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        if output.len() >= max_items {
            break;
        }
        push_optional_clean(&mut output, Some(value), max_chars);
    }
    output
}

fn push_optional_clean(output: &mut Vec<String>, value: Option<&str>, max_chars: usize) {
    let Some(value) = clean_optional(value, max_chars) else {
        return;
    };
    if !output.contains(&value) {
        output.push(value);
    }
}

fn safe_enum_text(value: &str, fallback: &str) -> String {
    let value = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .take(64)
        .collect::<String>();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn enum_name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn confidence_from_quality(value: Option<&str>) -> ActivityEvidenceConfidence {
    match value {
        Some("strong" | "high") => ActivityEvidenceConfidence::High,
        Some("medium") => ActivityEvidenceConfidence::Medium,
        _ => ActivityEvidenceConfidence::Low,
    }
}

fn confidence_from_score(value: f64) -> ActivityEvidenceConfidence {
    if value >= 0.75 {
        ActivityEvidenceConfidence::High
    } else if value >= 0.5 {
        ActivityEvidenceConfidence::Medium
    } else {
        ActivityEvidenceConfidence::Low
    }
}

fn term_bank(values: Vec<Option<&str>>) -> Vec<String> {
    let mut terms = HashSet::new();
    for value in values.into_iter().flatten() {
        for token in normalized_tokens(value) {
            if terms.len() >= MAX_TERM_BANK {
                break;
            }
            terms.insert(token);
        }
    }
    let mut terms = terms.into_iter().collect::<Vec<_>>();
    terms.sort();
    terms
}

pub(crate) fn normalized_tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| {
            !character.is_ascii_alphanumeric() && character != '-' && character != '_'
        })
        .filter_map(|token| {
            let token = token.trim().to_ascii_lowercase();
            (token.len() > 1).then_some(token)
        })
        .collect()
}

fn non_empty_ref(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub(crate) fn synthesize_activity_recap_with_fixture_response(
    local_recap: &ContinueActivityRecap,
    pack: &ActivityRecapModelPack,
    response: Result<Value, &str>,
) -> ActivityRecapSynthesisResult {
    let audit = ActivityRecapSynthesisAudit {
        model_pack: serde_json::to_value(pack).unwrap(),
        openai_request: json!({"fixture": true}),
        ..ActivityRecapSynthesisAudit::default()
    };
    match response {
        Ok(response) => finish_activity_recap_synthesis(local_recap, pack, response, audit, true),
        Err(reason) => fallback_result(
            local_recap,
            audit,
            reason,
            ActivityRecapValidationStatus::Fallback,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_output_json() -> Value {
        json!({
            "identity": {
                "task_turn_id": "turn-current",
                "task_turn_revision": 1,
                "task_identity_key": "identity-current",
                "bounded_semantic_label": "Smalltalk recap",
                "execution_state": "active",
                "current_actor": "assistant_or_agent",
                "waiting_on": "agent",
                "relation_to_prior": "new_task",
                "workstream_id": "smalltalk"
            },
            "target_policy": {
                "has_safe_target": false,
                "openability": "none_or_thin",
                "may_explain_target": false,
                "must_explain_no_safe_target": true
            },
            "primary_work_summary": "Planning the Smalltalk recap",
            "primary_where_summary": null,
            "last_meaningful_state": null,
            "unfinished_state": null,
            "next_action_summary": null,
            "why_this_target": null,
            "why_no_safe_target": null,
            "detour_summaries": [],
            "confidence": "medium",
            "uncertainty_notes": [],
            "used_evidence_handles": ["e1"],
            "claim_proofs": [{
                "claim_key": "primary_work_summary",
                "evidence_handles": ["e1"],
                "confidence": 0.7
            }]
        })
    }

    #[test]
    fn activity_specific_model_precedes_existing_continue_model() {
        assert_eq!(
            select_activity_recap_model(
                None,
                Some("process-recap"),
                Some("project-recap"),
                "existing-continue"
            ),
            "process-recap"
        );
        assert_eq!(
            select_activity_recap_model(None, None, Some("project-recap"), "existing-continue"),
            "project-recap"
        );
        assert_eq!(
            select_activity_recap_model(
                Some("request-override"),
                Some("process-recap"),
                Some("project-recap"),
                "existing-continue"
            ),
            "request-override"
        );
        assert_eq!(
            select_activity_recap_model(None, None, None, "existing-continue"),
            "existing-continue"
        );
    }

    #[test]
    fn parser_accepts_strict_json_and_rejects_unknown_or_incomplete_output() {
        let output = valid_output_json();
        let parsed = parse_activity_recap_model_response(&json!({
            "output_text": output.to_string()
        }))
        .unwrap();
        assert_eq!(parsed.confidence, "medium");
        assert_eq!(parsed.used_evidence_handles, vec!["e1"]);

        let mut unknown = output.clone();
        unknown["invented_field"] = json!("not allowed");
        assert!(parse_activity_recap_model_response(&json!({
            "output_text": unknown.to_string()
        }))
        .is_err());
        assert!(parse_activity_recap_model_response(&json!({
            "status": "incomplete",
            "output_text": output.to_string()
        }))
        .is_err());
    }

    #[test]
    fn parser_accepts_structured_json_content_parts() {
        let parsed = parse_activity_recap_model_response(&json!({
            "output": [{
                "content": [{
                    "type": "output_json",
                    "json": valid_output_json()
                }]
            }]
        }))
        .unwrap();
        assert_eq!(
            parsed.primary_work_summary.as_deref(),
            Some("Planning the Smalltalk recap")
        );
    }
}
