use super::activity_recap::{
    model_public_text_is_safe, ActivityConfidence, ActivityDetourRole, ActivityEvidenceSource,
    ActivityRecapGeneratedBy, ActivityRecapValidationStatus, ContinueActivityRecap,
};
use super::activity_recap_model::{
    normalized_tokens, ActivityRecapModelDetour, ActivityRecapModelOutput, ActivityRecapModelPack,
};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(crate) struct ActivityRecapValidationResult {
    pub recap: ContinueActivityRecap,
    pub report: ActivityRecapValidationReport,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ActivityRecapValidationReport {
    pub schema: String,
    pub outcome: String,
    pub failures: Vec<String>,
    pub repairs: Vec<String>,
    pub accepted_fields: Vec<String>,
    pub preserved_policy_fields: Vec<String>,
}

pub(crate) fn validate_activity_recap_model_output(
    local_recap: &ContinueActivityRecap,
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
) -> ActivityRecapValidationResult {
    let mut failures = Vec::new();
    let mut repairs = Vec::new();
    let mut accepted_fields = Vec::new();

    validate_semantic_identity(pack, output, &mut failures);
    validate_cross_layer_truth(pack, &mut failures);
    validate_confidence(local_recap, output, &mut failures, &mut repairs);
    validate_uncertainty(local_recap, output, &mut failures);
    validate_evidence_handles(pack, output, &mut failures);
    validate_claim_proofs(pack, output, &mut failures, &mut repairs);
    validate_all_public_copy(output, &mut failures);
    validate_claim_slots(local_recap, output, &mut failures);
    validate_target_policy(pack, output, &mut failures);
    validate_claim_terms(pack, output, &mut failures);
    validate_forbidden_primary_terms(pack, output, &mut failures);
    validate_temporal_state(pack, output, &mut failures);
    validate_detours(pack, output, &mut failures);
    dedupe(&mut failures);
    dedupe(&mut repairs);

    if !failures.is_empty() {
        let mut recap = local_recap.clone();
        recap.generated_by = ActivityRecapGeneratedBy::Fallback;
        recap.validation_status = ActivityRecapValidationStatus::Rejected;
        push_unique(
            &mut recap.warnings,
            "activity_recap_model_validation_rejected".to_string(),
        );
        for failure in failures.iter().take(8) {
            push_unique(
                &mut recap.warnings,
                format!("activity_recap_model_validation:{failure}"),
            );
        }
        return ActivityRecapValidationResult {
            recap,
            report: report(
                rejection_outcome(&failures),
                failures,
                repairs,
                accepted_fields,
            ),
        };
    }

    let mut recap = local_recap.clone();
    apply_claim(
        &mut recap,
        "primary_work_summary",
        output.primary_work_summary.as_deref(),
        &[
            "primary_work_summary",
            "primary_work_label",
            "primary_work_object",
        ],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_claim(
        &mut recap,
        "primary_where_summary",
        output.primary_where_summary.as_deref(),
        &[
            "primary_where_summary",
            "primary_work_label",
            "primary_work_object",
        ],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_claim(
        &mut recap,
        "last_meaningful_state",
        output.last_meaningful_state.as_deref(),
        &["last_meaningful_state"],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_claim(
        &mut recap,
        "unfinished_state",
        output.unfinished_state.as_deref(),
        &["unfinished_state"],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_claim(
        &mut recap,
        "next_action_summary",
        output.next_action_summary.as_deref(),
        &["next_action_summary"],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_claim(
        &mut recap,
        "why_this_target",
        output.why_this_target.as_deref(),
        &["why_this_target"],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_claim(
        &mut recap,
        "why_no_safe_target",
        output.why_no_safe_target.as_deref(),
        &["why_no_safe_target"],
        &mut accepted_fields,
        &mut repairs,
    );
    apply_detour_summaries(&mut recap, pack, output, &mut accepted_fields);

    if accepted_fields.is_empty() {
        repairs.push("no_model_phrase_was_grounded".to_string());
        recap.generated_by = ActivityRecapGeneratedBy::Fallback;
        recap.validation_status = ActivityRecapValidationStatus::Fallback;
        push_unique(
            &mut recap.warnings,
            "activity_recap_model_fallback:no_grounded_phrase".to_string(),
        );
    } else {
        recap.generated_by = ActivityRecapGeneratedBy::ModelAssisted;
        recap.validation_status = local_recap.validation_status;
        for repair in &repairs {
            push_unique(
                &mut recap.warnings,
                format!("activity_recap_model_repaired:{repair}"),
            );
        }
    }
    dedupe(&mut accepted_fields);
    dedupe(&mut repairs);
    let outcome = if accepted_fields.is_empty() {
        "fallback_local"
    } else if repairs.is_empty() {
        "valid"
    } else {
        "repairable_copy_only"
    };
    ActivityRecapValidationResult {
        recap: recap.sanitized(),
        report: report(outcome, failures, repairs, accepted_fields),
    }
}

fn validate_semantic_identity(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    if output.identity.task_turn_id != pack.task_truth.identity.task_turn_id
        || output.identity.task_turn_revision != pack.task_truth.identity.task_turn_revision
        || output.identity.task_identity_key != pack.task_truth.identity.task_identity_key
        || output.identity.bounded_semantic_label != pack.task_truth.identity.bounded_semantic_label
    {
        failures.push("task_identity_mismatch".to_string());
    }
    if output.identity.execution_state != pack.task_truth.identity.execution_state
        || output.identity.current_actor != pack.task_truth.identity.current_actor
        || output.identity.waiting_on != pack.task_truth.identity.waiting_on
        || output.identity.relation_to_prior != pack.task_truth.identity.relation_to_prior
    {
        failures.push("task_lifecycle_identity_mismatch".to_string());
    }
    if output.identity.workstream_id != pack.task_truth.identity.workstream_id {
        failures.push("workstream_identity_mismatch".to_string());
    }
    if output.target_policy != pack.target_policy {
        failures.push("target_policy_identity_mismatch".to_string());
    }
}

fn validate_cross_layer_truth(pack: &ActivityRecapModelPack, failures: &mut Vec<String>) {
    if pack.task_truth.consistency_status == "conflicting" {
        failures.push("cross_layer_semantic_conflict".to_string());
    }
}

fn validate_claim_proofs(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
    repairs: &mut Vec<String>,
) {
    let material = [
        ("primary_work_summary", output.primary_work_summary.as_ref()),
        (
            "primary_where_summary",
            output.primary_where_summary.as_ref(),
        ),
        ("last_meaningful_state", output.last_meaning_state_ref()),
        ("unfinished_state", output.unfinished_state.as_ref()),
        ("next_action_summary", output.next_action_summary.as_ref()),
        ("why_this_target", output.why_this_target.as_ref()),
        ("why_no_safe_target", output.why_no_safe_target.as_ref()),
    ];
    let allowed_handles = pack
        .evidence_handles
        .iter()
        .map(|value| value.handle.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    for (key, value) in material {
        if value.is_none() {
            continue;
        }
        let Some(proof) = output
            .claim_proofs
            .iter()
            .find(|proof| proof.claim_key == key)
        else {
            failures.push(format!("missing_claim_proof:{key}"));
            continue;
        };
        if !seen.insert(key) {
            failures.push(format!("duplicate_claim_proof:{key}"));
        }
        if proof.evidence_handles.is_empty()
            || proof
                .evidence_handles
                .iter()
                .any(|handle| !allowed_handles.contains(handle.as_str()))
        {
            failures.push(format!("unsupported_claim_evidence:{key}"));
        }
        if proof
            .evidence_handles
            .iter()
            .any(|handle| !output.used_evidence_handles.contains(handle))
        {
            failures.push(format!("claim_evidence_not_declared_used:{key}"));
        }
        let allowed_for_claim = pack
            .task_truth
            .claim_evidence_handles
            .get(key)
            .cloned()
            .unwrap_or_default();
        if !allowed_for_claim.is_empty()
            && proof
                .evidence_handles
                .iter()
                .any(|handle| !allowed_for_claim.contains(handle))
        {
            failures.push(format!("wrong_task_turn_claim_evidence:{key}"));
        }
        let cap = pack
            .task_truth
            .claim_confidence_caps
            .get(key)
            .copied()
            .unwrap_or(0.0);
        if proof.confidence > cap + 0.000_001 {
            repairs.push(format!("claim_confidence_capped:{key}"));
        }
    }
}

trait LastMeaningfulStateRef {
    fn last_meaning_state_ref(&self) -> Option<&String>;
}

impl LastMeaningfulStateRef for ActivityRecapModelOutput {
    fn last_meaning_state_ref(&self) -> Option<&String> {
        self.last_meaningful_state.as_ref()
    }
}

fn validate_forbidden_primary_terms(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    let primary = [
        output.primary_work_summary.as_deref(),
        output.last_meaningful_state.as_deref(),
        output.unfinished_state.as_deref(),
        output.next_action_summary.as_deref(),
    ]
    .into_iter()
    .flatten()
    .flat_map(normalized_tokens)
    .collect::<HashSet<_>>();
    if pack
        .local_guard
        .forbidden_primary_terms
        .iter()
        .any(|term| primary.contains(term))
    {
        failures.push("ineligible_source_primary_term_leak".to_string());
    }
}

fn validate_temporal_state(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    let state = pack.task_truth.identity.execution_state.as_str();
    let primary_text = [
        output.primary_work_summary.as_deref(),
        output.unfinished_state.as_deref(),
        output.next_action_summary.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    if state == "active"
        && [" completed", " finished", " done", " was complete"]
            .iter()
            .any(|term| primary_text.contains(term))
    {
        failures.push("prior_completion_applied_to_current_task".to_string());
    }
    if state == "completed" && output.unfinished_state.is_some() {
        failures.push("completed_task_described_as_unfinished".to_string());
    }
    let waiting_on = pack.task_truth.identity.waiting_on.as_str();
    let next = output
        .next_action_summary
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if waiting_on == "agent" && (next.contains("start working") || next.contains("provide input")) {
        failures.push("next_action_incompatible_with_waiting_on_agent".to_string());
    }
    if waiting_on == "user" && next.contains("wait for the agent") {
        failures.push("next_action_incompatible_with_waiting_on_user".to_string());
    }
}

fn rejection_outcome(failures: &[String]) -> &'static str {
    if failures.iter().any(|value| value.contains("target")) {
        "rejected_target_policy"
    } else if failures
        .iter()
        .any(|value| value.contains("workstream") || value.contains("cross_layer"))
    {
        "rejected_workstream_conflict"
    } else if failures.iter().any(|value| {
        value.contains("completion") || value.contains("temporal") || value.contains("lifecycle")
    }) {
        "rejected_temporal_conflict"
    } else if failures
        .iter()
        .any(|value| value.contains("ineligible_source"))
    {
        "rejected_ineligible_source"
    } else {
        "rejected_unsupported_claim"
    }
}

fn validate_confidence(
    local_recap: &ContinueActivityRecap,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
    repairs: &mut Vec<String>,
) {
    let model_rank = match output.confidence.as_str() {
        "low" => 1,
        "medium" => 2,
        "high" => 3,
        _ => {
            failures.push("unknown_confidence".to_string());
            return;
        }
    };
    let local_rank = confidence_rank(local_recap.activity_confidence)
        .min(confidence_rank(local_recap.target_confidence).max(1));
    let thin = local_recap.validation_status != ActivityRecapValidationStatus::Valid
        || !local_recap.missing_evidence.is_empty()
        || local_rank <= 1;
    if thin && model_rank == 3 {
        if output.uncertainty_notes.is_empty() {
            failures.push("high_confidence_on_thin_evidence".to_string());
        } else {
            repairs.push("model_confidence_capped_to_local_evidence".to_string());
        }
    } else if model_rank > local_rank.max(1) {
        repairs.push("model_confidence_capped_to_local_evidence".to_string());
    }
}

fn validate_uncertainty(
    local_recap: &ContinueActivityRecap,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    let uncertainty_required = local_recap.validation_status
        != ActivityRecapValidationStatus::Valid
        || !local_recap.missing_evidence.is_empty()
        || matches!(
            local_recap.activity_confidence,
            ActivityConfidence::None | ActivityConfidence::Low
        );
    if uncertainty_required && output.uncertainty_notes.is_empty() {
        failures.push("required_uncertainty_missing".to_string());
    }
    for note in &output.uncertainty_notes {
        if !model_public_text_is_safe(note, 180) {
            failures.push("unsafe_uncertainty_note".to_string());
        }
    }
}

fn validate_evidence_handles(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    let allowed = pack
        .evidence_handles
        .iter()
        .map(|item| item.handle.as_str())
        .collect::<HashSet<_>>();
    let produces_copy = output_texts(output)
        .iter()
        .any(|value| !value.trim().is_empty())
        || !output.detour_summaries.is_empty();
    if produces_copy && output.used_evidence_handles.is_empty() {
        failures.push("used_evidence_handles_missing".to_string());
    }
    for handle in &output.used_evidence_handles {
        if !allowed.contains(handle.as_str()) {
            failures.push("unknown_evidence_handle".to_string());
        }
    }
    let mut seen = HashSet::new();
    if output
        .used_evidence_handles
        .iter()
        .any(|handle| !seen.insert(handle))
    {
        failures.push("duplicate_evidence_handle".to_string());
    }
}

fn validate_all_public_copy(output: &ActivityRecapModelOutput, failures: &mut Vec<String>) {
    for value in output_texts(output) {
        if !model_public_text_is_safe(value, 280) {
            failures.push("raw_locator_or_internal_id_leak".to_string());
        }
        if contains_opaque_handle(value) {
            failures.push("opaque_handle_in_user_copy".to_string());
        }
    }
    for detour in &output.detour_summaries {
        if !model_public_text_is_safe(&detour.summary, 220) {
            failures.push("raw_locator_or_internal_id_leak".to_string());
        }
        if contains_opaque_handle(&detour.summary) {
            failures.push("opaque_handle_in_user_copy".to_string());
        }
    }
}

fn validate_claim_slots(
    local_recap: &ContinueActivityRecap,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    let pairs = [
        (
            output.primary_work_summary.as_ref(),
            local_recap.primary_work_summary.as_ref(),
            "unsupported_primary_work_claim",
        ),
        (
            output.primary_where_summary.as_ref(),
            local_recap.primary_where_summary.as_ref(),
            "unsupported_where_claim",
        ),
        (
            output.last_meaningful_state.as_ref(),
            local_recap.last_meaningful_state.as_ref(),
            "unsupported_last_state_claim",
        ),
        (
            output.unfinished_state.as_ref(),
            local_recap.unfinished_state.as_ref(),
            "unsupported_unfinished_state_claim",
        ),
        (
            output.next_action_summary.as_ref(),
            local_recap.next_action_summary.as_ref(),
            "incompatible_next_action",
        ),
        (
            output.why_this_target.as_ref(),
            local_recap.why_this_target.as_ref(),
            "unsupported_target_claim",
        ),
        (
            output.why_no_safe_target.as_ref(),
            local_recap.why_no_safe_target.as_ref(),
            "unsupported_no_target_claim",
        ),
    ];
    for (model, local, failure) in pairs {
        if model.is_some() && local.is_none() {
            failures.push(failure.to_string());
        }
    }
}

fn validate_target_policy(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    if !pack.target_policy.has_safe_target {
        if output.why_this_target.is_some() {
            failures.push("target_openability_lie".to_string());
        }
        for value in [
            output.next_action_summary.as_deref(),
            output.why_no_safe_target.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if claims_openable_target(value) {
                failures.push("target_openability_lie".to_string());
            }
        }
    } else if output.why_no_safe_target.is_some() {
        failures.push("safe_target_denied_by_model".to_string());
    }
}

fn validate_claim_terms(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    validate_supported_text(
        output.primary_work_summary.as_deref(),
        &pack.allowed_primary_terms,
        "unsupported_primary_work_claim",
        failures,
    );
    validate_supported_text(
        output.primary_where_summary.as_deref(),
        &pack.allowed_where_terms,
        "unsupported_where_claim",
        failures,
    );
    let mut state_terms = pack.allowed_state_terms.clone();
    extend_terms(&mut state_terms, &pack.allowed_primary_terms);
    validate_supported_text(
        output.last_meaningful_state.as_deref(),
        &state_terms,
        "unsupported_last_state_claim",
        failures,
    );
    validate_supported_text(
        output.unfinished_state.as_deref(),
        &state_terms,
        "unsupported_unfinished_state_claim",
        failures,
    );
    let mut action_terms = pack.allowed_next_action_terms.clone();
    extend_terms(&mut action_terms, &pack.allowed_primary_terms);
    validate_supported_text(
        output.next_action_summary.as_deref(),
        &action_terms,
        "incompatible_next_action",
        failures,
    );
    let mut target_terms = pack.allowed_target_terms.clone();
    extend_terms(&mut target_terms, &pack.allowed_where_terms);
    extend_terms(&mut target_terms, &pack.allowed_primary_terms);
    validate_supported_text(
        output.why_this_target.as_deref(),
        &target_terms,
        "unsupported_target_claim",
        failures,
    );
    validate_supported_text(
        output.why_no_safe_target.as_deref(),
        &target_terms,
        "unsupported_no_target_claim",
        failures,
    );
}

fn validate_detours(
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    failures: &mut Vec<String>,
) {
    let mut seen = HashSet::new();
    for model_detour in &output.detour_summaries {
        if !seen.insert(model_detour.source_detour_id.as_str()) {
            failures.push("duplicate_detour_reference".to_string());
            continue;
        }
        let Some(local_detour) = pack
            .detours
            .iter()
            .find(|detour| detour.source_detour_id == model_detour.source_detour_id)
        else {
            failures.push("unknown_detour_reference".to_string());
            continue;
        };
        let allowed = detour_term_bank(local_detour);
        if !text_is_grounded(&model_detour.summary, &allowed) {
            failures.push("unsupported_detour_claim".to_string());
        }
        if local_detour.role != ActivityDetourRole::PromotedPrimary
            && claims_primary_or_return(&model_detour.summary)
        {
            failures.push("unpromoted_detour_claimed_as_primary".to_string());
        }
    }
    if pack.detours.iter().any(|detour| {
        detour.role != ActivityDetourRole::PromotedPrimary
            && output
                .primary_work_summary
                .as_deref()
                .is_some_and(|summary| mentions_detour_identity(summary, detour))
    }) {
        failures.push("unpromoted_detour_claimed_as_primary".to_string());
    }
}

fn apply_claim(
    recap: &mut ContinueActivityRecap,
    field: &str,
    replacement: Option<&str>,
    source_claim_keys: &[&str],
    accepted_fields: &mut Vec<String>,
    repairs: &mut Vec<String>,
) {
    let Some(replacement) = replacement.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let old_value = claim_value(recap, field).map(str::to_string);
    let Some(old_value) = old_value else {
        return;
    };
    if replacement == old_value {
        return;
    }

    let exact_index = recap
        .evidence_spans
        .iter()
        .position(|span| span.claim_text == old_value);
    let source_index = exact_index.or_else(|| {
        recap
            .evidence_spans
            .iter()
            .position(|span| source_claim_keys.contains(&span.claim_key.as_str()))
    });
    let Some(source_index) = source_index else {
        repairs.push(format!("missing_grounding_span:{field}"));
        return;
    };

    if exact_index.is_some() {
        let span = &mut recap.evidence_spans[source_index];
        span.claim_text = replacement.to_string();
        span.source = ActivityEvidenceSource::ModelValidated;
    } else {
        let mut span = recap.evidence_spans[source_index].clone();
        span.claim_key = field.to_string();
        span.claim_text = replacement.to_string();
        span.source = ActivityEvidenceSource::ModelValidated;
        recap.evidence_spans.push(span);
    }
    set_claim_value(recap, field, replacement.to_string());
    accepted_fields.push(field.to_string());
}

fn apply_detour_summaries(
    recap: &mut ContinueActivityRecap,
    pack: &ActivityRecapModelPack,
    output: &ActivityRecapModelOutput,
    accepted_fields: &mut Vec<String>,
) {
    for model_detour in &output.detour_summaries {
        let Some(pack_index) = pack
            .detours
            .iter()
            .position(|detour| detour.source_detour_id == model_detour.source_detour_id)
        else {
            continue;
        };
        let Some(detour) = recap.recent_detours.get_mut(pack_index) else {
            continue;
        };
        if detour.reason != model_detour.summary {
            detour.reason = model_detour.summary.trim().to_string();
            accepted_fields.push(format!("detour:{}", model_detour.source_detour_id));
        }
    }
}

fn claim_value<'a>(recap: &'a ContinueActivityRecap, field: &str) -> Option<&'a str> {
    match field {
        "primary_work_summary" => recap.primary_work_summary.as_deref(),
        "primary_where_summary" => recap.primary_where_summary.as_deref(),
        "last_meaningful_state" => recap.last_meaningful_state.as_deref(),
        "unfinished_state" => recap.unfinished_state.as_deref(),
        "next_action_summary" => recap.next_action_summary.as_deref(),
        "why_this_target" => recap.why_this_target.as_deref(),
        "why_no_safe_target" => recap.why_no_safe_target.as_deref(),
        _ => None,
    }
}

fn set_claim_value(recap: &mut ContinueActivityRecap, field: &str, value: String) {
    match field {
        "primary_work_summary" => recap.primary_work_summary = Some(value),
        "primary_where_summary" => recap.primary_where_summary = Some(value),
        "last_meaningful_state" => recap.last_meaningful_state = Some(value),
        "unfinished_state" => recap.unfinished_state = Some(value),
        "next_action_summary" => recap.next_action_summary = Some(value),
        "why_this_target" => recap.why_this_target = Some(value),
        "why_no_safe_target" => recap.why_no_safe_target = Some(value),
        _ => {}
    }
}

fn validate_supported_text(
    value: Option<&str>,
    allowed_terms: &[String],
    failure: &str,
    failures: &mut Vec<String>,
) {
    if value.is_some_and(|value| !text_is_grounded(value, allowed_terms)) {
        failures.push(failure.to_string());
    }
}

fn text_is_grounded(value: &str, allowed_terms: &[String]) -> bool {
    let allowed = expanded_allowed_terms(allowed_terms);
    normalized_tokens(value).into_iter().all(|token| {
        is_grammar_token(&token)
            || allowed.contains(&token)
            || allowed.iter().any(|allowed| same_stem(&token, allowed))
    })
}

fn expanded_allowed_terms(allowed_terms: &[String]) -> HashSet<String> {
    let mut allowed = allowed_terms.iter().cloned().collect::<HashSet<_>>();
    let snapshot = allowed.clone();
    for cluster in synonym_clusters() {
        if cluster.iter().any(|term| snapshot.contains(*term)) {
            allowed.extend(cluster.iter().map(|term| term.to_string()));
        }
    }
    allowed
}

fn synonym_clusters() -> &'static [&'static [&'static str]] {
    &[
        &[
            "plan",
            "planning",
            "write",
            "writing",
            "draft",
            "drafting",
            "compose",
            "composing",
        ],
        &[
            "review",
            "reviewing",
            "read",
            "reading",
            "inspect",
            "inspecting",
        ],
        &["debug", "debugging", "fix", "fixing", "error", "failure"],
        &[
            "code",
            "coding",
            "implement",
            "implementing",
            "build",
            "building",
        ],
        &[
            "research",
            "researching",
            "investigate",
            "investigating",
            "search",
            "searching",
        ],
        &["pause", "paused", "stop", "stopped", "leave", "left"],
    ]
}

fn is_grammar_token(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "at"
            | "back"
            | "because"
            | "before"
            | "but"
            | "by"
            | "can"
            | "could"
            | "did"
            | "does"
            | "during"
            | "exact"
            | "for"
            | "from"
            | "had"
            | "has"
            | "have"
            | "in"
            | "into"
            | "is"
            | "it"
            | "last"
            | "local"
            | "main"
            | "next"
            | "no"
            | "not"
            | "of"
            | "on"
            | "only"
            | "or"
            | "primary"
            | "recent"
            | "safe"
            | "state"
            | "still"
            | "that"
            | "the"
            | "their"
            | "then"
            | "there"
            | "this"
            | "to"
            | "was"
            | "were"
            | "where"
            | "while"
            | "with"
            | "work"
            | "working"
            | "you"
            | "your"
            | "visible"
            | "evidence"
            | "grounded"
            | "target"
            | "task"
            | "activity"
            | "surface"
            | "remains"
            | "looks"
            | "looked"
    )
}

fn same_stem(left: &str, right: &str) -> bool {
    stem(left) == stem(right)
}

fn stem(value: &str) -> &str {
    for suffix in ["ing", "ed", "es", "s"] {
        if value.len() > suffix.len() + 3 && value.ends_with(suffix) {
            return &value[..value.len() - suffix.len()];
        }
    }
    value
}

fn detour_term_bank(detour: &ActivityRecapModelDetour) -> Vec<String> {
    let mut values = Vec::new();
    for source in [
        detour.surface_title.as_deref(),
        detour.app_name.as_deref(),
        detour.activity_label.as_deref(),
        Some(detour.local_reason.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        extend_terms(&mut values, &normalized_tokens(source));
    }
    values
}

fn mentions_detour_identity(summary: &str, detour: &ActivityRecapModelDetour) -> bool {
    let summary_tokens = normalized_tokens(summary)
        .into_iter()
        .collect::<HashSet<_>>();
    [detour.surface_title.as_deref(), detour.app_name.as_deref()]
        .into_iter()
        .flatten()
        .flat_map(normalized_tokens)
        .filter(|token| !is_grammar_token(token))
        .any(|token| summary_tokens.contains(&token))
}

fn claims_primary_or_return(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("primary work")
        || lower.contains("main work")
        || lower.contains("return to")
        || lower.contains("resume in")
        || lower.contains("continue there")
        || lower.contains("open the")
}

fn claims_openable_target(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("open the")
        || lower.contains("open this")
        || lower.contains("return to")
        || lower.contains("resume in")
        || lower.contains("click to")
}

fn contains_opaque_handle(value: &str) -> bool {
    normalized_tokens(value).iter().any(|token| {
        token
            .strip_prefix('e')
            .or_else(|| token.strip_prefix('d'))
            .or_else(|| token.strip_prefix('s'))
            .is_some_and(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
            })
            || token
                .strip_prefix("detour_")
                .or_else(|| token.strip_prefix("support_"))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
                })
    })
}

fn output_texts(output: &ActivityRecapModelOutput) -> Vec<&str> {
    [
        output.primary_work_summary.as_deref(),
        output.primary_where_summary.as_deref(),
        output.last_meaningful_state.as_deref(),
        output.unfinished_state.as_deref(),
        output.next_action_summary.as_deref(),
        output.why_this_target.as_deref(),
        output.why_no_safe_target.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn confidence_rank(value: ActivityConfidence) -> i32 {
    match value {
        ActivityConfidence::None => 0,
        ActivityConfidence::Low => 1,
        ActivityConfidence::Medium => 2,
        ActivityConfidence::High => 3,
    }
}

fn report(
    outcome: &str,
    failures: Vec<String>,
    repairs: Vec<String>,
    accepted_fields: Vec<String>,
) -> ActivityRecapValidationReport {
    ActivityRecapValidationReport {
        schema: "smalltalk.activity_recap_model_validation.v2".to_string(),
        outcome: outcome.to_string(),
        failures,
        repairs,
        accepted_fields,
        preserved_policy_fields: vec![
            "primary_work_label".to_string(),
            "current_state".to_string(),
            "activity_confidence".to_string(),
            "target_confidence".to_string(),
            "detour_roles".to_string(),
            "evidence_anchor_ids".to_string(),
            "missing_evidence".to_string(),
            "return_target_policy".to_string(),
            "branch_promotion_policy".to_string(),
        ],
    }
}

fn extend_terms(target: &mut Vec<String>, source: &[String]) {
    for value in source {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn dedupe(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::activity_recap::{
        ActivityCurrentState, ActivityDetourSummary, ActivityEvidenceAnchorType,
        ActivityEvidenceConfidence, ActivityEvidenceSpan,
    };
    use crate::continuation::activity_recap_model::ActivityRecapModelClaimProof;
    use crate::continuation::activity_recap_model::{
        synthesize_activity_recap_with_fixture_response, ActivityRecapModelDetourOutput,
        ActivityRecapModelEvidenceHandle, ActivityRecapModelLocalSeed,
        ActivityRecapModelTargetPolicy, ACTIVITY_RECAP_MODEL_PACK_SCHEMA,
    };
    use crate::continuation::activity_recap_truth::{
        ActivityRecapLocalGuard, ActivityRecapTaskIdentity, ActivityRecapTaskTruth,
        ACTIVITY_RECAP_TASK_TRUTH_SCHEMA, ACTIVITY_RECAP_VALIDATOR_POLICY_VERSION,
    };
    use serde_json::json;

    fn local_recap(has_target: bool, thin: bool) -> ContinueActivityRecap {
        let validation_status = if thin {
            ActivityRecapValidationStatus::Thin
        } else {
            ActivityRecapValidationStatus::Valid
        };
        let activity_confidence = if thin {
            ActivityConfidence::Low
        } else {
            ActivityConfidence::Medium
        };
        let target_confidence = if has_target {
            ActivityConfidence::Medium
        } else {
            ActivityConfidence::None
        };
        let mut recap = ContinueActivityRecap {
            primary_work_summary: Some(
                "Planning the Smalltalk activity recap implementation".to_string(),
            ),
            primary_work_label: Some("Planning activity recap".to_string()),
            primary_where_summary: Some("Smalltalk project".to_string()),
            activity_confidence,
            target_confidence,
            current_state: ActivityCurrentState::ActivelyWorking,
            last_meaningful_state: Some(
                "The Smalltalk activity recap plan was left unfinished".to_string(),
            ),
            unfinished_state: Some(
                "The activity recap validation still needed implementation".to_string(),
            ),
            next_action_summary: Some(
                "Continue implementing activity recap validation".to_string(),
            ),
            why_this_target: has_target
                .then(|| "The Smalltalk project is the grounded work surface".to_string()),
            why_no_safe_target: (!has_target)
                .then(|| "Recent work is visible, but no safe target is grounded".to_string()),
            recent_detours: vec![ActivityDetourSummary {
                surface_title: Some("Photos".to_string()),
                app_name: Some("Finder".to_string()),
                role: ActivityDetourRole::Detour,
                activity_label: Some("Photo browsing".to_string()),
                reason: "Finder briefly showed photos as a browsing detour".to_string(),
                start_ms: Some(10),
                end_ms: Some(20),
                confidence: ActivityEvidenceConfidence::Medium,
                evidence_anchor_ids: vec!["frame-detour".to_string()],
            }],
            missing_evidence: if thin {
                vec!["The exact next step is thin".to_string()]
            } else {
                Vec::new()
            },
            generated_by: ActivityRecapGeneratedBy::Local,
            validation_status,
            ..ContinueActivityRecap::default()
        };
        for (key, text) in [
            (
                "primary_work_summary",
                recap.primary_work_summary.clone().unwrap(),
            ),
            (
                "primary_where_summary",
                recap.primary_where_summary.clone().unwrap(),
            ),
            (
                "last_meaningful_state",
                recap.last_meaningful_state.clone().unwrap(),
            ),
            ("unfinished_state", recap.unfinished_state.clone().unwrap()),
            (
                "next_action_summary",
                recap.next_action_summary.clone().unwrap(),
            ),
        ] {
            recap.evidence_spans.push(ActivityEvidenceSpan {
                claim_key: key.to_string(),
                claim_text: text,
                anchor_type: ActivityEvidenceAnchorType::Action,
                anchor_ids: vec!["action-primary".to_string()],
                confidence: ActivityEvidenceConfidence::Medium,
                source: ActivityEvidenceSource::Local,
            });
        }
        if let Some(text) = recap.why_this_target.clone() {
            recap.evidence_spans.push(ActivityEvidenceSpan {
                claim_key: "why_this_target".to_string(),
                claim_text: text,
                anchor_type: ActivityEvidenceAnchorType::Workstream,
                anchor_ids: vec!["workstream-primary".to_string()],
                confidence: ActivityEvidenceConfidence::Medium,
                source: ActivityEvidenceSource::Local,
            });
        }
        if let Some(text) = recap.why_no_safe_target.clone() {
            recap.evidence_spans.push(ActivityEvidenceSpan {
                claim_key: "why_no_safe_target".to_string(),
                claim_text: text,
                anchor_type: ActivityEvidenceAnchorType::Frame,
                anchor_ids: vec!["frame-primary".to_string()],
                confidence: ActivityEvidenceConfidence::Low,
                source: ActivityEvidenceSource::Local,
            });
        }
        recap
    }

    fn model_pack(local: &ContinueActivityRecap, has_target: bool) -> ActivityRecapModelPack {
        let target_policy = ActivityRecapModelTargetPolicy {
            has_safe_target: has_target,
            openability: if has_target {
                "direct".to_string()
            } else {
                "none_or_thin".to_string()
            },
            may_explain_target: has_target,
            must_explain_no_safe_target: !has_target,
        };
        let identity = ActivityRecapTaskIdentity {
            task_turn_id: "turn-current".to_string(),
            task_turn_revision: 2,
            task_identity_key: "identity-current".to_string(),
            bounded_semantic_label: Some("Smalltalk activity recap".to_string()),
            execution_state: "active".to_string(),
            current_actor: "assistant_or_agent".to_string(),
            waiting_on: "agent".to_string(),
            relation_to_prior: "new_task".to_string(),
            workstream_id: Some("workstream-smalltalk".to_string()),
        };
        let claim_confidence_caps = [
            ("primary_work_summary", 0.8),
            ("primary_where_summary", 0.8),
            ("last_meaningful_state", 0.8),
            ("unfinished_state", 0.8),
            ("next_action_summary", 0.8),
            ("why_this_target", if has_target { 0.8 } else { 0.0 }),
            ("why_no_safe_target", 0.8),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
        let claim_evidence_handles = [
            "primary_work_summary",
            "primary_where_summary",
            "last_meaningful_state",
            "unfinished_state",
            "next_action_summary",
            "why_this_target",
            "why_no_safe_target",
        ]
        .into_iter()
        .map(|key| (key.to_string(), vec!["e1".to_string()]))
        .collect();
        ActivityRecapModelPack {
            schema: ACTIVITY_RECAP_MODEL_PACK_SCHEMA.to_string(),
            instructions: "fixture".to_string(),
            task_truth: ActivityRecapTaskTruth {
                schema: ACTIVITY_RECAP_TASK_TRUTH_SCHEMA.to_string(),
                validator_policy_version: ACTIVITY_RECAP_VALIDATOR_POLICY_VERSION.to_string(),
                identity,
                latest_user_goal: Some(
                    "Write the Smalltalk activity recap implementation".to_string(),
                ),
                task_object: Some("Smalltalk activity recap".to_string()),
                prior_task_turn_id: Some("turn-prior".to_string()),
                prior_context_role: Some("prior_completed".to_string()),
                consistency_status: "consistent".to_string(),
                consistency_policy_version: "fixture".to_string(),
                selected_primary_segment_id: None,
                selected_open_loop_id: None,
                direct_target_allowed: has_target,
                direct_target_policy_version: "fixture".to_string(),
                direct_target_locator_kind: if has_target { "browser_url" } else { "none" }
                    .to_string(),
                claim_confidence_caps,
                claim_evidence_handles,
                missing_evidence: Vec::new(),
                allowed_context_roles: vec!["same_task".to_string()],
            },
            current_surface: None,
            primary_segment: None,
            detours: vec![ActivityRecapModelDetour {
                source_detour_id: "detour_1".to_string(),
                surface_title: Some("Photos".to_string()),
                app_name: Some("Finder".to_string()),
                role: ActivityDetourRole::Detour,
                activity_label: Some("Photo browsing".to_string()),
                local_reason: "Finder briefly showed photos as a browsing detour".to_string(),
                confidence: ActivityEvidenceConfidence::Medium,
                evidence_handle: "d1".to_string(),
            }],
            supporting_context: Vec::new(),
            local_seed: ActivityRecapModelLocalSeed {
                primary_work_summary: local.primary_work_summary.clone(),
                primary_work_label: local.primary_work_label.clone(),
                primary_where_summary: local.primary_where_summary.clone(),
                current_state: local.current_state,
                last_meaningful_state: local.last_meaningful_state.clone(),
                unfinished_state: local.unfinished_state.clone(),
                next_action_summary: local.next_action_summary.clone(),
                why_this_target: local.why_this_target.clone(),
                why_no_safe_target: local.why_no_safe_target.clone(),
                activity_confidence: local.activity_confidence,
                target_confidence: local.target_confidence,
                validation_status: local.validation_status,
            },
            objective_terms: vec!["Smalltalk activity recap".to_string()],
            safe_next_action_candidates: local.next_action_summary.clone().into_iter().collect(),
            missing_evidence: local.missing_evidence.clone(),
            target_policy,
            evidence_handles: vec![
                ActivityRecapModelEvidenceHandle {
                    handle: "e1".to_string(),
                    role: "primary_activity".to_string(),
                    confidence: ActivityEvidenceConfidence::Medium,
                },
                ActivityRecapModelEvidenceHandle {
                    handle: "d1".to_string(),
                    role: "detour:detour_1".to_string(),
                    confidence: ActivityEvidenceConfidence::Medium,
                },
            ],
            allowed_primary_terms: vec![
                "planning".to_string(),
                "smalltalk".to_string(),
                "activity".to_string(),
                "recap".to_string(),
                "implementation".to_string(),
            ],
            allowed_where_terms: vec!["smalltalk".to_string(), "project".to_string()],
            allowed_state_terms: vec![
                "smalltalk".to_string(),
                "activity".to_string(),
                "recap".to_string(),
                "plan".to_string(),
                "left".to_string(),
                "unfinished".to_string(),
                "validation".to_string(),
                "needed".to_string(),
                "implementation".to_string(),
            ],
            allowed_next_action_terms: vec![
                "continue".to_string(),
                "implementing".to_string(),
                "activity".to_string(),
                "recap".to_string(),
                "validation".to_string(),
            ],
            allowed_target_terms: vec![
                "smalltalk".to_string(),
                "project".to_string(),
                "grounded".to_string(),
                "work".to_string(),
                "surface".to_string(),
                "recent".to_string(),
                "visible".to_string(),
                "safe".to_string(),
                "target".to_string(),
            ],
            local_guard: ActivityRecapLocalGuard::default(),
        }
    }

    fn output() -> ActivityRecapModelOutput {
        let pack = model_pack(&local_recap(true, false), true);
        ActivityRecapModelOutput {
            identity: pack.task_truth.identity,
            target_policy: pack.target_policy,
            primary_work_summary: Some(
                "You were writing the Smalltalk activity recap implementation".to_string(),
            ),
            primary_where_summary: None,
            last_meaningful_state: None,
            unfinished_state: None,
            next_action_summary: None,
            why_this_target: None,
            why_no_safe_target: None,
            detour_summaries: Vec::new(),
            confidence: "medium".to_string(),
            uncertainty_notes: Vec::new(),
            used_evidence_handles: vec!["e1".to_string()],
            claim_proofs: vec![ActivityRecapModelClaimProof {
                claim_key: "primary_work_summary".to_string(),
                evidence_handles: vec!["e1".to_string()],
                confidence: 0.8,
            }],
        }
    }

    #[test]
    fn valid_model_wording_uses_model_assisted_copy_without_changing_local_facts() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let result = validate_activity_recap_model_output(&local, &pack, &output());

        assert_eq!(result.report.outcome, "valid");
        assert_eq!(
            result.recap.generated_by,
            ActivityRecapGeneratedBy::ModelAssisted
        );
        assert_eq!(
            result.recap.primary_work_summary.as_deref(),
            Some("You were writing the Smalltalk activity recap implementation")
        );
        assert_eq!(result.recap.current_state, local.current_state);
        assert_eq!(result.recap.target_confidence, local.target_confidence);
        assert_eq!(result.recap.missing_evidence, local.missing_evidence);
        assert_eq!(
            result.recap.recent_detours[0].role,
            local.recent_detours[0].role
        );
        assert_eq!(
            result.recap.recent_detours[0].evidence_anchor_ids,
            local.recent_detours[0].evidence_anchor_ids
        );
        let model_span = result
            .recap
            .evidence_spans
            .iter()
            .find(|span| span.claim_key == "primary_work_summary")
            .unwrap();
        assert_eq!(model_span.anchor_ids, vec!["action-primary"]);
        assert_eq!(model_span.source, ActivityEvidenceSource::ModelValidated);
    }

    #[test]
    fn invented_task_is_rejected_and_local_seed_survives() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let mut model = output();
        model.primary_work_summary =
            Some("You were deploying Kubernetes production clusters".to_string());

        let result = validate_activity_recap_model_output(&local, &pack, &model);

        assert!(result.report.outcome.starts_with("rejected_"));
        assert!(result
            .report
            .failures
            .contains(&"unsupported_primary_work_claim".to_string()));
        assert_eq!(
            result.recap.primary_work_summary,
            local.primary_work_summary
        );
        assert_eq!(result.recap.evidence_spans, local.evidence_spans);
    }

    #[test]
    fn finder_detour_cannot_replace_primary_work_without_p2_promotion() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let mut model = output();
        model.primary_work_summary =
            Some("You were browsing Finder photos as the main work".to_string());
        model.used_evidence_handles = vec!["d1".to_string()];

        let result = validate_activity_recap_model_output(&local, &pack, &model);

        assert!(result.report.outcome.starts_with("rejected_"));
        assert!(result
            .report
            .failures
            .contains(&"unpromoted_detour_claimed_as_primary".to_string()));
        assert_eq!(
            result.recap.primary_work_summary,
            local.primary_work_summary
        );
        assert_eq!(
            result.recap.recent_detours[0].role,
            ActivityDetourRole::Detour
        );
    }

    #[test]
    fn model_cannot_claim_openable_target_when_local_target_is_absent() {
        let local = local_recap(false, false);
        let pack = model_pack(&local, false);
        let mut model = output();
        model.primary_work_summary = None;
        model.why_this_target = Some("Return to the Smalltalk project".to_string());

        let result = validate_activity_recap_model_output(&local, &pack, &model);

        assert!(result.report.outcome.starts_with("rejected_"));
        assert!(result
            .report
            .failures
            .contains(&"target_openability_lie".to_string()));
        assert!(result.recap.why_this_target.is_none());
        assert_eq!(result.recap.why_no_safe_target, local.why_no_safe_target);
        assert_eq!(result.recap.target_confidence, ActivityConfidence::None);
    }

    #[test]
    fn raw_internal_id_or_path_leak_is_rejected() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let mut model = output();
        model.primary_work_summary =
            Some("Writing /Users/example/secret.rs from candidate-secret".to_string());

        let result = validate_activity_recap_model_output(&local, &pack, &model);

        assert!(result.report.outcome.starts_with("rejected_"));
        assert!(result
            .report
            .failures
            .contains(&"raw_locator_or_internal_id_leak".to_string()));
        assert_eq!(
            result.recap.primary_work_summary,
            local.primary_work_summary
        );
    }

    #[test]
    fn high_confidence_on_thin_evidence_falls_back_when_uncertainty_is_missing() {
        let local = local_recap(false, true);
        let pack = model_pack(&local, false);
        let mut model = output();
        model.confidence = "high".to_string();
        model.uncertainty_notes.clear();

        let result = validate_activity_recap_model_output(&local, &pack, &model);

        assert!(result.report.outcome.starts_with("rejected_"));
        assert!(result
            .report
            .failures
            .contains(&"high_confidence_on_thin_evidence".to_string()));
        assert_eq!(result.recap.activity_confidence, ActivityConfidence::Low);
        assert_eq!(result.recap.missing_evidence, local.missing_evidence);
    }

    #[test]
    fn model_unavailable_returns_useful_local_fallback_without_network() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let result = synthesize_activity_recap_with_fixture_response(
            &local,
            &pack,
            Err("model_unavailable"),
        );

        assert_eq!(
            result.recap.primary_work_summary,
            local.primary_work_summary
        );
        assert_eq!(result.recap.evidence_spans, local.evidence_spans);
        assert_eq!(result.recap.current_state, local.current_state);
        assert_eq!(
            result.recap.generated_by,
            ActivityRecapGeneratedBy::Fallback
        );
        assert_eq!(
            result.recap.validation_status,
            ActivityRecapValidationStatus::Fallback
        );
        assert_eq!(result.audit.fallback["reason"], json!("model_unavailable"));
    }

    #[test]
    fn invalid_json_and_timeout_use_local_fallback() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);

        let invalid = synthesize_activity_recap_with_fixture_response(
            &local,
            &pack,
            Ok(json!({"output_text": "{not valid json"})),
        );
        assert_eq!(
            invalid.recap.primary_work_summary,
            local.primary_work_summary
        );
        assert_eq!(
            invalid.recap.validation_status,
            ActivityRecapValidationStatus::Rejected
        );
        assert_eq!(invalid.audit.fallback["reason"], json!("invalid_json"));

        let timeout =
            synthesize_activity_recap_with_fixture_response(&local, &pack, Err("timeout"));
        assert_eq!(timeout.recap.evidence_spans, local.evidence_spans);
        assert_eq!(
            timeout.recap.validation_status,
            ActivityRecapValidationStatus::Fallback
        );
        assert_eq!(timeout.audit.fallback["reason"], json!("timeout"));
    }

    #[test]
    fn unknown_detour_and_incompatible_next_action_are_rejected() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let mut model = output();
        model.primary_work_summary = None;
        model.next_action_summary = Some("Deploy the Kubernetes cluster".to_string());
        model.detour_summaries = vec![ActivityRecapModelDetourOutput {
            source_detour_id: "detour_99".to_string(),
            summary: "A detour".to_string(),
        }];

        let result = validate_activity_recap_model_output(&local, &pack, &model);

        assert!(result.report.outcome.starts_with("rejected_"));
        assert!(result
            .report
            .failures
            .contains(&"unknown_detour_reference".to_string()));
        assert!(result
            .report
            .failures
            .contains(&"incompatible_next_action".to_string()));
    }

    #[test]
    fn active_current_task_cannot_inherit_prior_completion() {
        let local = local_recap(true, false);
        let mut pack = model_pack(&local, true);
        pack.allowed_primary_terms.extend([
            "completed".to_string(),
            "smalltalk".to_string(),
            "activity".to_string(),
            "recap".to_string(),
        ]);
        let mut model = output();
        model.primary_work_summary = Some("You completed the Smalltalk activity recap".to_string());
        let result = validate_activity_recap_model_output(&local, &pack, &model);
        assert_eq!(result.report.outcome, "rejected_temporal_conflict");
        assert!(result
            .report
            .failures
            .contains(&"prior_completion_applied_to_current_task".to_string()));
    }

    #[test]
    fn model_workstream_must_match_local_task_identity() {
        let local = local_recap(true, false);
        let pack = model_pack(&local, true);
        let mut model = output();
        model.identity.workstream_id = Some("workstream-helium".to_string());
        let result = validate_activity_recap_model_output(&local, &pack, &model);
        assert_eq!(result.report.outcome, "rejected_workstream_conflict");
        assert!(result
            .report
            .failures
            .contains(&"workstream_identity_mismatch".to_string()));
    }

    #[test]
    fn local_only_forbidden_terms_override_a_permissive_term_bank() {
        let local = local_recap(true, false);
        let mut pack = model_pack(&local, true);
        pack.allowed_primary_terms.extend([
            "stremio".to_string(),
            "research".to_string(),
            "smalltalk".to_string(),
        ]);
        pack.local_guard.forbidden_primary_terms = vec!["stremio".to_string()];
        let mut model = output();
        model.primary_work_summary = Some("Stremio research for Smalltalk".to_string());
        let result = validate_activity_recap_model_output(&local, &pack, &model);
        assert_eq!(result.report.outcome, "rejected_ineligible_source");
        assert!(result
            .report
            .failures
            .contains(&"ineligible_source_primary_term_leak".to_string()));
    }

    #[test]
    fn claim_confidence_is_copy_only_repair_when_identity_is_unchanged() {
        let local = local_recap(true, false);
        let mut pack = model_pack(&local, true);
        pack.task_truth
            .claim_confidence_caps
            .insert("primary_work_summary".to_string(), 0.5);
        let mut model = output();
        model.claim_proofs[0].confidence = 0.9;
        let result = validate_activity_recap_model_output(&local, &pack, &model);
        assert_eq!(result.report.outcome, "repairable_copy_only");
        assert!(result
            .report
            .repairs
            .contains(&"claim_confidence_capped:primary_work_summary".to_string()));
    }

    #[test]
    fn conflicting_local_semantic_graph_never_validates_high_confidence_copy() {
        let local = local_recap(true, false);
        let mut pack = model_pack(&local, true);
        pack.task_truth.consistency_status = "conflicting".to_string();
        let result = validate_activity_recap_model_output(&local, &pack, &output());
        assert_eq!(result.report.outcome, "rejected_workstream_conflict");
        assert!(result
            .report
            .failures
            .contains(&"cross_layer_semantic_conflict".to_string()));
    }

    #[test]
    fn normal_model_pack_never_serializes_rejected_private_terms() {
        let local = local_recap(true, false);
        let mut pack = model_pack(&local, true);
        pack.local_guard.forbidden_primary_terms = vec!["private-stale-term".to_string()];
        pack.local_guard.rejected_source_hashes = vec!["hash-only-local".to_string()];
        let serialized = serde_json::to_string(&pack).unwrap();
        assert!(!serialized.contains("private-stale-term"));
        assert!(!serialized.contains("hash-only-local"));
    }
}
