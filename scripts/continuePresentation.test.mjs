import assert from "node:assert/strict";
import test from "node:test";

import {
  authoritativeTaskTruthActionState,
  authoritativeTaskTruthTarget,
  compareContinueDecisionAdoption,
  getContinuePresentationActionState,
  inspectTargetCopy,
  NO_CLEAR_CURRENT_TASK_HEADLINE,
  selectPrimaryTaskHeadline,
  splitConfidenceLabels,
} from "../src/continuePresentation.ts";

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

test("frame preview is inspect-only and contains no target-shaped action copy", () => {
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
  assert.equal(copy.targetLine, "Captured evidence is available to inspect");
  assert.doesNotMatch(JSON.stringify({ action, copy }), /Continue here|safest return point|open the work/i);
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
