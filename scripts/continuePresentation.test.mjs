import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  authoritativeTaskTruthAnswer,
  authoritativeTaskTruthActionState,
  authoritativeTaskTruthTarget,
  buildContinuePublicProjection,
  canExecuteContinueProductAction,
  compareContinueDecisionAdoption,
  getContinuePresentationActionState,
  hasVisibleTaskTruthSemantics,
  inspectTargetCopy,
  isTaskInferenceUnavailable,
  recentContextForPresentation,
  recentContextRoleLabel,
  recentContextSurfaceLabel,
  taskInferenceFailurePresentation,
  NO_CLEAR_CURRENT_TASK_HEADLINE,
  selectPrimaryTaskHeadline,
  splitConfidenceLabels,
} from "../src/continuePresentation.ts";

function productProjection(overrides = {}) {
  return {
    answer_identity: "answer-one",
    presentation_state: "action_known_target_unavailable",
    primary_instruction: "Run the receiving-path tests.",
    resume_context: "The authoritative snapshot was selected; receiving-path verification remains.",
    location_context: "Smalltalk",
    semantic_status: "resolved",
    task_state: "in_progress",
    target_status: "task_known_target_unknown",
    primary_action: { kind: "inspect_evidence", label: "View last screen" },
    inspect_available: true,
    unresolved_reason: null,
    ...overrides,
  };
}

function adoptionDecision(overrides = {}) {
  return {
    decision_id: "decision-manual",
    task_resolution_status: "resolved_current_task",
    current_task_turn: {
      task_turn_id: "task-one",
      revision: 2,
      latest_user_goal_summary: "Repair causal evidence",
      goal_confidence: 0.9,
      last_observed_at_ms: 200,
      updated_at_ms: 200,
    },
    evidence_watermark_hash: "watermark-200",
    latest_boundary_revision: 2,
    evidence_freshness_ledger: {
      latest_any_evidence_ms: 200,
      latest_non_self_evidence_ms: 200,
    },
    confidence_summary: {
      task: { score: 0.9, label: "high", missing_evidence: [] },
      state: { score: 0.8, label: "high", missing_evidence: [] },
      target: { score: 0.8, label: "high", missing_evidence: [] },
    },
    validation_status: "validated",
    continue_output_mode: "strong_continue",
    target_truth: { state: "direct_continue_ready" },
    direct_target_policy: { direct_target_allowed: true },
    answer: {
      what_you_were_doing: "Repair causal evidence",
      where_label: "Smalltalk",
      where_you_left_off: "The failing relationship was isolated",
      next: "Implement and verify the repair",
    },
    activity_recap: {
      primary_work_summary: "Repair causal evidence",
      primary_where_summary: "Smalltalk",
      last_meaningful_state: "The failing relationship was isolated",
      next_action_summary: "Implement and verify the repair",
      generated_by: "model_assisted",
    },
    return_target: {
      artifact_id: "artifact-one",
      openability: "openable",
      document_path: "/tmp/phase.md",
    },
    wording_source: "model_assisted",
    ...overrides,
  };
}

function authoritativeDecision({
  snapshotId = "snapshot-one",
  revision = 2,
  status = "resolved",
  confidence = 0.9,
  target = null,
  overrides = {},
} = {}) {
  return adoptionDecision({
    task_truth_v2: {
      effective_state: "authoritative",
      release_gate_passed: true,
      answer: {
        task_resolution_status: status,
        target_status: target ? "direct_target_ready" : "task_known_target_unknown",
        product_projection: productProjection(target ? {
          presentation_state: "actionable_direct_target",
          target_status: "direct_target_ready",
          primary_action: { kind: "open_direct_target", label: "Continue here" },
        } : {}),
        task_summary: "Repair Task Truth adoption",
        task_object: "Continue card",
        last_meaningful_progress: "The authoritative snapshot was selected",
        unfinished_state: "The receiving paths still need verification",
        next_action: "Run the receiving-path tests",
        where_summary: "Smalltalk",
        direct_return_target: target,
        field_support: {
          task_summary: { confidence, support_status: "supported", evidence_refs: ["frame-1"] },
          task_object: { confidence, support_status: "supported", evidence_refs: ["frame-1"] },
        },
        wording_source: "deterministic",
        snapshot_id: snapshotId,
        snapshot_revision: revision,
        evidence_watermark: `watermark-${revision}`,
        selected_hypothesis_id: "hypothesis-one",
        alternative_hypotheses: [],
        atomic_identity: {
          session_id: "session-one",
          task_thread_id: "thread-one",
          task_thread_revision: revision,
          task_snapshot_id: snapshotId,
          snapshot_revision: revision,
          selected_hypothesis_id: "hypothesis-one",
          model_request_id: "request-one",
          model_response_id: "response-one",
          observation_packet_id: `packet-${revision}`,
          evidence_watermark: `watermark-${revision}`,
          correction_fingerprint: "",
        },
      },
    },
    ...overrides,
  });
}

test("direct target shows Continue here only with complete policy eligibility", () => {
  assert.deepEqual(
    getContinuePresentationActionState({
      decisionId: "decision",
      outputMode: "strong_continue",
      target: { openability: "openable", browser_url: "https://example.test/task" },
      targetTruthState: "direct_continue_ready",
      directTargetAllowed: true,
      answerAction: "continue_here",
      supportEvidenceOnly: false,
      thinCurrentWork: false,
    }),
    { kind: "openable_return_target", label: "Continue here" },
  );
});

test("authoritative task truth owns the action and target without legacy field mixing", () => {
  const authoritativeTarget = {
    artifact_id: "artifact-v2",
    openability: "openable",
    document_path: "/tmp/v2-task.md",
  };
  const decision = authoritativeDecision({
    target: authoritativeTarget,
    overrides: {
      continue_output_mode: "no_clear_continuation",
      target_truth: { state: "support_only" },
      direct_target_policy: { direct_target_allowed: false },
      answer: { action: "inspect_evidence" },
      return_target: {
        artifact_id: "artifact-legacy",
        openability: "openable",
        document_path: "/tmp/legacy-task.md",
      },
    },
  });
  assert.deepEqual(authoritativeTaskTruthActionState(decision), {
    kind: "openable_return_target",
    label: "Continue here",
  });
  assert.equal(authoritativeTaskTruthTarget(decision)?.artifact_id, "artifact-v2");
});

test("authoritative target-null cannot revive a legacy openable target", () => {
  const decision = authoritativeDecision({
    target: null,
    overrides: {
      return_target: {
        artifact_id: "artifact-legacy",
        openability: "openable",
        document_path: "/tmp/legacy-task.md",
      },
    },
  });
  assert.deepEqual(authoritativeTaskTruthActionState(decision), {
    kind: "thin_current_work",
    label: "Inspect evidence",
  });
  assert.equal(authoritativeTaskTruthTarget(decision), null);
});

test("atomic identity validation failures have a retryable validation state", () => {
  assert.deepEqual(
    taskInferenceFailurePresentation("invalid_atomic_identity", "resolved", "live_cloud", 2),
    {
      kind: "model_response_invalid",
      headline: "The model response could not be validated",
      detail: "The provider responded, but the task, snapshot, and inference identities did not form one valid decision.",
      retryable: true,
    },
  );
});

test("resolved Task Truth without complete atomic identity becomes honest unresolved", () => {
  const decision = authoritativeDecision();
  Object.assign(decision.task_truth_v2.answer, {
    admitted_semantic_status: "resolved",
    semantic_source_kind: "verified_cloud_explicit_goal",
    unfinished_task: "Verify the visual cue",
    task_state: "needs_user_verification",
    resume_point: "Implementation complete; verification remains",
    next_supported_action: "Test the cue",
    completed_context: "Checks passed",
    target_status: "direct_target_ready",
    atomic_answer_identity: "admitted-result",
    field_admission: { unfinished_task: { verdict: "accepted", reasons: [] } },
    claim_confidence: { unfinished_task: 0.9 },
  });
  decision.task_truth_v2.answer.atomic_identity.model_response_id = null;

  const answer = authoritativeTaskTruthAnswer(decision);
  assert.equal(answer?.task_resolution_status, "unresolved");
  assert.equal(answer?.admitted_semantic_status, "unresolved");
  assert.equal(answer?.semantic_source_kind, "unresolved");
  assert.equal(answer?.unfinished_task, null);
  assert.equal(answer?.task_state, "unclear");
  assert.equal(answer?.resume_point, null);
  assert.equal(answer?.next_supported_action, null);
  assert.equal(answer?.completed_context, null);
  assert.equal(answer?.target_status, "no_task");
  assert.equal(answer?.atomic_answer_identity, "");
  assert.equal(answer?.product_projection, null);
  assert.deepEqual(answer?.field_admission, {});
  assert.deepEqual(answer?.claim_confidence, {});
  assert.equal(answer?.task_summary, null);
  assert.equal(answer?.inference_status, "invalid_atomic_identity");
  assert.deepEqual(authoritativeTaskTruthActionState(decision), {
    kind: "no_clear_continuation",
    label: "Inspect evidence",
  });
  assert.equal(authoritativeTaskTruthTarget(decision), null);
});

test("startup with model-first evidence cannot fall through to legacy semantic copy", () => {
  const decision = authoritativeDecision({
    overrides: {
      request_trigger: "startup",
      return_target: {
        artifact_id: "legacy-target",
        openability: "openable",
        document_path: "/tmp/legacy.md",
      },
    },
  });
  decision.task_truth_v2.effective_state = "eligible";
  decision.task_truth_v2.release_gate_passed = false;

  const answer = authoritativeTaskTruthAnswer(decision);
  assert.equal(answer?.task_resolution_status, "unresolved");
  assert.equal(answer?.task_summary, null);
  assert.equal(answer?.inference_status, "authority_not_released");
  assert.equal(authoritativeTaskTruthTarget(decision), null);
  assert.deepEqual(authoritativeTaskTruthActionState(decision), {
    kind: "no_clear_continuation",
    label: "Inspect evidence",
  });
});

test("provider-failure unresolved answer remains usable without task identity", () => {
  const decision = authoritativeDecision({ status: "unresolved" });
  Object.assign(decision.task_truth_v2.answer, {
    task_summary: null,
    inference_status: "provider_error",
  });
  Object.assign(decision.task_truth_v2.answer.atomic_identity, {
    task_thread_id: null,
    task_thread_revision: null,
    selected_hypothesis_id: null,
    model_response_id: null,
  });

  const answer = authoritativeTaskTruthAnswer(decision);
  assert.equal(answer?.task_resolution_status, "unresolved");
  assert.equal(answer?.inference_status, "provider_error");
  assert.deepEqual(authoritativeTaskTruthActionState(decision), {
    kind: "no_clear_continuation",
    label: "Inspect evidence",
  });
});

test("startup preserves the backend projection for a typed unresolved failure", () => {
  const decision = authoritativeDecision({
    status: "unresolved",
    overrides: { request_trigger: "startup" },
  });
  decision.task_truth_v2.effective_state = "eligible";
  decision.task_truth_v2.release_gate_passed = false;
  const failure = productProjection({
    presentation_state: "task_unknown",
    primary_instruction: "I couldn’t identify the unfinished task.",
    semantic_status: "unresolved",
    task_state: "unclear",
    target_status: "no_task",
    primary_action: { kind: "refresh_continue", label: "Try Continue again" },
    unresolved_reason: "task_evidence_acquisition_failure:no_admissible_semantic_fields",
  });
  Object.assign(decision.task_truth_v2.answer, {
    task_summary: null,
    unfinished_task: null,
    target_status: "no_task",
    inference_status: "request_not_built",
    unresolved_or_failure_reason:
      "task_evidence_acquisition_failure:no_admissible_semantic_fields",
    product_projection: failure,
    stale_product_projection: { ...failure },
  });

  const answer = authoritativeTaskTruthAnswer(decision);
  assert.equal(answer?.product_projection, failure);
  assert.equal(answer?.stale_product_projection?.presentation_state, "task_unknown");
  assert.equal(
    buildContinuePublicProjection(answer)?.primary_instruction,
    "I couldn’t identify the unfinished task.",
  );
  assert.notEqual(answer?.inference_status, "authority_not_released");
});

test("a presentable backend failure replaces a stuck incumbent with no projection", () => {
  const incumbent = authoritativeDecision({ status: "unresolved" });
  incumbent.decision_id = "decision-missing-projection";
  incumbent.task_truth_v2.answer.product_projection = null;
  incumbent.task_truth_v2.answer.stale_product_projection = null;

  const challenger = authoritativeDecision({
    status: "unresolved",
    overrides: {
      decision_id: "decision-presentable-failure",
      request_trigger: "background",
      current_task_turn: null,
      evidence_freshness_ledger: {
        latest_any_evidence_ms: 100,
        latest_non_self_evidence_ms: 100,
      },
    },
  });
  challenger.task_truth_v2.answer.task_summary = null;
  challenger.task_truth_v2.answer.product_projection = productProjection({
    presentation_state: "task_unknown",
    primary_instruction: "I couldn’t identify the unfinished task.",
    semantic_status: "unresolved",
    task_state: "unclear",
    target_status: "no_task",
    primary_action: { kind: "refresh_continue", label: "Try Continue again" },
  });

  assert.deepEqual(
    compareContinueDecisionAdoption({
      incumbent,
      challenger,
      incumbentTrigger: "manual",
      challengerTrigger: "background",
    }),
    {
      adopt: true,
      reasonCodes: ["adopted:repaired_product_projection"],
    },
  );
});

test("a malformed refresh cannot replace a presentable backend answer", () => {
  const incumbent = authoritativeDecision({ status: "unresolved" });
  const challenger = authoritativeDecision({ status: "unresolved" });
  challenger.task_truth_v2.answer.product_projection = null;
  challenger.task_truth_v2.answer.stale_product_projection = null;

  assert.deepEqual(
    compareContinueDecisionAdoption({
      incumbent,
      challenger,
      incumbentTrigger: "background",
      challengerTrigger: "manual",
    }),
    {
      adopt: false,
      reasonCodes: ["rejected:missing_product_projection"],
    },
  );
});

test("recent context keeps the latest six meaningful visits for every answer state", () => {
  const visits = Array.from({ length: 10 }, (_, index) => ({
    sequence_index: index + 1,
    app_label: index === 1 ? "Private activity" : "Helium",
    site_hostname: index === 1 ? null : `site-${index}.example`,
    first_observed_at_ms: index * 100,
    last_observed_at_ms: index * 100 + 50,
    is_current: index === 9,
    revisited: index > 4,
    evidence_refs: [`frame-${index}`],
  }));

  for (const status of ["resolved", "partial", "unresolved"]) {
    const decision = authoritativeDecision({ status });
    decision.task_truth_v2.answer.recent_context = visits;
    const answer = authoritativeTaskTruthAnswer(decision);
    const visible = recentContextForPresentation(answer);
    assert.equal(visible.length, 6);
    assert.equal(visible[0].sequence_index, 5);
    assert.equal(visible.at(-1).sequence_index, 10);
  }
});

test("recent context compresses repeated shell visits without losing the latest state", () => {
  const decision = authoritativeDecision();
  decision.task_truth_v2.answer.recent_context = [
    {
      sequence_index: 1,
      app_label: "Helium",
      site_hostname: "thinkingmachines.ai",
      first_observed_at_ms: 100,
      last_observed_at_ms: 120,
      is_current: false,
      revisited: false,
      semantic_role: "detour_or_unrelated",
      relationship_to_primary_task: "Brief detour",
      evidence_refs: ["frame-1"],
    },
    {
      sequence_index: 2,
      app_label: "Helium",
      site_hostname: "thinkingmachines.ai",
      first_observed_at_ms: 130,
      last_observed_at_ms: 180,
      is_current: false,
      revisited: true,
      semantic_role: "detour_or_unrelated",
      relationship_to_primary_task: "Brief detour",
      evidence_refs: ["frame-2"],
    },
  ];

  const visible = recentContextForPresentation(authoritativeTaskTruthAnswer(decision));
  assert.equal(visible.length, 1);
  assert.equal(visible[0].sequence_index, 2);
  assert.equal(visible[0].first_observed_at_ms, 100);
  assert.deepEqual(visible[0].evidence_refs, ["frame-1", "frame-2"]);
  assert.equal(recentContextSurfaceLabel(visible[0]), "Thinking Machines");
});

test("React consumes the backend product projection without recomposing its meaning", () => {
  const decision = authoritativeDecision();
  const projection = productProjection({
    answer_identity: "answer-public",
    primary_instruction: "Finish the two manual checks.",
    resume_context: "The build and automated tests had already passed.",
    location_context: "Codex",
  });
  Object.assign(decision.task_truth_v2.answer, {
    product_projection: projection,
    unfinished_state: "This must never be promoted into an action.",
    next_action: "A legacy action that must not replace the canonical instruction.",
  });

  assert.strictEqual(
    buildContinuePublicProjection(authoritativeTaskTruthAnswer(decision)),
    projection,
  );
});

test("the four critical cases preserve their canonical visible meaning", () => {
  const cases = [
    {
      id: "05cd",
      instruction: "Continue reviewing the answer about whether the product solves a real need.",
      context: "The answer has begun and continues beyond the visible section.",
    },
    {
      id: "0d1c",
      instruction: "Return to the Codex visual-cue task and inspect its implementation result.",
      context: "The backend connection was already complete; the newer visual-cue request was still active.",
    },
    {
      id: "0056",
      instruction: "Test the new answer-linked visual cue in Smalltalk.",
      context: "Implementation passed its checks; user verification remains.",
    },
    {
      id: "0e34",
      instruction: "Return to the drafted regression report and continue the investigation.",
      context: "The latest Continue result was rejected as insufficient evidence; the cause is not yet proven.",
    },
  ];

  for (const criticalCase of cases) {
    const decision = authoritativeDecision();
    decision.task_truth_v2.answer.product_projection = productProjection({
      answer_identity: `answer-${criticalCase.id}`,
      primary_instruction: criticalCase.instruction,
      resume_context: criticalCase.context,
    });
    const projection = buildContinuePublicProjection(authoritativeTaskTruthAnswer(decision));
    assert.equal(projection.primary_instruction, criticalCase.instruction, criticalCase.id);
    assert.equal(projection.resume_context, criticalCase.context, criticalCase.id);
    assert.deepEqual(
      projection.primary_action,
      { kind: "inspect_evidence", label: "View last screen" },
      criticalCase.id,
    );
    assert.equal(projection.inspect_available, true, criticalCase.id);
  }
});

test("canonical action execution keeps direct open, preview, refresh, and none distinct", () => {
  assert.equal(
    canExecuteContinueProductAction(
      { kind: "open_direct_target", label: "Continue here" },
      false,
    ),
    false,
  );
  assert.equal(
    canExecuteContinueProductAction(
      { kind: "open_direct_target", label: "Continue here" },
      true,
    ),
    true,
  );
  assert.equal(
    canExecuteContinueProductAction(
      { kind: "inspect_evidence", label: "View last screen" },
      false,
    ),
    true,
  );
  assert.equal(
    canExecuteContinueProductAction(
      { kind: "refresh_continue", label: "Refresh Continue" },
      false,
    ),
    true,
  );
  assert.equal(
    canExecuteContinueProductAction({ kind: "none", label: "" }, true),
    false,
  );
});

test("canonical stale presentation blocks open and offers refresh", () => {
  const decision = authoritativeDecision();
  decision.task_truth_v2.answer.product_projection = productProjection({
    presentation_state: "stale_decision",
    primary_instruction: "The saved answer is older than the latest work.",
    resume_context: null,
    target_status: "stale_decision",
    primary_action: { kind: "refresh_continue", label: "Refresh Continue" },
    unresolved_reason: "newer_evidence_available",
  });
  const projection = buildContinuePublicProjection(authoritativeTaskTruthAnswer(decision));
  assert.equal(projection.primary_instruction, "The saved answer is older than the latest work.");
  assert.deepEqual(projection.primary_action, {
    kind: "refresh_continue",
    label: "Refresh Continue",
  });
  assert.notEqual(projection.primary_action.kind, "open_direct_target");
});

test("typed acquisition failure remains the canonical projection after evidence changes", () => {
  const failure = productProjection({
    presentation_state: "task_unknown",
    primary_instruction: "I couldn't identify a current task from the available evidence.",
    resume_context: "Try Continue again while the task conversation is visible.",
    location_context: null,
    semantic_status: "unresolved",
    task_state: "unknown",
    target_status: "no_task",
    primary_action: { kind: "refresh_continue", label: "Try Continue again" },
    inspect_available: true,
    unresolved_reason: "task_evidence_acquisition_failure:no_admissible_semantic_fields",
  });
  const decision = authoritativeDecision();
  decision.task_truth_v2.answer.product_projection = failure;
  decision.task_truth_v2.answer.stale_product_projection = { ...failure };

  const projection = buildContinuePublicProjection({
    ...authoritativeTaskTruthAnswer(decision),
    product_projection: decision.task_truth_v2.answer.stale_product_projection,
  });
  assert.equal(projection.presentation_state, "task_unknown");
  assert.equal(
    projection.unresolved_reason,
    "task_evidence_acquisition_failure:no_admissible_semantic_fields",
  );
  assert.notEqual(projection.primary_instruction, "The saved answer is older than the latest work.");
  assert.notEqual(projection.primary_action.kind, "open_direct_target");
});

test("canonical projection and first-screen source exclude semantic recomposition and diagnostics", () => {
  const presentationSource = readFileSync(
    new URL("../src/continuePresentation.ts", import.meta.url),
    "utf8",
  );
  const projectionHelper = presentationSource.slice(
    presentationSource.indexOf("export function buildContinuePublicProjection"),
    presentationSource.indexOf("export function canExecuteContinueProductAction"),
  );
  assert.match(projectionHelper, /answer\.product_projection/);
  assert.doesNotMatch(
    projectionHelper,
    /unfinished_state|next_action|last_meaningful_progress|task_summary|where_summary/,
  );

  const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(appSource, /cardTaskTruthAnswer\?\.stale_product_projection/);
  assert.doesNotMatch(
    appSource,
    /primary_instruction:\s*"The saved answer is older than the latest work\."/,
  );
  const freshnessComparison = appSource.slice(
    appSource.indexOf("function continueEvidenceChanged"),
    appSource.indexOf("function continueEvidenceSignature"),
  );
  assert.match(freshnessComparison, /return false/);
  assert.doesNotMatch(
    freshnessComparison,
    /frameCount\s*>|artifactCount\s*>|workstreamCount\s*>|latestTimestamp\(/,
  );
  const canonicalHero = appSource.slice(
    appSource.lastIndexOf('<div className="answer-hero answer-hero-public">'),
    appSource.indexOf("{!displayProjection && recentContext.length"),
  );
  assert.match(canonicalHero, /primary_instruction|resume_context|location_context/);
  assert.doesNotMatch(
    canonicalHero,
    /support slots|confidence|snapshot|frame_ids|recent trail|unfinished_state|next_action/i,
  );
  assert.match(canonicalHero, /aria-label="Inspect this Continue answer"/);
  assert.doesNotMatch(appSource, /answer_identity:\s*decision\?\.decision_id\s*\|\|\s*"projection-unavailable"/);
});

test("local memory failure copy is contextualized once instead of repeating the error label", () => {
  const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  const cueSource = appSource.slice(
    appSource.indexOf("const memoryCueLabel"),
    appSource.indexOf("const continuePrimaryMessage"),
  );
  const companionSource = appSource.slice(
    appSource.indexOf("function ContinueCompanionPanel"),
    appSource.indexOf("function PrivacyPanel"),
  );
  assert.match(cueSource, /: "Local memory"/);
  assert.match(companionSource, /<span>Local memory<\/span>/);
  assert.doesNotMatch(companionSource, /<span>\{statusLabel\}<\/span>/);
});

test("recent context roles use concise user-facing labels", () => {
  for (const role of ["primary_work", "continuation", "return_to_prior_task"]) {
    assert.equal(recentContextRoleLabel(role), "Primary work", role);
  }
  for (const role of ["supporting_work", "supporting_research", "verification"]) {
    assert.equal(recentContextRoleLabel(role), "Supporting work", role);
  }
  for (const role of ["detour_or_unrelated", "temporary_detour", "interruption", "new_task"]) {
    assert.equal(recentContextRoleLabel(role), "Detour or unrelated", role);
  }
  for (const role of ["unclear", "unrelated_or_unknown"]) {
    assert.equal(recentContextRoleLabel(role), "Relationship unclear", role);
  }
  assert.equal(recentContextRoleLabel(null), null);
});

test("field-limited model output remains visible instead of becoming the default state", () => {
  const decision = authoritativeDecision({ status: "partial" });
  Object.assign(decision.task_truth_v2.answer, {
    task_summary: null,
    current_subtask: "Verify the repaired Continue output",
    last_meaningful_progress: "The model response was parsed and locally admitted",
    unfinished_state: "The visible Continue card still needs confirmation",
    next_action: null,
    where_summary: null,
    direct_return_target: null,
    evidence_preview: null,
    inference_status: "model_answer_visible_with_validation_limits",
    product_projection: productProjection({
      presentation_state: "task_known_action_unknown",
      primary_instruction: "I found the task, but not a safe next step.",
      resume_context: "The model response was parsed and locally admitted.",
      location_context: null,
      semantic_status: "partial",
      task_state: "in_progress",
      target_status: "task_known_target_unknown",
      primary_action: { kind: "inspect_evidence", label: "Inspect" },
      unresolved_reason: "next_supported_action_not_admitted",
    }),
  });
  decision.task_truth_v2.answer.field_support.task_summary = {
    confidence: 0,
    support_status: "unsupported",
    evidence_refs: [],
  };

  const answer = authoritativeTaskTruthAnswer(decision);
  assert.equal(answer?.task_resolution_status, "partial");
  assert.equal(answer?.task_summary, null);
  assert.equal(answer?.current_subtask, "Verify the repaired Continue output");
  assert.equal(
    answer?.last_meaningful_progress,
    "The model response was parsed and locally admitted",
  );
  assert.equal(
    answer?.unfinished_state,
    "The visible Continue card still needs confirmation",
  );
  assert.deepEqual(authoritativeTaskTruthActionState(decision), {
    kind: "thin_current_work",
    label: "Inspect evidence",
  });
  assert.equal(hasVisibleTaskTruthSemantics(answer), true);
  assert.deepEqual(
    buildContinuePublicProjection(answer),
    decision.task_truth_v2.answer.product_projection,
  );
  assert.doesNotMatch(
    buildContinuePublicProjection(answer).primary_instruction,
    /visible Continue card still needs confirmation/i,
  );
});

test("task inference availability names only actual provider availability failures", () => {
  for (const status of ["model_unavailable", "provider_error", "provider_failure", "provider_unavailable"]) {
    assert.equal(isTaskInferenceUnavailable(status), true, status);
  }
  for (const status of ["disabled", "credentials_missing", "timeout", "request_invalid", "invalid_response"]) {
    assert.equal(isTaskInferenceUnavailable(status), false, status);
  }
  assert.equal(isTaskInferenceUnavailable("insufficient_evidence"), false);
  assert.equal(isTaskInferenceUnavailable("privacy_blocked"), false);
});

test("task inference failures have distinct user-facing states and retry policy", () => {
  assert.deepEqual(taskInferenceFailurePresentation("request_invalid"), {
    kind: "capture_unavailable",
    headline: "Capture was unavailable for this Continue attempt",
    detail: "Smalltalk could not prepare a readable current-work packet for this request.",
    retryable: false,
  });
  assert.equal(taskInferenceFailurePresentation("disabled").kind, "provider_disabled");
  assert.equal(taskInferenceFailurePresentation("credentials_missing").kind, "credentials_missing");
  assert.equal(taskInferenceFailurePresentation("model_unavailable").kind, "model_unavailable");
  assert.equal(taskInferenceFailurePresentation("timeout").kind, "provider_timeout");
  assert.equal(taskInferenceFailurePresentation("timeout").retryable, true);
  assert.equal(taskInferenceFailurePresentation("request_rejected").kind, "provider_request_rejected");
  assert.equal(
    taskInferenceFailurePresentation("request_invalid", null, "live_cloud", 1).kind,
    "provider_request_rejected",
  );
  assert.equal(
    taskInferenceFailurePresentation("request_invalid", null, "live_cloud", 0, 0).kind,
    "capture_unavailable",
  );
  assert.equal(
    taskInferenceFailurePresentation("request_invalid", null, "live_cloud", 0, 1).kind,
    "provider_request_rejected",
  );
  assert.equal(taskInferenceFailurePresentation("invalid_response").kind, "model_response_invalid");
  assert.equal(taskInferenceFailurePresentation("invalid_response").retryable, true);
  assert.deepEqual(taskInferenceFailurePresentation("provider_no_usable_output"), {
    kind: "provider_no_usable_output",
    headline: "Cloud task inference did not return a usable answer",
    detail: "The provider did not return one complete, valid task answer.",
    retryable: true,
  });
  assert.equal(
    taskInferenceFailurePresentation("structured_parse_failure").kind,
    "model_response_invalid",
  );
  assert.equal(
    taskInferenceFailurePresentation("support_slot_validation_failure").kind,
    "evidence_verifier_rejected",
  );
  assert.equal(
    taskInferenceFailurePresentation("provider_rejected").kind,
    "provider_request_rejected",
  );
  assert.equal(
    taskInferenceFailurePresentation("provider_unavailable").kind,
    "provider_unavailable",
  );
  assert.equal(
    taskInferenceFailurePresentation("success", "verification_rejected").kind,
    "evidence_verifier_rejected",
  );
  assert.equal(
    taskInferenceFailurePresentation("insufficient_evidence").kind,
    "insufficient_evidence",
  );
});

test("frame preview does not masquerade as a continuation target", () => {
  const action = getContinuePresentationActionState({
    decisionId: "decision",
    outputMode: "thin_continue",
    target: null,
    targetTruthState: "task_known_target_unknown",
    directTargetAllowed: false,
    answerAction: "inspect_evidence",
    supportEvidenceOnly: false,
    thinCurrentWork: true,
  });
  const copy = inspectTargetCopy({
    taskKnown: true,
    evidencePreviewAvailable: true,
    appFocusOnly: false,
  });
  assert.equal(action.label, "Inspect evidence");
  assert.equal(copy.targetLine, "The task is understood, but no exact return point is ready");
  assert.equal(copy.actionLabel, "Try Continue again");
  assert.doesNotMatch(JSON.stringify({ action, copy }), /Continue here|safest return point|open the work/i);
});

test("a known task without an attached target does not claim the observed page was missing", () => {
  const copy = inspectTargetCopy({
    taskKnown: true,
    evidencePreviewAvailable: true,
    appFocusOnly: false,
  });
  assert.equal(copy.targetLine, "The task is understood, but no exact return point is ready");
  assert.doesNotMatch(copy.targetLine, /no verified page|was not found/i);
});

test("task-known target-null copy stays specific about task understanding", () => {
  const copy = inspectTargetCopy({
    taskKnown: true,
    evidencePreviewAvailable: false,
    appFocusOnly: false,
  });
  assert.equal(copy.targetBlockLabel, "Exact location unavailable");
  assert.match(copy.targetMeta, /I know the task/);
});

test("current focus cannot replace the primary task", () => {
  assert.equal(
    selectPrimaryTaskHeadline(
      "Investigating the Capture button",
      "Older recap",
      "Older workstream",
      "Finder",
    ),
    "Investigating the Capture button",
  );
});

test("task and target confidence remain separate", () => {
  assert.deepEqual(splitConfidenceLabels("High", "None"), {
    task: "high",
    target: "none",
  });
});

test("support-only and stale states never expose Continue here", () => {
  for (const targetTruthState of ["support_only", "target_suppressed", "stale_decision"]) {
    const action = getContinuePresentationActionState({
      decisionId: "decision",
      outputMode: "strong_continue",
      target: { openability: "openable", document_path: "/tmp/example" },
      targetTruthState,
      directTargetAllowed: false,
      answerAction: "view_summary",
      supportEvidenceOnly: targetTruthState === "support_only",
      thinCurrentWork: false,
    });
    assert.equal(action.label, "Inspect evidence");
  }
});

test("typed no-clear task state defeats an otherwise openable target", () => {
  const action = getContinuePresentationActionState({
    decisionId: "decision",
    outputMode: "strong_continue",
    taskResolutionStatus: "no_clear_current_task",
    target: { openability: "openable", document_path: "/tmp/polluted" },
    targetTruthState: "direct_continue_ready",
    directTargetAllowed: true,
    answerAction: "continue_here",
    supportEvidenceOnly: false,
    thinCurrentWork: false,
  });
  assert.deepEqual(action, { kind: "no_clear_continuation", label: "Inspect evidence" });
});

test("observed activity can expose its strictly validated direct target without an explicit goal", () => {
  const action = getContinuePresentationActionState({
    decisionId: "decision-activity",
    outputMode: "thin_continue",
    taskResolutionStatus: "no_clear_current_task",
    workResolutionStatus: "activity_supported",
    target: { openability: "openable", document_path: "/tmp/tt2-05-completion-audit.md" },
    targetTruthState: "direct_continue_ready",
    directTargetAllowed: true,
    answerAction: "continue_here",
    supportEvidenceOnly: false,
    thinCurrentWork: false,
  });
  assert.deepEqual(action, { kind: "openable_return_target", label: "Continue here" });
});

test("no-clear task headline ignores polluted answer recap workstream and focus text", () => {
  assert.equal(
    selectPrimaryTaskHeadline(
      "Approve for me",
      "Old completed task",
      "Historical workstream",
      "ChatGPT",
      "no_clear_current_task",
    ),
    NO_CLEAR_CURRENT_TASK_HEADLINE,
  );
});

test("weaker background result cannot replace a stronger manual answer", () => {
  const incumbent = adoptionDecision();
  const challenger = adoptionDecision({
    decision_id: "decision-background",
    evidence_watermark_hash: "watermark-300",
    evidence_freshness_ledger: {
      latest_any_evidence_ms: 300,
      latest_non_self_evidence_ms: 300,
    },
    confidence_summary: {
      ...incumbent.confidence_summary,
      task: { score: 0.62, label: "medium", missing_evidence: ["speaker_attribution"] },
    },
    answer: {
      what_you_were_doing: "Repair causal evidence",
      where_you_left_off: "The failing relationship was isolated",
      next: "Inspect evidence",
    },
    activity_recap: {
      primary_work_summary: "Repair causal evidence",
      last_meaningful_state: "The failing relationship was isolated",
      next_action_summary: "Inspect evidence",
      generated_by: "local",
    },
    wording_source: "local",
  });
  const comparison = compareContinueDecisionAdoption({
    incumbent,
    challenger,
    incumbentTrigger: "manual",
    challengerTrigger: "background",
  });
  assert.equal(comparison.adopt, false);
  assert.ok(comparison.reasonCodes.includes("rejected:lower_task_identity_confidence"));
  assert.ok(comparison.reasonCodes.includes("rejected:lost_supported_where"));
  assert.ok(comparison.reasonCodes.includes("retained:stronger_manual_result"));
});

test("authoritative adoption compares snapshot truth instead of polluted legacy fields", () => {
  const incumbent = authoritativeDecision({ confidence: 0.95 });
  const challenger = authoritativeDecision({
    confidence: 0.55,
    revision: 3,
    overrides: {
      current_task_turn: {
        task_turn_id: "misleading-legacy-task",
        revision: 99,
        latest_user_goal_summary: "Unrelated legacy task",
        goal_confidence: 1,
        last_observed_at_ms: 300,
        updated_at_ms: 300,
      },
      evidence_freshness_ledger: {
        latest_any_evidence_ms: 300,
        latest_non_self_evidence_ms: 300,
      },
    },
  });
  const comparison = compareContinueDecisionAdoption({
    incumbent,
    challenger,
    incumbentTrigger: "manual",
    challengerTrigger: "background",
  });
  assert.equal(comparison.adopt, false);
  assert.ok(comparison.reasonCodes.includes("rejected:lower_task_identity_confidence"));
  assert.ok(comparison.reasonCodes.includes("retained:stronger_manual_result"));
  assert.ok(!comparison.reasonCodes.includes("rejected:new_task_not_causally_newer"));
});

test("background result without causally newer evidence is retained even when timestamps differ by request", () => {
  const incumbent = adoptionDecision();
  const challenger = adoptionDecision({
    decision_id: "decision-background",
    evidence_freshness_ledger: {
      decision_watermark_ms: 999,
      latest_any_evidence_ms: 200,
      latest_non_self_evidence_ms: 200,
    },
  });
  const comparison = compareContinueDecisionAdoption({
    incumbent,
    challenger,
    incumbentTrigger: "manual",
    challengerTrigger: "background",
  });
  assert.equal(comparison.adopt, false);
  assert.ok(comparison.reasonCodes.includes("rejected:evidence_not_causally_newer"));
});

test("genuinely newer stronger background task can replace the older answer", () => {
  const incumbent = adoptionDecision();
  const challenger = adoptionDecision({
    decision_id: "decision-new-task",
    current_task_turn: {
      task_turn_id: "task-two",
      revision: 1,
      latest_user_goal_summary: "Verify the repaired relationship",
      goal_confidence: 0.96,
      last_observed_at_ms: 400,
      updated_at_ms: 400,
    },
    evidence_watermark_hash: "watermark-400",
    latest_boundary_revision: 3,
    evidence_freshness_ledger: {
      latest_any_evidence_ms: 400,
      latest_non_self_evidence_ms: 400,
    },
    confidence_summary: {
      task: { score: 0.96, label: "high", missing_evidence: [] },
      state: { score: 0.9, label: "high", missing_evidence: [] },
      target: { score: 0.9, label: "high", missing_evidence: [] },
    },
    answer: {
      what_you_were_doing: "Verify the repaired relationship",
      where_label: "Smalltalk",
      where_you_left_off: "The implementation is ready for verification",
      next: "Run the focused regression tests",
    },
  });
  const comparison = compareContinueDecisionAdoption({
    incumbent,
    challenger,
    incumbentTrigger: "manual",
    challengerTrigger: "background",
  });
  assert.deepEqual(comparison, {
    adopt: true,
    reasonCodes: ["adopted:quality_not_lower"],
  });
});

test("weaker native-island update also retains the stronger manual answer", () => {
  const incumbent = adoptionDecision();
  const challenger = adoptionDecision({
    decision_id: "decision-island",
    evidence_freshness_ledger: {
      latest_any_evidence_ms: 300,
      latest_non_self_evidence_ms: 300,
    },
    confidence_summary: {
      ...incumbent.confidence_summary,
      task: { score: 0.5, label: "low", missing_evidence: ["latest_user_goal"] },
    },
  });
  const comparison = compareContinueDecisionAdoption({
    incumbent,
    challenger,
    incumbentTrigger: "manual",
    challengerTrigger: "island",
  });
  assert.equal(comparison.adopt, false);
  assert.ok(comparison.reasonCodes.includes("retained:stronger_manual_result"));
});

test("explicit manual refresh may replace an old answer with an honest no-clear state", () => {
  const incumbent = adoptionDecision();
  const challenger = adoptionDecision({
    decision_id: "decision-manual-no-clear",
    task_resolution_status: "no_clear_current_task",
    current_task_turn: null,
    continue_output_mode: "no_clear_continuation",
    target_truth: { state: "no_clear_task" },
    direct_target_policy: { direct_target_allowed: false },
    return_target: null,
  });
  assert.deepEqual(
    compareContinueDecisionAdoption({
      incumbent,
      challenger,
      incumbentTrigger: "manual",
      challengerTrigger: "manual",
    }),
    {
      adopt: true,
      reasonCodes: ["adopted:explicit_manual_result"],
    },
  );
});
