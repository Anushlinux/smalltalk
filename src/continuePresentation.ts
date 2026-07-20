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

export type ContinueTaskTruthAtomicIdentity = {
  session_id?: string | null;
  task_thread_id: string | null;
  task_thread_revision: number | null;
  task_snapshot_id: string;
  snapshot_revision: number;
  selected_hypothesis_id: string | null;
  model_request_id?: string | null;
  model_response_id: string | null;
  observation_packet_id: string;
  evidence_watermark: string;
  correction_fingerprint: string;
};

export type ContinueTaskTruthAlternative = {
  hypothesis_id: string;
  task_summary: string;
  relation: string;
  confidence: number;
  evidence_refs: string[];
  contradicting_evidence_refs?: string[];
  task_thread_id?: string | null;
  task_thread_revision?: number | null;
  last_supported_at_ms?: number | null;
  disposition?: string | null;
  reason_codes?: string[];
  semantic_payload?: unknown | null;
};

export type ContinueTaskTruthRecentContext = {
  sequence_index: number;
  app_label: string;
  site_hostname?: string | null;
  first_observed_at_ms: number;
  last_observed_at_ms: number;
  is_current: boolean;
  revisited: boolean;
  evidence_refs?: string[];
  semantic_role?: "primary_work" | "supporting_work" | "detour_or_unrelated" | "unclear" | null;
  role_confidence?: number | null;
  relationship_to_primary_task?: string | null;
  role_evidence_refs?: string[];
};

export type ContinuePublicProjection = {
  headline: string;
  memoryLine: string | null;
  resumeSurface: string | null;
  openActionLabel: string | null;
  exactTargetNote: string | null;
};

export type ContinueSurfaceProjection = {
  label: string;
  kind: "App" | "Page";
  appLabel: string | null;
  siteHostname: string | null;
};

export type ContinueContinuationFieldProjection = {
  checkpoint: string | null;
  continuation: string;
  checkpointSurface: ContinueSurfaceProjection | null;
  continuationSurface: ContinueSurfaceProjection | null;
  locationLabel: string | null;
  targetStatus: string | null;
  openActionLabel: string | null;
  recentContext: ContinueTaskTruthRecentContext[];
};

export type ContinueTaskTruthAnswer = {
  schema: string;
  task_basis?: string | null;
  task_resolution_status?: string | null;
  observed_surface?: string | null;
  immediate_user_operation?: string | null;
  semantic_effect_of_operation?: string | null;
  current_subtask?: string | null;
  current_activity?: {
    observed_surface?: string | null;
    immediate_user_operation?: string | null;
    semantic_effect_of_operation?: string | null;
    current_subtask?: string | null;
    relationship_to_primary?: string | null;
  } | null;
  task_summary?: string | null;
  task_object?: string | null;
  last_meaningful_progress?: string | null;
  unfinished_state?: string | null;
  execution_state?: string | null;
  next_action?: string | null;
  where_summary?: string | null;
  relationship_to_prior?: string | null;
  recent_context?: ContinueTaskTruthRecentContext[];
  alternative_hypotheses: ContinueTaskTruthAlternative[];
  direct_return_target?: ContinueAdoptionTarget | null;
  evidence_preview?: {
    frame_id?: string | null;
  } | null;
  field_support?: Record<string, ContinueTaskTruthFieldSupport> | null;
  task_understanding_source?: string | null;
  wording_source?: string | null;
  target_selection_source?: string | null;
  snapshot_id: string;
  snapshot_revision: number;
  evidence_watermark: string;
  semantic_source?: string | null;
  provider_name?: string | null;
  provider_model?: string | null;
  request_id?: string | null;
  response_id?: string | null;
  selected_hypothesis_id: string | null;
  inference_status?: string | null;
  atomic_identity: ContinueTaskTruthAtomicIdentity;
};

type ContinueTaskTruthProductionDecision = {
  effective_state?: string | null;
  release_gate_passed?: boolean | null;
  answer?: ContinueTaskTruthAnswer | null;
  inference_diagnostic?: {
    status?: string | null;
    origin?: string | null;
  } | null;
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

export const NO_CLEAR_CURRENT_TASK_HEADLINE = "I couldn't determine where to resume";

export const NO_CLEAR_CURRENT_TASK_COPY = {
  heroLabel: "No safe continuation yet",
  targetBlockLabel: "What to do",
  targetLine: "Return to the work you want to continue, then try Continue again",
  targetMeta: "Smalltalk did not find a verified page, conversation, or file to reopen.",
  lastStateLine: "Your recent activity did not establish one safe continuation point.",
  nextActionLine: "Keep the work surface visible briefly before trying again.",
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

export function isTaskInferenceUnavailable(status?: string | null) {
  return [
    "model_unavailable",
    "provider_error",
    "provider_failure",
    "provider_unavailable",
  ].includes(normalizeToken(status));
}

export type TaskInferenceFailureKind =
  | "capture_unavailable"
  | "provider_disabled"
  | "credentials_missing"
  | "model_unavailable"
  | "provider_timeout"
  | "provider_request_rejected"
  | "provider_failure"
  | "provider_unavailable"
  | "provider_no_usable_output"
  | "model_response_invalid"
  | "evidence_verifier_rejected"
  | "insufficient_evidence";

export function taskInferenceFailurePresentation(
  status?: string | null,
  verificationStatus?: string | null,
  origin?: string | null,
  imageCount?: number | null,
  providerAttemptCount?: number | null,
) {
  const normalized = normalizeToken(status);
  const verification = normalizeToken(verificationStatus);
  const normalizedOrigin = normalizeToken(origin);
  const providerRequestWasBuilt = (imageCount || 0) > 0;
  const providerWasAttempted = (providerAttemptCount || 0) > 0
    || (normalizedOrigin === "live_cloud" && providerRequestWasBuilt);
  if (verification === "verification_rejected" || normalized === "verification_rejected") {
    return {
      kind: "evidence_verifier_rejected" as TaskInferenceFailureKind,
      headline: "The proposed task did not match the captured evidence",
      detail: "Cloud inference returned a task, but the local evidence verifier rejected it.",
      retryable: true,
    };
  }
  if (normalized === "support_slot_validation_failure") {
    return {
      kind: "evidence_verifier_rejected" as TaskInferenceFailureKind,
      headline: "The proposed task did not match the captured evidence",
      detail: "Cloud inference returned an answer, but its cited evidence did not pass local validation.",
      retryable: true,
    };
  }
  if (normalized === "provider_no_usable_output") {
    return {
      kind: "provider_no_usable_output" as TaskInferenceFailureKind,
      headline: "Cloud task inference did not return a usable answer",
      detail: "The provider did not return one complete, valid task answer.",
      retryable: true,
    };
  }
  if ([
    "invalid_response",
    "invalid_atomic_identity",
    "missing_model_response_identity",
    "structured_parse_failure",
    "unsupported_semantic_source",
  ].includes(normalized)) {
    return {
      kind: "model_response_invalid" as TaskInferenceFailureKind,
      headline: "The model response could not be validated",
      detail: normalized === "invalid_atomic_identity"
        ? "The provider responded, but the task, snapshot, and inference identities did not form one valid decision."
        : "The provider responded, but its evidence references did not satisfy the response contract.",
      retryable: true,
    };
  }
  if (
    normalized === "request_rejected"
    || normalized === "provider_rejected"
    || (normalized === "request_invalid" && providerWasAttempted)
  ) {
    return {
      kind: "provider_request_rejected" as TaskInferenceFailureKind,
      headline: "Cloud task inference could not accept this request",
      detail: "The provider rejected the bounded inference request before returning a task answer.",
      retryable: true,
    };
  }
  if (
    ["privacy_blocked", "request_invalid", "capture_unavailable"].includes(normalized)
  ) {
    return {
      kind: "capture_unavailable" as TaskInferenceFailureKind,
      headline: "Capture was unavailable for this Continue attempt",
      detail: "Smalltalk could not prepare a readable current-work packet for this request.",
      retryable: false,
    };
  }
  if (normalized === "disabled") {
    return {
      kind: "provider_disabled" as TaskInferenceFailureKind,
      headline: "Cloud task inference is disabled in this build",
      detail: "No provider request was attempted. Enable Task Truth inference and run Continue again.",
      retryable: false,
    };
  }
  if (normalized === "credentials_missing") {
    return {
      kind: "credentials_missing" as TaskInferenceFailureKind,
      headline: "Cloud task inference is not configured",
      detail: "No provider request was attempted because an OpenAI API key was not available.",
      retryable: false,
    };
  }
  if (normalized === "model_unavailable") {
    return {
      kind: "model_unavailable" as TaskInferenceFailureKind,
      headline: "The configured inference model is unavailable",
      detail: "The provider could not use the configured model for this request.",
      retryable: true,
    };
  }
  if (normalized === "provider_unavailable") {
    return {
      kind: "provider_unavailable" as TaskInferenceFailureKind,
      headline: "Cloud task inference is unavailable",
      detail: "Smalltalk could not complete the provider request. Check the provider configuration or try Continue again.",
      retryable: true,
    };
  }
  if (normalized === "timeout") {
    return {
      kind: "provider_timeout" as TaskInferenceFailureKind,
      headline: "Cloud task inference timed out",
      detail: "The provider did not return a verified answer before the request deadline.",
      retryable: true,
    };
  }
  if (["provider_error", "provider_failure"].includes(normalized)) {
    return {
      kind: "provider_failure" as TaskInferenceFailureKind,
      headline: "Cloud task inference failed",
      detail: "The provider request failed before Smalltalk received a verified task answer.",
      retryable: true,
    };
  }
  return {
    kind: "insufficient_evidence" as TaskInferenceFailureKind,
    headline: "There is not enough evidence for a clear continuation",
    detail: "Smalltalk could not support one task strongly enough, so it did not invent one.",
    retryable: false,
  };
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
  const releaseAuthoritative =
    normalizeToken(decision?.task_truth_v2?.effective_state) === "authoritative" &&
    decision?.task_truth_v2?.release_gate_passed === true;
  const currentManualInference =
    normalizeToken(decision?.request_trigger) === "manual" &&
    Boolean(decision?.task_truth_v2?.inference_diagnostic);
  const answer = decision?.task_truth_v2?.answer || null;
  if (!answer) return null;
  if (!releaseAuthoritative && !currentManualInference) {
    // Once model-first evidence exists, a startup/cache/background path must
    // not fall through to legacy semantic copy. Until the release gate opens,
    // present the boundary honestly as unresolved.
    return unresolvedTaskTruthAnswer(answer, "authority_not_released");
  }
  if (normalizeToken(answer.task_resolution_status) === "unresolved") return answer;
  if (hasCompleteTaskTruthAtomicIdentity(answer.atomic_identity)) return answer;

  // A Task Truth decision with incomplete provenance must not fall through to
  // the legacy task turn. Preserve the boundary as an honest unresolved result
  // while removing every semantic claim that lacked its required identity.
  return unresolvedTaskTruthAnswer(answer, "invalid_atomic_identity");
}

function unresolvedTaskTruthAnswer(
  answer: ContinueTaskTruthAnswer,
  inferenceStatus: string,
): ContinueTaskTruthAnswer {
  return {
    ...answer,
    task_resolution_status: "unresolved",
    observed_surface: null,
    immediate_user_operation: null,
    semantic_effect_of_operation: null,
    current_subtask: null,
    current_activity: {
      observed_surface: null,
      immediate_user_operation: null,
      semantic_effect_of_operation: null,
      current_subtask: null,
      relationship_to_primary: "unrelated_or_unknown",
    },
    task_summary: null,
    task_object: null,
    last_meaningful_progress: null,
    unfinished_state: null,
    execution_state: "unclear",
    next_action: null,
    where_summary: null,
    relationship_to_prior: "unrelated_or_unknown",
    alternative_hypotheses: [],
    direct_return_target: null,
    field_support: {},
    task_understanding_source: "unresolved",
    semantic_source: "unresolved",
    inference_status: inferenceStatus,
  };
}

function hasCompleteTaskTruthAtomicIdentity(
  identity: ContinueTaskTruthAtomicIdentity | null | undefined,
) {
  return Boolean(
    identity &&
      identity.task_thread_id?.trim() &&
      Number.isInteger(identity.task_thread_revision) &&
      (identity.task_thread_revision || 0) > 0 &&
      identity.task_snapshot_id.trim() &&
      Number.isInteger(identity.snapshot_revision) &&
      identity.snapshot_revision > 0 &&
      identity.selected_hypothesis_id?.trim() &&
      identity.model_response_id?.trim() &&
      identity.observation_packet_id.trim() &&
      identity.evidence_watermark.trim(),
  );
}

export function authoritativeTaskTruthTarget(
  decision?: ContinueAdoptionComparableDecision | null,
) {
  const answer = authoritativeTaskTruthAnswer(decision);
  return answer ? answer.direct_return_target || null : null;
}

export function recentContextForPresentation(
  answer?: ContinueTaskTruthAnswer | null,
) {
  const visible: ContinueTaskTruthRecentContext[] = [];
  for (const visit of answer?.recent_context || []) {
    if (
      !["primary_work", "supporting_work"].includes(visit.semantic_role || "")
      || !visit.relationship_to_primary_task?.trim()
    ) {
      continue;
    }
    const previous = visible[visible.length - 1];
    const sameSurface = previous
      && recentContextSurfaceLabel(previous) === recentContextSurfaceLabel(visit);
    const sameRole = previous?.semantic_role === visit.semantic_role;
    const sameRelationship = (previous?.relationship_to_primary_task || "").trim()
      === (visit.relationship_to_primary_task || "").trim();

    if (sameSurface && sameRole && sameRelationship) {
      visible[visible.length - 1] = {
        ...previous,
        ...visit,
        first_observed_at_ms: previous.first_observed_at_ms,
        evidence_refs: [
          ...(previous.evidence_refs || []),
          ...(visit.evidence_refs || []),
        ],
      };
      continue;
    }

    visible.push(visit);
  }
  if (visible.length > 0) return visible.slice(-4);

  const currentObservedSurface = [...(answer?.recent_context || [])]
    .reverse()
    .find((visit) => (
      visit.is_current
      && visit.app_label.trim()
      && normalizeToken(visit.app_label) !== "smalltalk"
    ));
  return currentObservedSurface ? [currentObservedSurface] : [];
}

export function recentContextSurfaceLabel(
  visit: ContinueTaskTruthRecentContext,
) {
  const app = visit.app_label.trim();
  const hostname = (visit.site_hostname || "")
    .trim()
    .toLowerCase()
    .replace(/^www\./, "");
  const appKey = app.toLowerCase();

  if (appKey === "code" || appKey === "visual studio code") return "VS Code";
  if (hostname === "thinkingmachines.ai") return "Thinking Machines";
  if (hostname === "chatgpt.com") return "ChatGPT";
  if (hostname === "platform.openai.com") return "OpenAI Platform";

  const browserShells = ["helium", "safari", "google chrome", "chrome", "arc", "firefox"];
  if (hostname && browserShells.includes(appKey)) {
    return hostname;
  }

  return app || "Work surface";
}

export function recentContextSurfaceKind(
  visit: ContinueTaskTruthRecentContext,
): ContinueSurfaceProjection["kind"] {
  return visit.site_hostname?.trim() ? "Page" : "App";
}

function surfaceProjectionFromVisit(
  visit?: ContinueTaskTruthRecentContext | null,
): ContinueSurfaceProjection | null {
  if (!visit?.app_label.trim() || normalizeToken(visit.app_label) === "smalltalk") {
    return null;
  }
  return {
    label: recentContextSurfaceLabel(visit),
    kind: recentContextSurfaceKind(visit),
    appLabel: publicClause(visit.app_label) || null,
    siteHostname: publicClause(visit.site_hostname) || null,
  };
}

function surfaceProjectionFromLabel(
  label?: string | null,
): ContinueSurfaceProjection | null {
  const cleanLabel = publicClause(label);
  if (!cleanLabel) return null;
  const looksLikePage = /^(?:https?:\/\/|www\.)/i.test(cleanLabel)
    || /^[a-z0-9-]+(?:\.[a-z0-9-]+)+$/i.test(cleanLabel);
  const knownAppLabels = [
    "arc",
    "chatgpt",
    "chrome",
    "codex",
    "discord",
    "figma",
    "firefox",
    "github",
    "helium",
    "notion",
    "openai",
    "safari",
    "slack",
    "terminal",
    "visual studio code",
    "vs code",
  ];
  if (!looksLikePage && !knownAppLabels.some((app) => normalizeToken(cleanLabel).includes(app))) {
    return null;
  }
  return {
    label: cleanLabel,
    kind: looksLikePage ? "Page" : "App",
    appLabel: looksLikePage ? null : cleanLabel,
    siteHostname: looksLikePage
      ? cleanLabel.replace(/^https?:\/\//i, "").replace(/^www\./i, "").split("/")[0] || null
      : null,
  };
}

export function continuationSurfacesForPresentation(
  answer?: ContinueTaskTruthAnswer | null,
  fallbackLabel?: string | null,
) {
  const visits = (answer?.recent_context || []).filter(
    (visit) => visit.app_label.trim() && normalizeToken(visit.app_label) !== "smalltalk",
  );
  const visitNamedIn = (copy?: string | null) => {
    const normalizedCopy = normalizeToken(copy || "");
    if (!normalizedCopy) return null;
    return [...visits].reverse().find((visit) => {
      const candidates = [
        visit.app_label,
        visit.site_hostname,
        recentContextSurfaceLabel(visit),
      ]
        .map((value) => normalizeToken(value || ""))
        .filter((value) => value.length >= 3);
      return candidates.some((value) => normalizedCopy.includes(value));
    }) || null;
  };
  const currentVisit = [...visits].reverse().find((visit) => visit.is_current) || null;
  const primaryVisit = [...visits].reverse().find((visit) => (
    visit.semantic_role === "primary_work"
    && Boolean(visit.relationship_to_primary_task?.trim())
  )) || null;
  const meaningfulVisit = [...visits].reverse().find((visit) => (
    ["primary_work", "supporting_work"].includes(visit.semantic_role || "")
    && Boolean(visit.relationship_to_primary_task?.trim())
  )) || null;
  const checkpointVisit = visitNamedIn(answer?.last_meaningful_progress)
    || primaryVisit
    || meaningfulVisit
    || currentVisit;
  const continuationVisit = visitNamedIn(answer?.unfinished_state || answer?.next_action)
    || null;
  const fallback = surfaceProjectionFromLabel(fallbackLabel);
  const checkpointSurface = surfaceProjectionFromVisit(checkpointVisit) || fallback;
  const continuationSurface = surfaceProjectionFromVisit(continuationVisit) || fallback;

  return { checkpointSurface, continuationSurface };
}

export function buildContinuePublicProjection(
  answer: ContinueTaskTruthAnswer,
  targetOpenable: boolean,
): ContinuePublicProjection {
  const task = publicClause(answer.task_summary);
  const currentStep = publicClause(answer.current_subtask);
  const action = publicClause(answer.next_action || answer.unfinished_state);
  const progress = publicClause(answer.last_meaningful_progress || answer.unfinished_state);
  const continuationField = buildContinueContinuationFieldProjection(
    answer,
    targetOpenable,
  );
  const surface = continuationField.locationLabel;
  const headline = currentStep || task || action || progress || "Continue";

  return {
    headline,
    memoryLine: null,
    resumeSurface: surface,
    openActionLabel: continuationField.openActionLabel,
    exactTargetNote: continuationField.targetStatus,
  };
}

export function buildContinueContinuationFieldProjection(
  answer: ContinueTaskTruthAnswer,
  targetOpenable: boolean,
  preciseTargetLabel?: string | null,
): ContinueContinuationFieldProjection {
  const checkpoint = publicSentence(answer.last_meaningful_progress) || null;
  const supportedContinuation = publicSentence(
    answer.unfinished_state || answer.next_action,
  );
  const executionState = normalizeToken(answer.execution_state);
  const explicitlyComplete = ["complete", "completed", "complete_or_idle"].includes(
    executionState,
  );
  const locationLabel = publicClause(preciseTargetLabel)
    || publicClause(answer.where_summary)
    || null;
  const projectedSurfaces = continuationSurfacesForPresentation(
    answer,
    preciseTargetLabel || answer.where_summary,
  );
  const genericAppDestination = /\b(?:the )?(?:running|current) app\b/i.test(
    supportedContinuation,
  );
  const taskNamesSmalltalk = [
    answer.task_summary,
    answer.task_object,
    answer.current_subtask,
  ].some((value) => normalizeToken(value || "").includes("smalltalk"));
  const continuationNamesSmalltalk = normalizeToken(supportedContinuation).includes("smalltalk");
  const continuationSurface = (genericAppDestination || continuationNamesSmalltalk)
    && taskNamesSmalltalk
    ? {
        label: "Smalltalk",
        kind: "App" as const,
        appLabel: "Smalltalk",
        siteHostname: null,
      }
    : projectedSurfaces.continuationSurface;
  const contextualContinuation = continuationSurface
    ? supportedContinuation
        .replace(/\bin the (?:running|current) app\b/gi, `in ${continuationSurface.label}`)
        .replace(/\bthe (?:running|current) app\b/gi, continuationSurface.label)
    : supportedContinuation;
  const continuation = contextualContinuation || (
    explicitlyComplete
      ? "No unfinished step remains."
      : "No unfinished step was clearly captured."
  );

  return {
    checkpoint,
    continuation,
    checkpointSurface: projectedSurfaces.checkpointSurface,
    continuationSurface,
    locationLabel,
    targetStatus: targetOpenable
      ? null
      : locationLabel
        ? "Exact place not captured"
        : "Return location not captured",
    openActionLabel: targetOpenable
      ? locationLabel ? `Open ${locationLabel}` : "Open continuation"
      : null,
    recentContext: recentContextForPresentation(answer),
  };
}

export function hasVisibleTaskTruthSemantics(
  answer?: ContinueTaskTruthAnswer | null,
) {
  return [
    answer?.task_summary,
    answer?.task_object,
    answer?.current_subtask,
    answer?.observed_surface,
    answer?.immediate_user_operation,
    answer?.semantic_effect_of_operation,
    answer?.last_meaningful_progress,
    answer?.unfinished_state,
    answer?.next_action,
    answer?.where_summary,
  ].some((value) => Boolean(value?.trim()));
}

export function hasVisibleTaskTruthContinuationDetails(
  answer?: ContinueTaskTruthAnswer | null,
) {
  return [
    answer?.last_meaningful_progress,
    answer?.unfinished_state,
    answer?.next_action,
    answer?.where_summary,
  ].some((value) => Boolean(value?.trim()));
}

function publicClause(value?: string | null) {
  return (value || "")
    .trim()
    .replace(/[\s.!?;:]+$/, "")
    .replace(/\s+/g, " ");
}

function publicSentence(value?: string | null) {
  return (value || "")
    .trim()
    .replace(/\s+/g, " ");
}

export function recentContextRoleLabel(
  role?: ContinueTaskTruthRecentContext["semantic_role"],
) {
  switch (role) {
    case "primary_work":
      return "Primary work";
    case "supporting_work":
      return "Supporting work";
    case "detour_or_unrelated":
      return "Detour or unrelated";
    case "unclear":
      return "Relationship unclear";
    default:
      return null;
  }
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
  const targetLine = input.taskKnown
    ? "The task is understood, but no exact return point is ready"
    : input.appFocusOnly
      ? "The app is known, but the exact page, conversation, or file is unavailable"
      : "No exact return point is ready";
  const targetMeta = input.targetNote?.trim() || (
    input.taskKnown
      ? "I know the task, but I do not have a direct page or file locator."
      : "The exact task location is unavailable from the current evidence."
  );
  return {
    targetBlockLabel: "Exact location unavailable",
    targetLine,
    targetMeta,
    actionLabel: "Try Continue again" as const,
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

  const explicitRequest = challengerTrigger === "manual" || challengerTrigger === "island";
  if (
    explicitRequest &&
    hasVisibleTaskTruthSemantics(authoritativeTaskTruthAnswer(incumbent)) &&
    isFailedEmptySemanticRefresh(challenger)
  ) {
    return {
      adopt: false,
      reasonCodes: [
        "rejected:explicit_refresh_failed_without_semantics",
        "retained:last_usable_continue_answer",
      ],
    };
  }

  const incumbentTask = adoptionTaskIdentity(incumbent);
  const challengerTask = adoptionTaskIdentity(challenger);
  const sameTask = Boolean(
    incumbentTask?.task_turn_id &&
      challengerTask?.task_turn_id &&
      incumbentTask.task_turn_id === challengerTask.task_turn_id,
  );
  const reasons: string[] = [];

  if (
    sameTask &&
    typeof incumbentTask?.revision === "number" &&
    typeof challengerTask?.revision === "number" &&
    challengerTask.revision < incumbentTask.revision
  ) {
    reasons.push("rejected:older_task_revision");
  }

  if (!explicitRequest) {
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

    // An island press is an explicit Continue request. Its full decision may
    // reach the React window through an event or startup hydration. Do not let
    // a later quiet refresh replace that exact provider-backed answer; another
    // explicit manual or island request is still allowed to replace it.
    if (
      incumbentTrigger === "island" &&
      hasProviderBackedVisibleTaskTruth(incumbent)
    ) {
      reasons.push("rejected:background_cannot_replace_explicit_island_answer");
    }

    if (
      (incumbentTrigger === "manual" || incumbentTrigger === "island") &&
      reasons.length > 0
    ) {
      reasons.push(
        incumbentTrigger === "island"
          ? "retained:explicit_island_result"
          : "retained:stronger_manual_result",
      );
    }
  }

  const reasonCodes = [...new Set(reasons)].slice(0, 12);
  if (reasonCodes.length > 0) {
    return { adopt: false, reasonCodes };
  }
  return {
    adopt: true,
    reasonCodes: [
      challengerTrigger === "manual"
        ? "adopted:explicit_manual_result"
        : challengerTrigger === "island"
          ? "adopted:explicit_island_result"
          : "adopted:quality_not_lower",
    ],
  };
}

function hasProviderBackedVisibleTaskTruth(
  decision: ContinueAdoptionComparableDecision,
) {
  const answer = authoritativeTaskTruthAnswer(decision);
  return Boolean(
    answer &&
      hasVisibleTaskTruthSemantics(answer) &&
      (answer.response_id?.trim() || answer.atomic_identity.model_response_id?.trim()),
  );
}

function isFailedEmptySemanticRefresh(
  decision: ContinueAdoptionComparableDecision,
) {
  const answer = decision.task_truth_v2?.answer || null;
  const diagnosticStatus = normalizeToken(
    decision.task_truth_v2?.inference_diagnostic?.status,
  );
  return Boolean(
    diagnosticStatus &&
      diagnosticStatus !== "success" &&
      !hasVisibleTaskTruthSemantics(answer),
  );
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
  if (taskTruth) {
    const atomic = taskTruth.atomic_identity;
    if (!hasCompleteTaskTruthAtomicIdentity(atomic)) return null;
    return {
      task_turn_id: atomic.task_thread_id as string,
      revision: atomic.task_thread_revision as number,
      latest_user_goal_summary: taskTruth?.task_summary,
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
