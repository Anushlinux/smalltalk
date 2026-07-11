export type ContinuePresentationActionState =
  | { kind: "openable_return_target"; label: "Continue here" }
  | { kind: "thin_current_work"; label: "Inspect evidence" }
  | { kind: "no_clear_continuation"; label: "Inspect evidence" };

export type ContinueTaskResolutionStatus =
  | "resolved_current_task"
  | "current_task_supported"
  | "no_clear_current_task"
  | "unknown";

export type ContinueDecisionRequestTrigger =
  | "manual"
  | "startup"
  | "background"
  | "island";

export type ContinueDecisionSupportedSurface =
  | string
  | {
      app_name?: string | null;
      window_title?: string | null;
      surface_key_hash?: string | null;
      evidence_ids?: string[];
    };

export type ContinueAlternativeHypothesis =
  | string
  | {
      label?: string | null;
      confidence?: number | null;
      evidence_ids?: string[];
    };

export type ContinueCurrentTaskTurnSummary = {
  task_turn_id: string;
  revision: number;
  latest_user_goal_summary?: string | null;
  goal_confidence: number;
  evidence_quality?: string | null;
  execution_state?: string | null;
  last_observed_at_ms?: number | null;
  updated_at_ms?: number | null;
};

export type ContinueEvidenceFreshnessSummary = {
  decision_watermark_ms?: number | null;
  latest_any_evidence_ms?: number | null;
  latest_non_self_evidence_ms?: number | null;
  latest_heavy_frame_ms?: number | null;
  latest_event_ms?: number | null;
  selected_candidate_evidence_ms?: number | null;
};

type ContinueClaimSummary = {
  score?: number | null;
  label?: string | null;
  missing_evidence?: string[];
};

type ContinueAdoptionTarget = ContinuePresentationTarget & {
  artifact_id?: string | null;
};

type ContinueTaskTruthFieldSupport = {
  confidence?: number | null;
  support_status?: string | null;
  evidence_refs?: string[];
};

type ContinueTaskTruthAnswer = {
  task_resolution_status?: string | null;
  task_summary?: string | null;
  task_object?: string | null;
  last_meaningful_progress?: string | null;
  unfinished_state?: string | null;
  next_action?: string | null;
  where_summary?: string | null;
  direct_return_target?: ContinueAdoptionTarget | null;
  evidence_preview?: {
    frame_id?: string | null;
  } | null;
  field_support?: Record<string, ContinueTaskTruthFieldSupport> | null;
  task_understanding_source?: string | null;
  wording_source?: string | null;
  target_selection_source?: string | null;
  snapshot_id?: string | null;
  snapshot_revision?: number | null;
  evidence_watermark?: string | null;
};

type ContinueTaskTruthProductionDecision = {
  effective_state?: string | null;
  release_gate_passed?: boolean | null;
  answer?: ContinueTaskTruthAnswer | null;
};

export type ContinueAdoptionComparableDecision = {
  decision_id: string;
  source?: string | null;
  request_trigger?: string | null;
  task_resolution_status?: string | null;
  current_task_turn?: ContinueCurrentTaskTurnSummary | null;
  evidence_watermark_hash?: string | null;
  latest_boundary_revision?: number | null;
  evidence_freshness_ledger?: ContinueEvidenceFreshnessSummary | null;
  confidence_summary?: {
    task?: ContinueClaimSummary | null;
    state?: ContinueClaimSummary | null;
    target?: ContinueClaimSummary | null;
  } | null;
  validation_status?: string | null;
  continue_output_mode?: string | null;
  target_truth?: {
    state?: string | null;
  } | null;
  direct_target_policy?: {
    direct_target_allowed?: boolean | null;
  } | null;
  answer?: {
    what_you_were_doing?: string | null;
    where_label?: string | null;
    where_you_left_off?: string | null;
    next?: string | null;
  } | null;
  activity_recap?: {
    primary_work_summary?: string | null;
    primary_where_summary?: string | null;
    last_meaningful_state?: string | null;
    unfinished_state?: string | null;
    next_action_summary?: string | null;
    generated_by?: string | null;
  } | null;
  return_target?: ContinueAdoptionTarget | null;
  resume_work_target?: ContinueAdoptionTarget | null;
  wording_source?: string | null;
  task_truth_v2?: ContinueTaskTruthProductionDecision | null;
};

export type ContinueAdoptionComparison = {
  adopt: boolean;
  reasonCodes: string[];
};

export const NO_CLEAR_CURRENT_TASK_HEADLINE = "The exact task could not be determined";

export const NO_CLEAR_CURRENT_TASK_COPY = {
  heroLabel: "Recent activity was captured",
  targetBlockLabel: "Evidence available",
  targetLine: "Recent activity is available to inspect",
  targetMeta: "The current evidence does not support one exact task or return location.",
  lastStateLine: "No current task state is supported by the available evidence.",
  nextActionLine: "Inspect the recent evidence or keep working until the task becomes clear.",
  uncertaintyLine:
    "Smalltalk will not turn older activity into a current task without supporting evidence.",
} as const;

export type ContinuePresentationTarget = {
  browser_url?: string | null;
  document_path?: string | null;
  openability?: string | null;
};

export type ContinueActionPolicyInput = {
  decisionId?: string | null;
  outputMode?: string | null;
  taskResolutionStatus?: string | null;
  workResolutionStatus?: string | null;
  target?: ContinuePresentationTarget | null;
  targetTruthState?: string | null;
  directTargetAllowed?: boolean | null;
  answerAction?: string | null;
  supportEvidenceOnly: boolean;
  thinCurrentWork: boolean;
};

export type InspectTargetCopyInput = {
  taskKnown: boolean;
  evidencePreviewAvailable: boolean;
  appFocusOnly: boolean;
  targetNote?: string | null;
};

function normalizeToken(value?: string | null) {
  return (value || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

export function isDirectPresentationTargetOpenable(
  target?: ContinuePresentationTarget | null,
) {
  return Boolean(
    target &&
      normalizeToken(target.openability) === "openable" &&
      (target.browser_url || target.document_path),
  );
}

export function authoritativeTaskTruthAnswer(
  decision?: ContinueAdoptionComparableDecision | null,
) {
  if (
    normalizeToken(decision?.task_truth_v2?.effective_state) !== "authoritative" ||
    decision?.task_truth_v2?.release_gate_passed !== true
  ) {
    return null;
  }
  return decision.task_truth_v2.answer || null;
}

export function authoritativeTaskTruthTarget(
  decision?: ContinueAdoptionComparableDecision | null,
) {
  const answer = authoritativeTaskTruthAnswer(decision);
  return answer ? answer.direct_return_target || null : null;
}

export function authoritativeTaskTruthActionState(
  decision?: ContinueAdoptionComparableDecision | null,
): ContinuePresentationActionState | null {
  const answer = authoritativeTaskTruthAnswer(decision);
  if (!decision || !answer) return null;
  const target = answer.direct_return_target || null;
  const unresolved = normalizeToken(answer.task_resolution_status) === "unresolved";
  return getContinuePresentationActionState({
    decisionId: decision.decision_id,
    outputMode: unresolved ? "no_clear_continuation" : "strong_continue",
    taskResolutionStatus: unresolved ? "no_clear_current_task" : "resolved_current_task",
    target,
    targetTruthState: target ? "direct_continue_ready" : "task_known_target_unknown",
    directTargetAllowed: Boolean(target),
    answerAction: target ? "continue_here" : "inspect_evidence",
    supportEvidenceOnly: false,
    thinCurrentWork: !unresolved && !target,
  });
}

export function getContinuePresentationActionState(
  input: ContinueActionPolicyInput,
): ContinuePresentationActionState {
  const workSupported = ["task_supported", "activity_supported"].includes(
    normalizeToken(input.workResolutionStatus),
  );
  if (
    normalizeTaskResolutionStatus(input.taskResolutionStatus) === "no_clear_current_task"
    && !workSupported
  ) {
    return { kind: "no_clear_continuation", label: "Inspect evidence" };
  }
  const directStateAllowed =
    !input.targetTruthState || input.targetTruthState === "direct_continue_ready";
  const directPolicyAllowed = input.directTargetAllowed !== false;
  const answerAllowsDirect = !input.answerAction || input.answerAction === "continue_here";
  if (
    input.decisionId?.trim() &&
    input.outputMode !== "no_clear_continuation" &&
    isDirectPresentationTargetOpenable(input.target) &&
    directStateAllowed &&
    directPolicyAllowed &&
    answerAllowsDirect &&
    !input.supportEvidenceOnly
  ) {
    return { kind: "openable_return_target", label: "Continue here" };
  }
  if (
    input.thinCurrentWork ||
    input.targetTruthState === "task_known_target_unknown" ||
    input.targetTruthState === "activity_known_target_unknown" ||
    input.targetTruthState === "thin_task_seen"
  ) {
    return { kind: "thin_current_work", label: "Inspect evidence" };
  }
  return { kind: "no_clear_continuation", label: "Inspect evidence" };
}

export function inspectTargetCopy(input: InspectTargetCopyInput) {
  const targetLine = input.evidencePreviewAvailable
    ? "Captured evidence is available to inspect"
    : input.appFocusOnly
      ? "The app is known, but the exact page, conversation, or file is unavailable"
      : "No direct page or file locator is available";
  const targetMeta = input.targetNote?.trim() || (
    input.taskKnown
      ? "I know the task, but I do not have a direct page or file locator."
      : "The exact task location is unavailable from the current evidence."
  );
  return {
    targetBlockLabel: "Exact location unavailable",
    targetLine,
    targetMeta,
    actionLabel: "Inspect evidence" as const,
  };
}

export function selectPrimaryTaskHeadline(
  answerTask?: string | null,
  recapTask?: string | null,
  workstreamTask?: string | null,
  currentFocus?: string | null,
  taskResolutionStatus?: string | null,
) {
  if (normalizeTaskResolutionStatus(taskResolutionStatus) === "no_clear_current_task") {
    return NO_CLEAR_CURRENT_TASK_HEADLINE;
  }
  return answerTask?.trim()
    || recapTask?.trim()
    || workstreamTask?.trim()
    || currentFocus?.trim()
    || "Recent work";
}

export function splitConfidenceLabels(
  taskLabel?: string | null,
  targetLabel?: string | null,
) {
  return {
    task: normalizeToken(taskLabel) || "none",
    target: normalizeToken(targetLabel) || "none",
  };
}

export function normalizeTaskResolutionStatus(
  value?: string | null,
): ContinueTaskResolutionStatus {
  const normalized = normalizeToken(value);
  if (["no_clear_current_task", "no_clear_task", "no_current_goal"].includes(normalized)) {
    return "no_clear_current_task";
  }
  if (
    [
      "resolved_current_task",
      "current_task_supported",
      "current_task_resolved",
      "resolved",
    ].includes(normalized)
  ) {
    return "resolved_current_task";
  }
  return "unknown";
}

export function compareContinueDecisionAdoption({
  incumbent,
  challenger,
  incumbentTrigger,
  challengerTrigger,
}: {
  incumbent?: ContinueAdoptionComparableDecision | null;
  challenger: ContinueAdoptionComparableDecision;
  incumbentTrigger?: ContinueDecisionRequestTrigger | null;
  challengerTrigger: ContinueDecisionRequestTrigger;
}): ContinueAdoptionComparison {
  if (!incumbent) {
    return { adopt: true, reasonCodes: ["adopted:no_incumbent"] };
  }

  const incumbentTask = adoptionTaskIdentity(incumbent);
  const challengerTask = adoptionTaskIdentity(challenger);
  const sameTask = Boolean(
    incumbentTask?.task_turn_id &&
      challengerTask?.task_turn_id &&
      incumbentTask.task_turn_id === challengerTask.task_turn_id,
  );
  const explicitManual = challengerTrigger === "manual";
  const reasons: string[] = [];

  if (
    sameTask &&
    typeof incumbentTask?.revision === "number" &&
    typeof challengerTask?.revision === "number" &&
    challengerTask.revision < incumbentTask.revision
  ) {
    reasons.push("rejected:older_task_revision");
  }

  if (!explicitManual) {
    const incumbentStatus = decisionTaskResolutionStatus(incumbent);
    const challengerStatus = decisionTaskResolutionStatus(challenger);
    const challengerEvidenceIsNewer = hasCausallyNewerEvidence(incumbent, challenger);

    if (incumbentTask && !challengerTask) {
      reasons.push("rejected:lost_task_identity");
    } else if (incumbentTask && challengerTask && !sameTask) {
      if (challengerStatus !== "resolved_current_task") {
        reasons.push("rejected:new_task_not_resolved");
      }
      if (!challengerEvidenceIsNewer) {
        reasons.push("rejected:new_task_not_causally_newer");
      }
    } else if (!challengerEvidenceIsNewer) {
      reasons.push("rejected:evidence_not_causally_newer");
    }

    if (
      incumbentStatus === "resolved_current_task" &&
      challengerStatus === "no_clear_current_task"
    ) {
      reasons.push("rejected:lost_supported_current_task");
    }

    const incumbentTaskConfidence = adoptionTaskConfidence(incumbent);
    const challengerTaskConfidence = adoptionTaskConfidence(challenger);
    if (challengerTaskConfidence + 0.000_001 < incumbentTaskConfidence) {
      reasons.push("rejected:lower_task_identity_confidence");
    }

    for (const field of lostSupportedFields(incumbent, challenger)) {
      reasons.push(`rejected:lost_supported_${field}`);
    }

    if (decisionValidationRank(challenger) < decisionValidationRank(incumbent)) {
      reasons.push("rejected:model_validation_downgrade");
    }
    if (targetPolicyRank(challenger) < targetPolicyRank(incumbent)) {
      reasons.push("rejected:target_policy_downgrade");
    }
    if (wordingSourceRank(challenger) < wordingSourceRank(incumbent)) {
      reasons.push("rejected:wording_source_downgrade");
    }

    if (
      incumbentTrigger === "manual" &&
      reasons.length > 0
    ) {
      reasons.push("retained:stronger_manual_result");
    }
  }

  const reasonCodes = [...new Set(reasons)].slice(0, 12);
  if (reasonCodes.length > 0) {
    return { adopt: false, reasonCodes };
  }
  return {
    adopt: true,
    reasonCodes: [explicitManual ? "adopted:explicit_manual_result" : "adopted:quality_not_lower"],
  };
}

function decisionTaskResolutionStatus(
  decision: ContinueAdoptionComparableDecision,
): ContinueTaskResolutionStatus {
  const taskTruth = authoritativeTaskTruthAnswer(decision);
  if (taskTruth) {
    const status = normalizeToken(taskTruth.task_resolution_status);
    if (status === "unresolved") return "no_clear_current_task";
    if (status === "resolved" || status === "ambiguous") return "resolved_current_task";
  }
  const explicit = normalizeTaskResolutionStatus(decision.task_resolution_status);
  if (explicit !== "unknown") return explicit;
  if (
    normalizeToken(decision.continue_output_mode) === "no_clear_continuation" ||
    normalizeToken(decision.target_truth?.state) === "no_clear_task"
  ) {
    return "no_clear_current_task";
  }
  return decision.current_task_turn ? "resolved_current_task" : "unknown";
}

function hasCausallyNewerEvidence(
  incumbent: ContinueAdoptionComparableDecision,
  challenger: ContinueAdoptionComparableDecision,
) {
  const incumbentTimestamp = causalEvidenceTimestamp(incumbent);
  const challengerTimestamp = causalEvidenceTimestamp(challenger);
  if (challengerTimestamp > incumbentTimestamp) return true;
  if (challengerTimestamp < incumbentTimestamp) return false;

  const incumbentRevision = adoptionTaskIdentity(incumbent)?.revision ?? -1;
  const challengerRevision = adoptionTaskIdentity(challenger)?.revision ?? -1;
  if (challengerRevision > incumbentRevision) return true;

  const incumbentBoundary = incumbent.latest_boundary_revision ?? -1;
  const challengerBoundary = challenger.latest_boundary_revision ?? -1;
  if (challengerBoundary > incumbentBoundary) return true;

  return false;
}

function causalEvidenceTimestamp(decision: ContinueAdoptionComparableDecision) {
  const ledger = decision.evidence_freshness_ledger;
  return Math.max(
    0,
    decision.current_task_turn?.last_observed_at_ms || 0,
    decision.current_task_turn?.updated_at_ms || 0,
    ledger?.latest_any_evidence_ms || 0,
    ledger?.latest_non_self_evidence_ms || 0,
    ledger?.latest_heavy_frame_ms || 0,
    ledger?.latest_event_ms || 0,
    ledger?.selected_candidate_evidence_ms || 0,
  );
}

function adoptionTaskIdentity(decision: ContinueAdoptionComparableDecision) {
  const taskTruth = authoritativeTaskTruthAnswer(decision);
  if (taskTruth?.snapshot_id) {
    return {
      task_turn_id: taskTruth.snapshot_id,
      revision: taskTruth.snapshot_revision ?? 0,
      latest_user_goal_summary: taskTruth.task_summary,
      goal_confidence: adoptionTaskConfidence(decision),
    } satisfies ContinueCurrentTaskTurnSummary;
  }
  return decision.current_task_turn || null;
}

function adoptionTaskConfidence(decision: ContinueAdoptionComparableDecision) {
  const taskTruth = authoritativeTaskTruthAnswer(decision);
  if (taskTruth) {
    const scores = ["task_summary", "task_object"]
      .map((field) => taskTruth.field_support?.[field]?.confidence)
      .filter((score): score is number => typeof score === "number");
    if (scores.length > 0) return Math.min(...scores);
    return normalizeToken(taskTruth.task_resolution_status) === "resolved" ? 0.85 : 0.6;
  }
  return claimScore(decision.confidence_summary?.task);
}

function decisionValidationRank(decision: ContinueAdoptionComparableDecision) {
  const taskTruth = authoritativeTaskTruthAnswer(decision);
  if (taskTruth) {
    const status = normalizeToken(taskTruth.task_resolution_status);
    if (status === "resolved") return 3;
    if (status === "ambiguous") return 2;
    return 1;
  }
  return validationRank(decision.validation_status);
}

function claimScore(claim?: ContinueClaimSummary | null) {
  if (typeof claim?.score === "number") return claim.score;
  const ranks: Record<string, number> = { none: 0, low: 0.25, medium: 0.6, high: 0.85 };
  return ranks[normalizeToken(claim?.label)] || 0;
}

function lostSupportedFields(
  incumbent: ContinueAdoptionComparableDecision,
  challenger: ContinueAdoptionComparableDecision,
) {
  const incumbentCoverage = supportedFieldCoverage(incumbent);
  const challengerCoverage = supportedFieldCoverage(challenger);
  return (Object.keys(incumbentCoverage) as Array<keyof typeof incumbentCoverage>).filter(
    (field) => incumbentCoverage[field] && !challengerCoverage[field],
  );
}

function supportedFieldCoverage(decision: ContinueAdoptionComparableDecision) {
  const noClear = decisionTaskResolutionStatus(decision) === "no_clear_current_task";
  const taskTruth = authoritativeTaskTruthAnswer(decision);
  if (taskTruth) {
    return {
      task: !noClear && Boolean(taskTruth.task_summary?.trim()),
      state: Boolean(
        taskTruth.last_meaningful_progress?.trim() || taskTruth.unfinished_state?.trim(),
      ),
      next: Boolean(taskTruth.next_action?.trim()),
      where: Boolean(taskTruth.where_summary?.trim()),
    };
  }
  return {
    task: !noClear && Boolean(
      decision.current_task_turn?.latest_user_goal_summary?.trim() ||
        decision.answer?.what_you_were_doing?.trim() ||
        decision.activity_recap?.primary_work_summary?.trim(),
    ),
    state: Boolean(
      decision.answer?.where_you_left_off?.trim() ||
        decision.activity_recap?.last_meaningful_state?.trim() ||
        decision.activity_recap?.unfinished_state?.trim(),
    ),
    next: Boolean(
      decision.answer?.next?.trim() || decision.activity_recap?.next_action_summary?.trim(),
    ),
    where: Boolean(
      decision.answer?.where_label?.trim() || decision.activity_recap?.primary_where_summary?.trim(),
    ),
  };
}

function validationRank(value?: string | null) {
  const normalized = normalizeToken(value);
  if (normalized.includes("rejected") || normalized.includes("invalid")) return 0;
  if (normalized.includes("fallback") || normalized.includes("thin")) return 1;
  if (normalized.includes("soft_recovered")) return 2;
  if (normalized.includes("validated") || normalized.includes("valid")) return 3;
  return 1;
}

function targetPolicyRank(decision: ContinueAdoptionComparableDecision) {
  const taskTruth = authoritativeTaskTruthAnswer(decision);
  if (taskTruth) {
    return isDirectPresentationTargetOpenable(taskTruth.direct_return_target) ? 2 : 1;
  }
  if (
    decision.direct_target_policy?.direct_target_allowed &&
    normalizeToken(decision.target_truth?.state) === "direct_continue_ready"
  ) {
    return 2;
  }
  if (
    ["task_known_target_unknown", "thin_task_seen"].includes(
      normalizeToken(decision.target_truth?.state),
    )
  ) {
    return 1;
  }
  return 0;
}

function wordingSourceRank(decision: ContinueAdoptionComparableDecision) {
  const source = normalizeToken(
    authoritativeTaskTruthAnswer(decision)?.wording_source ||
      decision.wording_source ||
      decision.activity_recap?.generated_by,
  );
  if (source.includes("model") || source.includes("cloud")) return 2;
  if (source.includes("fallback")) return 0;
  return 1;
}
