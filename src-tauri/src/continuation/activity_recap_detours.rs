use super::activity_recap::{
    ActivityCurrentState, ActivityDetourRole, ActivityDetourSummary, ActivityEvidenceAnchorType,
    ActivityEvidenceConfidence, ActivityEvidenceSource, ActivityEvidenceSpan, ActivitySupportRole,
    ActivitySupportSummary, ContinueActivityRecap,
};
use super::activity_recap_inputs::{ActivityRecapInputs, BranchContextFact, SupportEvidenceFact};
use super::activity_recap_segments::{
    ActivitySegmentPromotionState, ActivitySegmentRole, StitchedActivitySegment,
    StitchedActivityTimeline,
};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashSet;

const MAX_PUBLIC_DETOURS: usize = 3;
const MAX_PUBLIC_SUPPORT_ITEMS: usize = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivityDetourCategory {
    FileBrowsingDetour,
    PhotoBrowsingDetour,
    DocsSupport,
    SearchSupport,
    MessageInterrupt,
    DiagnosticSupport,
    TerminalSupportOutput,
    PromotedBranch,
    UnknownDetour,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DetourRecapResult {
    pub recent_detours: Vec<ActivityDetourSummary>,
    pub supporting_context: Vec<ActivitySupportSummary>,
    pub current_state: Option<ActivityCurrentState>,
    pub evidence_spans: Vec<ActivityEvidenceSpan>,
    pub missing_evidence: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct DetourCandidate {
    observed_at_ms: i64,
    priority: u8,
    identity: String,
    summary: ActivityDetourSummary,
    evidence_span: Option<ActivityEvidenceSpan>,
}

#[derive(Debug, Clone)]
struct SupportCandidate {
    observed_at_ms: i64,
    priority: u8,
    identity: String,
    summary: ActivitySupportSummary,
    evidence_span: Option<ActivityEvidenceSpan>,
}

/// Converts the already-stitched P5 timeline into public detour/support recap
/// items. P2 remains the only promotion authority: this function reads its
/// persisted state through the normalized segment role and never scores or
/// promotes a branch itself.
pub(crate) fn summarize_activity_detours(
    inputs: &ActivityRecapInputs,
    timeline: &StitchedActivityTimeline,
) -> DetourRecapResult {
    let mut result = DetourRecapResult::default();
    let mut detour_candidates = Vec::new();
    let mut support_candidates = Vec::new();

    for segment in &timeline.ordered_segments {
        let branch = latest_branch_for_segment(inputs, segment);
        if branch.is_none()
            && timeline.primary_segment.is_none()
            && segment.role == ActivitySegmentRole::CurrentFocusOnly
            && segment.app_name.is_none()
            && segment.surface_title.is_none()
        {
            continue;
        }
        let is_current = timeline
            .current_segment
            .as_ref()
            .is_some_and(|current| same_segment_identity(current, segment));
        let mut branch_like = branch.is_some()
            || matches!(
                segment.role,
                ActivitySegmentRole::Support
                    | ActivitySegmentRole::Detour
                    | ActivitySegmentRole::Interrupt
                    | ActivitySegmentRole::CurrentFocusOnly
            );
        let normalized_promoted = segment.role == ActivitySegmentRole::PromotedPrimary
            && segment.promotion_state == ActivitySegmentPromotionState::Promoted;
        let candidate_promoted = branch.is_none() && candidate_has_exact_promotion(inputs, segment);
        let promoted_branch = normalized_promoted
            && branch
                .map(branch_state_is_promoted)
                .unwrap_or(candidate_promoted);
        branch_like |= normalized_promoted;
        if !branch_like && !promoted_branch {
            continue;
        }

        let category = classify_detour_category(segment, branch, promoted_branch);
        let policy_known = branch.is_some_and(branch_state_is_known) || promoted_branch;
        let connected = branch.is_some_and(branch_has_origin);
        let role = public_detour_role(segment, branch, policy_known, connected);
        let reason = product_reason(category, segment, branch, role, is_current);
        let anchors = bounded_anchors(&segment.evidence_anchor_ids);
        if anchors.is_empty() {
            result.warnings.push(
                "A recent branch was omitted because it had no evidence anchors.".to_string(),
            );
            continue;
        }

        if !policy_known && branch_like {
            result.missing_evidence.push(
                "Promotion evidence for a recent branch is unavailable or unclear.".to_string(),
            );
        }
        if policy_known && branch.is_some() && !connected && category_requires_origin(category) {
            result.missing_evidence.push(
                "A recent support branch has no grounded link to the primary work.".to_string(),
            );
        }

        let item = ActivityDetourSummary {
            surface_title: segment.surface_title.clone(),
            app_name: segment.app_name.clone(),
            role,
            activity_label: Some(activity_label(category).to_string()),
            reason: reason.clone(),
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            confidence: if policy_known {
                segment.confidence
            } else {
                ActivityEvidenceConfidence::Low
            },
            evidence_anchor_ids: anchors.clone(),
        };
        let evidence_span = branch.map(|branch| ActivityEvidenceSpan {
            claim_key: "branch_policy".to_string(),
            claim_text: branch_policy_claim(branch, role),
            anchor_type: ActivityEvidenceAnchorType::Branch,
            anchor_ids: bounded_anchors(&branch_anchor_ids(branch)),
            confidence: item.confidence,
            source: ActivityEvidenceSource::Local,
        });

        if should_include_as_detour(segment, is_current, promoted_branch) {
            detour_candidates.push(DetourCandidate {
                observed_at_ms: segment.end_ms.or(segment.start_ms).unwrap_or(i64::MIN),
                priority: detour_priority(role, is_current),
                identity: segment_identity(segment, category),
                summary: item,
                evidence_span: evidence_span.clone(),
            });
        }

        if let Some(summary) = support_summary(category, segment, branch, role, connected, &anchors)
        {
            support_candidates.push(SupportCandidate {
                observed_at_ms: segment.end_ms.or(segment.start_ms).unwrap_or(i64::MIN),
                priority: support_priority(summary.role, promoted_branch),
                identity: segment_identity(segment, category),
                summary,
                evidence_span,
            });
        }
    }

    collect_support_only_evidence(inputs, &mut support_candidates, &mut result);
    detour_candidates.sort_by_key(|candidate| {
        (
            Reverse(candidate.priority),
            Reverse(candidate.observed_at_ms),
            candidate.identity.clone(),
        )
    });
    support_candidates.sort_by_key(|candidate| {
        (
            Reverse(candidate.priority),
            Reverse(candidate.observed_at_ms),
            candidate.identity.clone(),
        )
    });

    let mut seen_detours = HashSet::new();
    for candidate in detour_candidates {
        if result.recent_detours.len() >= MAX_PUBLIC_DETOURS {
            break;
        }
        if seen_detours.insert(candidate.identity) {
            result.recent_detours.push(candidate.summary);
            push_optional_span(&mut result.evidence_spans, candidate.evidence_span);
        }
    }

    let mut seen_support = HashSet::new();
    for candidate in support_candidates {
        if result.supporting_context.len() >= MAX_PUBLIC_SUPPORT_ITEMS {
            break;
        }
        if seen_support.insert(candidate.identity) {
            result.supporting_context.push(candidate.summary);
            push_optional_span(&mut result.evidence_spans, candidate.evidence_span);
        }
    }

    result.current_state =
        timeline
            .current_segment
            .as_ref()
            .and_then(|current| match current.role {
                ActivitySegmentRole::Primary
                | ActivitySegmentRole::Return
                | ActivitySegmentRole::PromotedPrimary => {
                    Some(ActivityCurrentState::ActivelyWorking)
                }
                ActivitySegmentRole::Support
                | ActivitySegmentRole::Detour
                | ActivitySegmentRole::Interrupt
                | ActivitySegmentRole::CurrentFocusOnly
                    if timeline
                        .primary_segment
                        .as_ref()
                        .is_some_and(|primary| !same_segment_identity(primary, current)) =>
                {
                    Some(ActivityCurrentState::RecentlyDetoured)
                }
                _ => None,
            });
    dedupe_strings(&mut result.missing_evidence);
    dedupe_strings(&mut result.warnings);
    result
}

/// Applies only P5-05-owned recap fields. Primary-work labels, target confidence,
/// target explanations, open-loop state, and target selection are left intact.
pub(crate) fn apply_activity_detour_recap(
    mut recap: ContinueActivityRecap,
    detours: DetourRecapResult,
) -> ContinueActivityRecap {
    recap.recent_detours = detours.recent_detours;
    recap.supporting_context = detours.supporting_context;
    if let Some(current_state) = detours.current_state {
        recap.current_state = current_state;
    }
    recap.evidence_spans.extend(detours.evidence_spans);
    recap.missing_evidence.extend(detours.missing_evidence);
    recap.warnings.extend(detours.warnings);
    dedupe_strings(&mut recap.missing_evidence);
    dedupe_strings(&mut recap.warnings);
    dedupe_evidence_spans(&mut recap.evidence_spans);
    recap.sanitized()
}

fn latest_branch_for_segment<'a>(
    inputs: &'a ActivityRecapInputs,
    segment: &StitchedActivitySegment,
) -> Option<&'a BranchContextFact> {
    let artifact_id = segment.artifact_id.as_deref()?;
    inputs
        .branch_contexts
        .iter()
        .filter(|branch| branch.branch_artifact_id == artifact_id)
        .max_by(|left, right| {
            left.last_branch_seen_at_ms
                .cmp(&right.last_branch_seen_at_ms)
                .then_with(|| left.updated_at_ms.cmp(&right.updated_at_ms))
                .then_with(|| left.branch_id.cmp(&right.branch_id))
        })
}

fn classify_detour_category(
    segment: &StitchedActivitySegment,
    branch: Option<&BranchContextFact>,
    promoted_branch: bool,
) -> ActivityDetourCategory {
    if promoted_branch {
        return ActivityDetourCategory::PromotedBranch;
    }
    match branch.map(|branch| branch.branch_kind.as_str()) {
        Some("documentation_reference" | "source_evidence") => ActivityDetourCategory::DocsSupport,
        Some("search_branch") => ActivityDetourCategory::SearchSupport,
        Some("message_interrupt" | "messaging_interrupt" | "interrupt") => {
            ActivityDetourCategory::MessageInterrupt
        }
        Some("diagnostic_self" | "diagnostic_only") => ActivityDetourCategory::DiagnosticSupport,
        Some("terminal_support_output") => ActivityDetourCategory::TerminalSupportOutput,
        Some("tool_or_agent_output" | "verification_branch")
            if segment.app_name.as_deref().is_some_and(|app| {
                contains_any(&app.to_ascii_lowercase(), &["terminal", "iterm", "warp"])
            }) =>
        {
            ActivityDetourCategory::TerminalSupportOutput
        }
        Some("tool_or_agent_output" | "verification_branch") => {
            ActivityDetourCategory::UnknownDetour
        }
        _ => classify_surface_category(segment),
    }
}

fn classify_surface_category(segment: &StitchedActivitySegment) -> ActivityDetourCategory {
    let app = segment
        .app_name
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let title = segment
        .surface_title
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kinds = segment.activity_kinds.join(" ").to_ascii_lowercase();
    let surface = format!("{app} {title} {kinds}");
    if contains_any(&surface, &["photos", "photo", "pictures", "image gallery"]) {
        ActivityDetourCategory::PhotoBrowsingDetour
    } else if contains_any(&app, &["finder", "file explorer", "files"]) {
        ActivityDetourCategory::FileBrowsingDetour
    } else if contains_any(&app, &["gmail", "mail", "messages", "slack"]) {
        ActivityDetourCategory::MessageInterrupt
    } else if contains_any(
        &surface,
        &[
            "smalltalk diagnostics",
            "developer diagnostics",
            "inspect evidence",
        ],
    ) {
        ActivityDetourCategory::DiagnosticSupport
    } else if contains_any(&app, &["terminal", "iterm", "warp"]) {
        ActivityDetourCategory::TerminalSupportOutput
    } else if contains_any(&surface, &["documentation", " docs", "reference"]) {
        ActivityDetourCategory::DocsSupport
    } else if contains_any(&surface, &["search", "results"]) {
        ActivityDetourCategory::SearchSupport
    } else {
        ActivityDetourCategory::UnknownDetour
    }
}

fn public_detour_role(
    segment: &StitchedActivitySegment,
    branch: Option<&BranchContextFact>,
    policy_known: bool,
    connected: bool,
) -> ActivityDetourRole {
    if !policy_known
        && !matches!(
            segment.role,
            ActivitySegmentRole::Primary | ActivitySegmentRole::Return
        )
    {
        return if segment.role == ActivitySegmentRole::CurrentFocusOnly {
            ActivityDetourRole::CurrentFocusOnly
        } else {
            ActivityDetourRole::Unclear
        };
    }
    if segment.role == ActivitySegmentRole::PromotedPrimary
        && branch.is_some_and(|branch| !branch_state_is_promoted(branch))
    {
        return fallback_unpromoted_role(branch);
    }
    if segment.role == ActivitySegmentRole::Support && !connected {
        return ActivityDetourRole::Detour;
    }
    match segment.role {
        ActivitySegmentRole::Support => ActivityDetourRole::Support,
        ActivitySegmentRole::Detour => ActivityDetourRole::Detour,
        ActivitySegmentRole::Interrupt => ActivityDetourRole::Interrupt,
        ActivitySegmentRole::CurrentFocusOnly => ActivityDetourRole::CurrentFocusOnly,
        ActivitySegmentRole::PromotedPrimary => ActivityDetourRole::PromotedPrimary,
        _ => ActivityDetourRole::Unclear,
    }
}

fn product_reason(
    category: ActivityDetourCategory,
    segment: &StitchedActivitySegment,
    branch: Option<&BranchContextFact>,
    role: ActivityDetourRole,
    is_current: bool,
) -> String {
    if role == ActivityDetourRole::Unclear {
        return if is_current {
            "The latest surface differed from the primary work, but its branch relation and promotion evidence are unclear."
                .to_string()
        } else {
            "This recent surface differed from the primary work, but its branch relation and promotion evidence are unclear."
                .to_string()
        };
    }
    if role == ActivityDetourRole::PromotedPrimary {
        return if branch.is_some_and(|branch| branch.promotion_state == "promoted_blocker") {
            if branch.is_some_and(|branch| branch.branch_kind == "terminal_support_output")
                || segment.app_name.as_deref().is_some_and(|app| {
                    contains_any(&app.to_ascii_lowercase(), &["terminal", "iterm", "warp"])
                })
            {
                "Terminal output showed an unresolved blocker and became primary because the error remained unresolved."
                    .to_string()
            } else {
                "This branch contained an unresolved blocker and became primary because the blocker remained unresolved."
                    .to_string()
            }
        } else {
            "This branch became primary after explicit local promotion evidence was recorded."
                .to_string()
        };
    }

    if branch.is_some_and(|branch| branch.promotion_state.starts_with("blocked_")) {
        let surface = match category {
            ActivityDetourCategory::FileBrowsingDetour
            | ActivityDetourCategory::PhotoBrowsingDetour => "browsing surface",
            ActivityDetourCategory::DocsSupport => "documentation branch",
            ActivityDetourCategory::SearchSupport => "search branch",
            ActivityDetourCategory::MessageInterrupt => "message surface",
            ActivityDetourCategory::DiagnosticSupport => "diagnostic surface",
            ActivityDetourCategory::TerminalSupportOutput => "terminal surface",
            _ => "branch",
        };
        return if is_current {
            format!(
                "The latest surface was a {surface}, but existing branch policy kept it as evidence rather than primary work."
            )
        } else {
            format!(
                "Existing branch policy kept this {surface} as evidence rather than primary work."
            )
        };
    }

    if branch.is_some_and(|branch| !branch_has_origin(branch)) && category_requires_origin(category)
    {
        let surface = match category {
            ActivityDetourCategory::DocsSupport => "documentation",
            ActivityDetourCategory::SearchSupport => "search",
            ActivityDetourCategory::DiagnosticSupport => "diagnostic",
            ActivityDetourCategory::TerminalSupportOutput => "terminal or output",
            _ => "support",
        };
        return if is_current {
            format!(
                "The latest surface was {surface}, but no grounded link to the primary work was found."
            )
        } else {
            format!(
                "This recent {surface} surface had no grounded link to the primary work and remained a detour."
            )
        };
    }

    let latest_prefix = is_current.then(|| {
        let surface = match category {
            ActivityDetourCategory::FileBrowsingDetour => {
                segment.app_name.as_deref().unwrap_or("file browsing")
            }
            ActivityDetourCategory::PhotoBrowsingDetour => {
                segment.app_name.as_deref().unwrap_or("photo browsing")
            }
            ActivityDetourCategory::DocsSupport => "documentation",
            ActivityDetourCategory::SearchSupport => "search",
            ActivityDetourCategory::MessageInterrupt => "messages",
            ActivityDetourCategory::DiagnosticSupport => "diagnostics",
            ActivityDetourCategory::TerminalSupportOutput => "terminal output",
            _ => segment.app_name.as_deref().unwrap_or("recent surface"),
        };
        format!("The latest surface was {surface}, but ")
    });

    let body = match category {
        ActivityDetourCategory::PhotoBrowsingDetour => {
            if segment_is_brief(segment) {
                "it looked like brief photo browsing; no direct file-work promotion evidence was recorded."
            } else {
                "it looked like photo browsing; no direct file-work promotion evidence was recorded."
            }
        }
        ActivityDetourCategory::FileBrowsingDetour => {
            if segment_is_brief(segment) {
                "it looked like brief file browsing; no direct file-work promotion evidence was recorded."
            } else {
                "it looked like file browsing; no direct file-work promotion evidence was recorded."
            }
        }
        ActivityDetourCategory::DocsSupport => {
            "the documentation branch supported the primary work and did not become the continuation target."
        }
        ActivityDetourCategory::SearchSupport => {
            "the search branch supported the primary work and did not become the continuation target."
        }
        ActivityDetourCategory::MessageInterrupt => {
            "it was a message interruption, and no promotion evidence made it continuation work."
        }
        ActivityDetourCategory::DiagnosticSupport => {
            "the diagnostics were inspected as evidence, not as the primary work."
        }
        ActivityDetourCategory::TerminalSupportOutput => {
            "the terminal output supported the primary work without promoted blocker evidence."
        }
        ActivityDetourCategory::PromotedBranch => unreachable!("handled above"),
        ActivityDetourCategory::UnknownDetour => {
            "the evidence identifies it as a recent detour rather than primary work."
        }
    };
    latest_prefix.map_or_else(
        || uppercase_first(body.trim_start_matches("it ")),
        |prefix| format!("{prefix}{body}"),
    )
}

fn support_summary(
    category: ActivityDetourCategory,
    segment: &StitchedActivitySegment,
    branch: Option<&BranchContextFact>,
    role: ActivityDetourRole,
    connected: bool,
    anchors: &[String],
) -> Option<ActivitySupportSummary> {
    if !connected
        && !matches!(
            category,
            ActivityDetourCategory::DiagnosticSupport | ActivityDetourCategory::PromotedBranch
        )
    {
        return None;
    }
    if category == ActivityDetourCategory::DiagnosticSupport
        && !connected
        && !is_confirmed_smalltalk_diagnostic(segment, branch)
    {
        return None;
    }
    let (support_role, summary) = match category {
        ActivityDetourCategory::DocsSupport => (
            if branch.is_some_and(|branch| branch.branch_kind == "source_evidence") {
                ActivitySupportRole::SourceEvidence
            } else {
                ActivitySupportRole::BranchSupport
            },
            "Documentation or source material supported the primary work without replacing it.",
        ),
        ActivityDetourCategory::SearchSupport => (
            ActivitySupportRole::BranchSupport,
            "Search results supported the primary work without becoming the return target.",
        ),
        ActivityDetourCategory::MessageInterrupt => (
            ActivitySupportRole::MessageInterrupt,
            "A message interruption was observed and remained non-primary under existing branch evidence.",
        ),
        ActivityDetourCategory::DiagnosticSupport => (
            ActivitySupportRole::Diagnostic,
            if is_confirmed_smalltalk_diagnostic(segment, branch) {
                "Smalltalk diagnostics were inspected as evidence, not as the primary work."
            } else {
                "Diagnostics were inspected as evidence, not as the primary work."
            },
        ),
        ActivityDetourCategory::TerminalSupportOutput
            if role == ActivityDetourRole::PromotedPrimary
                && branch.is_some_and(|branch| branch.promotion_state == "promoted_blocker") =>
        {
            (
                ActivitySupportRole::Blocker,
                "Terminal output contained the unresolved blocker that was promoted to primary work.",
            )
        }
        ActivityDetourCategory::TerminalSupportOutput => (
            ActivitySupportRole::OutputVerification,
            "Terminal output supported or verified the primary work without becoming its return target.",
        ),
        ActivityDetourCategory::PromotedBranch
            if branch.is_some_and(|branch| branch.promotion_state == "promoted_blocker") =>
        {
            (
                ActivitySupportRole::Blocker,
                "The branch contained the unresolved blocker that became primary work.",
            )
        }
        ActivityDetourCategory::UnknownDetour
            if segment.role == ActivitySegmentRole::Support && connected =>
        {
            (
                ActivitySupportRole::Unknown,
                "A connected support branch provided context without replacing the primary work.",
            )
        }
        _ => return None,
    };
    Some(ActivitySupportSummary {
        summary: summary.to_string(),
        role: support_role,
        confidence: segment.confidence,
        evidence_anchor_ids: anchors.to_vec(),
    })
}

fn collect_support_only_evidence(
    inputs: &ActivityRecapInputs,
    candidates: &mut Vec<SupportCandidate>,
    result: &mut DetourRecapResult,
) {
    for support in &inputs.support_evidence {
        let Some(category) = support_only_category(support) else {
            continue;
        };
        if support.public_return_eligible {
            continue;
        }
        if support_only_requires_origin(category) && support.origin_artifact_id.is_none() {
            result.missing_evidence.push(
                "A support-only surface was omitted because it has no grounded primary-work link."
                    .to_string(),
            );
            continue;
        }
        let anchors = bounded_anchors(&support.evidence_anchor_ids);
        if anchors.is_empty() {
            result.warnings.push(
                "A support-only diagnostic was omitted because it had no evidence anchor."
                    .to_string(),
            );
            continue;
        }
        let diagnostic_summary = if support.branch_kind == "diagnostic_self" {
            "Smalltalk diagnostics were inspected as evidence, not as the primary work."
        } else {
            "Diagnostics were inspected as evidence, not as the primary work."
        };
        let (role, summary) = match category {
            ActivityDetourCategory::DiagnosticSupport => {
                (ActivitySupportRole::Diagnostic, diagnostic_summary)
            }
            ActivityDetourCategory::DocsSupport => (
                ActivitySupportRole::BranchSupport,
                "Documentation supported the primary work without replacing it.",
            ),
            ActivityDetourCategory::SearchSupport => (
                ActivitySupportRole::BranchSupport,
                "Search results supported the primary work without becoming the return target.",
            ),
            ActivityDetourCategory::MessageInterrupt => (
                ActivitySupportRole::MessageInterrupt,
                "A message interruption was observed without continuation evidence.",
            ),
            ActivityDetourCategory::TerminalSupportOutput => (
                ActivitySupportRole::OutputVerification,
                "Terminal output supported the primary work without becoming its return target.",
            ),
            ActivityDetourCategory::UnknownDetour => (
                ActivitySupportRole::Unknown,
                "A connected support branch provided context without replacing the primary work.",
            ),
            _ => continue,
        };
        let identity_anchor = support
            .artifact_id
            .as_deref()
            .unwrap_or_else(|| anchors[0].as_str());
        candidates.push(SupportCandidate {
            observed_at_ms: i64::MIN,
            priority: 0,
            identity: format!("{}:{identity_anchor}", claim_key(category)),
            summary: ActivitySupportSummary {
                summary: summary.to_string(),
                role,
                confidence: ActivityEvidenceConfidence::Low,
                evidence_anchor_ids: anchors,
            },
            evidence_span: None,
        });
    }
}

fn support_only_category(support: &SupportEvidenceFact) -> Option<ActivityDetourCategory> {
    let value = format!("{} {}", support.branch_kind, support.role).to_ascii_lowercase();
    if contains_any(&value, &["diagnostic", "smalltalk_self"]) {
        Some(ActivityDetourCategory::DiagnosticSupport)
    } else if contains_any(&value, &["documentation", "source_evidence", "docs"]) {
        Some(ActivityDetourCategory::DocsSupport)
    } else if value.contains("search") {
        Some(ActivityDetourCategory::SearchSupport)
    } else if contains_any(&value, &["message", "gmail", "interrupt"]) {
        Some(ActivityDetourCategory::MessageInterrupt)
    } else if value.contains("terminal_support") || value.contains("terminal") {
        Some(ActivityDetourCategory::TerminalSupportOutput)
    } else if contains_any(&value, &["verification", "tool_or_agent", "output"]) {
        Some(ActivityDetourCategory::UnknownDetour)
    } else {
        None
    }
}

fn should_include_as_detour(
    segment: &StitchedActivitySegment,
    is_current: bool,
    promoted_branch: bool,
) -> bool {
    is_current
        || promoted_branch
        || matches!(
            segment.role,
            ActivitySegmentRole::Detour
                | ActivitySegmentRole::Interrupt
                | ActivitySegmentRole::CurrentFocusOnly
        )
}

fn detour_priority(role: ActivityDetourRole, is_current: bool) -> u8 {
    if is_current {
        return 10;
    }
    match role {
        ActivityDetourRole::PromotedPrimary => 9,
        ActivityDetourRole::Interrupt => 8,
        ActivityDetourRole::Detour => 7,
        ActivityDetourRole::CurrentFocusOnly => 6,
        ActivityDetourRole::Support => 5,
        ActivityDetourRole::Unclear => 4,
    }
}

fn support_priority(role: ActivitySupportRole, promoted_branch: bool) -> u8 {
    if promoted_branch || role == ActivitySupportRole::Blocker {
        return 10;
    }
    match role {
        ActivitySupportRole::Diagnostic => 8,
        ActivitySupportRole::MessageInterrupt => 7,
        ActivitySupportRole::BranchSupport | ActivitySupportRole::SourceEvidence => 6,
        ActivitySupportRole::OutputVerification => 5,
        ActivitySupportRole::Unknown => 1,
        ActivitySupportRole::Blocker => 10,
    }
}

fn activity_label(category: ActivityDetourCategory) -> &'static str {
    match category {
        ActivityDetourCategory::FileBrowsingDetour => "File browsing",
        ActivityDetourCategory::PhotoBrowsingDetour => "Photo browsing",
        ActivityDetourCategory::DocsSupport => "Documentation support",
        ActivityDetourCategory::SearchSupport => "Search support",
        ActivityDetourCategory::MessageInterrupt => "Message interruption",
        ActivityDetourCategory::DiagnosticSupport => "Diagnostic inspection",
        ActivityDetourCategory::TerminalSupportOutput => "Terminal support output",
        ActivityDetourCategory::PromotedBranch => "Promoted branch work",
        ActivityDetourCategory::UnknownDetour => "Recent detour",
    }
}

fn claim_key(category: ActivityDetourCategory) -> &'static str {
    match category {
        ActivityDetourCategory::FileBrowsingDetour => "file_browsing_detour",
        ActivityDetourCategory::PhotoBrowsingDetour => "photo_browsing_detour",
        ActivityDetourCategory::DocsSupport => "docs_support",
        ActivityDetourCategory::SearchSupport => "search_support",
        ActivityDetourCategory::MessageInterrupt => "message_interrupt",
        ActivityDetourCategory::DiagnosticSupport => "diagnostic_support",
        ActivityDetourCategory::TerminalSupportOutput => "terminal_support_output",
        ActivityDetourCategory::PromotedBranch => "promoted_branch",
        ActivityDetourCategory::UnknownDetour => "unknown_detour",
    }
}

fn segment_identity(segment: &StitchedActivitySegment, category: ActivityDetourCategory) -> String {
    format!(
        "{}:{}",
        claim_key(category),
        segment
            .artifact_id
            .as_deref()
            .unwrap_or(&segment.segment_id)
    )
}

fn same_segment_identity(left: &StitchedActivitySegment, right: &StitchedActivitySegment) -> bool {
    left.segment_id == right.segment_id
}

fn branch_state_is_known(branch: &BranchContextFact) -> bool {
    branch.promotion_state == "unpromoted"
        || branch_state_is_promoted(branch)
        || matches!(
            branch.promotion_state.as_str(),
            "blocked_diagnostic_self"
                | "blocked_feedback_suppressed"
                | "blocked_thin_current_focus"
        )
}

fn branch_state_is_promoted(branch: &BranchContextFact) -> bool {
    matches!(
        branch.promotion_state.as_str(),
        "promoted_primary"
            | "promoted_blocker"
            | "promoted_user_corrected"
            | "promoted_user_accepted"
            | "promoted_sustained_work"
    )
}

fn branch_has_origin(branch: &BranchContextFact) -> bool {
    branch.origin_artifact_id.is_some() && branch.reason_code.as_deref() != Some("branch:no_origin")
}

fn candidate_has_exact_promotion(
    inputs: &ActivityRecapInputs,
    segment: &StitchedActivitySegment,
) -> bool {
    inputs.selected_candidate.as_ref().is_some_and(|candidate| {
        let identity_matches = candidate.activity_segment_id.as_deref()
            == Some(segment.segment_id.as_str())
            || segment.artifact_id.as_deref().is_some_and(|artifact_id| {
                candidate.target_artifact_id.as_deref() == Some(artifact_id)
            });
        identity_matches
            && candidate.branch_public_return_eligible == Some(true)
            && matches!(
                candidate.branch_promotion_state.as_deref(),
                Some(
                    "promoted_primary"
                        | "promoted_blocker"
                        | "promoted_user_corrected"
                        | "promoted_user_accepted"
                        | "promoted_sustained_work"
                )
            )
    })
}

fn fallback_unpromoted_role(branch: Option<&BranchContextFact>) -> ActivityDetourRole {
    match branch.map(|branch| branch.branch_kind.as_str()) {
        Some("message_interrupt" | "messaging_interrupt" | "interrupt") => {
            ActivityDetourRole::Interrupt
        }
        Some("current_focus_only") => ActivityDetourRole::CurrentFocusOnly,
        Some(
            "search_branch"
            | "documentation_reference"
            | "source_evidence"
            | "terminal_support_output"
            | "tool_or_agent_output"
            | "verification_branch"
            | "diagnostic_self"
            | "unknown_support",
        ) if branch.is_some_and(branch_has_origin) => ActivityDetourRole::Support,
        _ => ActivityDetourRole::Detour,
    }
}

fn category_requires_origin(category: ActivityDetourCategory) -> bool {
    matches!(
        category,
        ActivityDetourCategory::DocsSupport
            | ActivityDetourCategory::SearchSupport
            | ActivityDetourCategory::DiagnosticSupport
            | ActivityDetourCategory::TerminalSupportOutput
    )
}

fn support_only_requires_origin(category: ActivityDetourCategory) -> bool {
    matches!(
        category,
        ActivityDetourCategory::DocsSupport
            | ActivityDetourCategory::SearchSupport
            | ActivityDetourCategory::TerminalSupportOutput
            | ActivityDetourCategory::UnknownDetour
    )
}

fn segment_is_brief(segment: &StitchedActivitySegment) -> bool {
    match (segment.start_ms, segment.end_ms) {
        (Some(start), Some(end)) => end.saturating_sub(start) <= 2 * 60 * 1000,
        _ => false,
    }
}

fn is_confirmed_smalltalk_diagnostic(
    segment: &StitchedActivitySegment,
    branch: Option<&BranchContextFact>,
) -> bool {
    branch.is_some_and(|branch| branch.branch_kind == "diagnostic_self")
        || segment
            .app_name
            .as_deref()
            .is_some_and(|app| app.to_ascii_lowercase().contains("smalltalk"))
}

fn branch_anchor_ids(branch: &BranchContextFact) -> Vec<String> {
    vec![branch.branch_id.clone()]
}

fn branch_policy_claim(branch: &BranchContextFact, role: ActivityDetourRole) -> String {
    if role == ActivityDetourRole::PromotedPrimary && branch_state_is_promoted(branch) {
        "Existing branch promotion state makes this branch primary-eligible.".to_string()
    } else if branch.promotion_state.starts_with("blocked_") {
        "Existing branch policy blocks this branch from becoming primary work.".to_string()
    } else if branch.promotion_state == "unpromoted" {
        "Existing branch state keeps this branch non-primary.".to_string()
    } else {
        "The branch relation is recorded, but its promotion state is unclear.".to_string()
    }
}

fn bounded_anchors(anchors: &[String]) -> Vec<String> {
    let mut output = Vec::new();
    for anchor in anchors {
        let anchor = anchor.trim();
        if !anchor.is_empty() && !output.iter().any(|value| value == anchor) {
            output.push(anchor.to_string());
        }
        if output.len() >= 16 {
            break;
        }
    }
    output
}

fn push_optional_span(spans: &mut Vec<ActivityEvidenceSpan>, span: Option<ActivityEvidenceSpan>) {
    let Some(span) = span else {
        return;
    };
    if !span.anchor_ids.is_empty() {
        spans.push(span);
    }
}

fn dedupe_evidence_spans(spans: &mut Vec<ActivityEvidenceSpan>) {
    let mut seen = HashSet::new();
    spans.retain(|span| {
        seen.insert(format!(
            "{}:{:?}:{}",
            span.claim_key,
            span.anchor_type,
            span.anchor_ids.join("|")
        ))
    });
}

fn dedupe_strings(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn uppercase_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::activity_recap::{ActivityConfidence, ActivityRecapValidationStatus};
    use super::super::activity_recap_inputs::{
        ActivityRecapDecisionContext, CandidateFact, ExistingQualityFacts,
    };
    use super::super::activity_recap_segments::ActivityPrimaryContinuity;
    use super::*;

    fn empty_inputs() -> ActivityRecapInputs {
        ActivityRecapInputs {
            schema: "smalltalk.activity_recap_inputs.v1".to_string(),
            current_task_turn: None,
            decision_context: ActivityRecapDecisionContext {
                decision_id_seed: Some("decision-detour-test".to_string()),
                mode: "normal".to_string(),
                lookback_ms: 10_000,
                evidence_watermark: Some("watermark-test".to_string()),
                output_mode: Some("continue_ready".to_string()),
            },
            current_surface: None,
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
            memory_facts: Vec::new(),
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

    fn stitched_segment(
        id: &str,
        app_name: &str,
        title: &str,
        artifact_id: &str,
        role: ActivitySegmentRole,
        promotion_state: ActivitySegmentPromotionState,
        at_ms: i64,
    ) -> StitchedActivitySegment {
        StitchedActivitySegment {
            segment_id: id.to_string(),
            start_ms: Some(at_ms),
            end_ms: Some(at_ms + 10),
            app_name: Some(app_name.to_string()),
            surface_title: Some(title.to_string()),
            artifact_kind: None,
            workstream_id: Some("ws-primary".to_string()),
            artifact_id: Some(artifact_id.to_string()),
            role,
            activity_kinds: Vec::new(),
            local_reason: "local test reason".to_string(),
            promotion_state,
            confidence: ActivityEvidenceConfidence::High,
            evidence_anchor_ids: vec![format!("segment-{id}"), format!("action-{id}")],
        }
    }

    fn branch(
        id: &str,
        origin_artifact_id: Option<&str>,
        branch_artifact_id: &str,
        branch_kind: &str,
        promotion_state: &str,
        at_ms: i64,
    ) -> BranchContextFact {
        BranchContextFact {
            branch_id: format!("branch-{id}"),
            branch_action_id: format!("branch-action-{id}"),
            task_turn_id: None,
            origin_task_turn_id: None,
            promotion_task_turn_id: None,
            promoted_at_ms: None,
            origin_artifact_id: origin_artifact_id.map(str::to_string),
            origin_workstream_id: origin_artifact_id.map(|_| "ws-primary".to_string()),
            branch_artifact_id: branch_artifact_id.to_string(),
            branch_kind: branch_kind.to_string(),
            branch_started_at_ms: at_ms,
            last_branch_seen_at_ms: at_ms + 10,
            returned_to_origin_at_ms: None,
            promotion_state: promotion_state.to_string(),
            promotion_reason: Some("internal reason must not be exposed".to_string()),
            confidence: 0.9,
            reason_code: Some("branch:internal".to_string()),
            evidence_action_ids: vec![format!("promotion-action-{id}")],
            promotion_evidence_action_ids: Vec::new(),
            eligible_feedback_event_ids: Vec::new(),
            feedback_rejection_reasons: Vec::new(),
            updated_at_ms: at_ms + 20,
        }
    }

    fn timeline(
        primary: Option<StitchedActivitySegment>,
        current: Option<StitchedActivitySegment>,
        ordered_segments: Vec<StitchedActivitySegment>,
    ) -> StitchedActivityTimeline {
        StitchedActivityTimeline {
            primary_segment: primary,
            current_segment: current,
            ordered_segments,
            recent_detours: Vec::new(),
            support_segments: Vec::new(),
            interruptions: Vec::new(),
            returned_to_primary: false,
            primary_continuity: ActivityPrimaryContinuity::Unclear,
            confidence: ActivityConfidence::High,
            warnings: Vec::new(),
        }
    }

    fn primary_chat() -> StitchedActivitySegment {
        stitched_segment(
            "chat",
            "ChatGPT",
            "Smalltalk research chat",
            "chat-artifact",
            ActivitySegmentRole::Primary,
            ActivitySegmentPromotionState::NotApplicable,
            10,
        )
    }

    #[test]
    fn chat_primary_and_finder_photos_produce_a_non_promoted_detour() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let photos = stitched_segment(
            "photos",
            "Finder",
            "Photos",
            "photo-artifact",
            ActivitySegmentRole::Detour,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        inputs.branch_contexts.push(branch(
            "photos",
            Some("chat-artifact"),
            "photo-artifact",
            "unrelated_browsing",
            "unpromoted",
            30,
        ));

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(primary.clone()),
                vec![primary, photos],
            ),
        );

        assert_eq!(result.recent_detours.len(), 1);
        assert_eq!(result.recent_detours[0].role, ActivityDetourRole::Detour);
        assert!(result.recent_detours[0]
            .reason
            .to_ascii_lowercase()
            .contains("photo browsing"));
        assert!(result.recent_detours[0]
            .evidence_anchor_ids
            .contains(&"action-photos".to_string()));
        assert!(result.supporting_context.is_empty());
        assert_eq!(
            result.current_state,
            Some(ActivityCurrentState::ActivelyWorking)
        );
    }

    #[test]
    fn editor_docs_and_search_branches_remain_supporting_context() {
        let mut inputs = empty_inputs();
        let editor = stitched_segment(
            "editor",
            "Visual Studio Code",
            "Smalltalk source",
            "editor-artifact",
            ActivitySegmentRole::Primary,
            ActivitySegmentPromotionState::NotApplicable,
            10,
        );
        let docs = stitched_segment(
            "docs",
            "Safari",
            "API documentation",
            "docs-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        let search = stitched_segment(
            "search",
            "Safari",
            "Search results",
            "search-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotPromoted,
            50,
        );
        inputs.branch_contexts.extend([
            branch(
                "docs",
                Some("editor-artifact"),
                "docs-artifact",
                "documentation_reference",
                "unpromoted",
                30,
            ),
            branch(
                "search",
                Some("editor-artifact"),
                "search-artifact",
                "search_branch",
                "unpromoted",
                50,
            ),
        ]);
        inputs.support_evidence.push(SupportEvidenceFact {
            artifact_id: Some("docs-artifact".to_string()),
            artifact_kind: Some("browser_tab".to_string()),
            display_title: Some("API documentation".to_string()),
            branch_kind: "documentation_reference".to_string(),
            origin_artifact_id: Some("editor-artifact".to_string()),
            role: "support".to_string(),
            public_return_eligible: false,
            reason: Some("internal support reason".to_string()),
            evidence_anchor_ids: vec!["support-docs-action".to_string()],
        });

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(editor.clone()),
                Some(editor.clone()),
                vec![editor, docs, search],
            ),
        );

        assert_eq!(result.supporting_context.len(), 2);
        assert!(result
            .supporting_context
            .iter()
            .all(|item| item.role == ActivitySupportRole::BranchSupport));
        assert!(result
            .supporting_context
            .iter()
            .any(|item| item.summary.contains("Documentation")));
        assert!(result
            .supporting_context
            .iter()
            .any(|item| item.summary.contains("Search")));
        assert!(result.recent_detours.is_empty());
        assert!(result
            .supporting_context
            .iter()
            .all(|item| !item.evidence_anchor_ids.is_empty()));
        assert!(result.evidence_spans.iter().all(|span| {
            span.anchor_type == ActivityEvidenceAnchorType::Branch
                && span.anchor_ids.len() == 1
                && span.anchor_ids[0].starts_with("branch-")
        }));
    }

    #[test]
    fn gmail_is_explained_as_an_interrupt_without_promotion() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let gmail = stitched_segment(
            "gmail",
            "Gmail",
            "Inbox",
            "gmail-artifact",
            ActivitySegmentRole::Interrupt,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        inputs.branch_contexts.push(branch(
            "gmail",
            Some("chat-artifact"),
            "gmail-artifact",
            "message_interrupt",
            "unpromoted",
            30,
        ));

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(gmail.clone()),
                vec![primary, gmail],
            ),
        );

        assert_eq!(result.recent_detours[0].role, ActivityDetourRole::Interrupt);
        assert!(result.recent_detours[0]
            .reason
            .contains("message interruption"));
        assert_eq!(
            result.supporting_context[0].role,
            ActivitySupportRole::MessageInterrupt
        );
        assert_eq!(
            result.current_state,
            Some(ActivityCurrentState::RecentlyDetoured)
        );
    }

    #[test]
    fn terminal_blocker_becomes_primary_only_when_p2_state_is_promoted() {
        let mut inputs = empty_inputs();
        let editor = stitched_segment(
            "editor",
            "Visual Studio Code",
            "Smalltalk source",
            "editor-artifact",
            ActivitySegmentRole::Primary,
            ActivitySegmentPromotionState::NotApplicable,
            10,
        );
        let terminal_support = stitched_segment(
            "terminal",
            "Terminal",
            "Test output",
            "terminal-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        inputs.branch_contexts.push(branch(
            "terminal",
            Some("editor-artifact"),
            "terminal-artifact",
            "terminal_support_output",
            "unpromoted",
            30,
        ));

        let unpromoted = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(editor.clone()),
                Some(editor.clone()),
                vec![editor.clone(), terminal_support],
            ),
        );
        assert!(unpromoted.recent_detours.is_empty());
        assert_eq!(
            unpromoted.supporting_context[0].role,
            ActivitySupportRole::OutputVerification
        );

        let stale_promoted_segment = stitched_segment(
            "terminal",
            "Terminal",
            "Test output",
            "terminal-artifact",
            ActivitySegmentRole::PromotedPrimary,
            ActivitySegmentPromotionState::Promoted,
            30,
        );
        let stale = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(stale_promoted_segment.clone()),
                Some(stale_promoted_segment.clone()),
                vec![editor.clone(), stale_promoted_segment.clone()],
            ),
        );
        assert!(stale
            .recent_detours
            .iter()
            .all(|item| item.role != ActivityDetourRole::PromotedPrimary));
        assert!(stale
            .supporting_context
            .iter()
            .all(|item| item.role != ActivitySupportRole::Blocker));

        inputs.branch_contexts[0].promotion_state = "blocked_feedback_suppressed".to_string();
        let blocked = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(stale_promoted_segment.clone()),
                Some(stale_promoted_segment.clone()),
                vec![editor.clone(), stale_promoted_segment],
            ),
        );
        assert!(blocked.recent_detours[0]
            .reason
            .contains("existing branch policy"));
        assert!(!blocked.recent_detours[0].reason.contains("no direct"));

        inputs.branch_contexts[0].promotion_state = "promoted_blocker".to_string();
        let terminal_primary = stitched_segment(
            "terminal",
            "Terminal",
            "Test output",
            "terminal-artifact",
            ActivitySegmentRole::PromotedPrimary,
            ActivitySegmentPromotionState::Promoted,
            30,
        );
        let promoted = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(terminal_primary.clone()),
                Some(terminal_primary.clone()),
                vec![editor, terminal_primary],
            ),
        );

        assert_eq!(
            promoted.recent_detours[0].role,
            ActivityDetourRole::PromotedPrimary
        );
        assert!(promoted.recent_detours[0]
            .reason
            .contains("unresolved blocker"));
        assert_eq!(
            promoted.supporting_context[0].role,
            ActivitySupportRole::Blocker
        );
        assert_eq!(
            promoted.current_state,
            Some(ActivityCurrentState::ActivelyWorking)
        );
    }

    #[test]
    fn filtered_smalltalk_diagnostics_survive_as_redacted_support_only() {
        let mut inputs = empty_inputs();
        inputs.support_evidence.push(SupportEvidenceFact {
            artifact_id: Some("diagnostic-artifact".to_string()),
            artifact_kind: Some("diagnostic".to_string()),
            display_title: Some("Inspect evidence".to_string()),
            branch_kind: "diagnostic_self".to_string(),
            origin_artifact_id: Some("chat-artifact".to_string()),
            role: "support".to_string(),
            public_return_eligible: false,
            reason: Some("internal diagnostic token".to_string()),
            evidence_anchor_ids: vec!["diagnostic-action".to_string()],
        });
        inputs.support_evidence.push(SupportEvidenceFact {
            artifact_id: Some("unlinked-docs".to_string()),
            artifact_kind: Some("browser_tab".to_string()),
            display_title: Some("Documentation".to_string()),
            branch_kind: "documentation_reference".to_string(),
            origin_artifact_id: None,
            role: "support".to_string(),
            public_return_eligible: false,
            reason: Some("no origin".to_string()),
            evidence_anchor_ids: vec!["unlinked-docs-action".to_string()],
        });

        let result = summarize_activity_detours(&inputs, &StitchedActivityTimeline::default());

        assert_eq!(result.supporting_context.len(), 1);
        assert_eq!(
            result.supporting_context[0].role,
            ActivitySupportRole::Diagnostic
        );
        assert!(result.supporting_context[0]
            .summary
            .contains("not as the primary work"));
        assert!(!result.supporting_context[0]
            .summary
            .contains("internal diagnostic token"));
        assert!(result
            .missing_evidence
            .iter()
            .any(|value| value.contains("no grounded primary-work link")));

        let primary = primary_chat();
        let generic_diagnostics = stitched_segment(
            "generic-diagnostics",
            "Developer Tool",
            "Developer diagnostics",
            "generic-diagnostic-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotApplicable,
            30,
        );
        let generic = summarize_activity_detours(
            &empty_inputs(),
            &timeline(
                Some(primary.clone()),
                Some(generic_diagnostics.clone()),
                vec![primary, generic_diagnostics],
            ),
        );
        assert!(generic.supporting_context.is_empty());
        assert_eq!(generic.recent_detours[0].role, ActivityDetourRole::Unclear);
    }

    #[test]
    fn latest_finder_surface_is_explicitly_separated_from_primary_work() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let finder = stitched_segment(
            "finder",
            "Finder",
            "Recent files",
            "finder-artifact",
            ActivitySegmentRole::Detour,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        inputs.branch_contexts.push(branch(
            "finder",
            Some("chat-artifact"),
            "finder-artifact",
            "unrelated_browsing",
            "unpromoted",
            30,
        ));

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(finder.clone()),
                vec![primary, finder],
            ),
        );

        assert!(result.recent_detours[0]
            .reason
            .starts_with("The latest surface was Finder"));
        assert_eq!(
            result.current_state,
            Some(ActivityCurrentState::RecentlyDetoured)
        );
    }

    #[test]
    fn missing_p2_state_stays_unclear_and_reports_missing_evidence() {
        let inputs = empty_inputs();
        let primary = primary_chat();
        let docs = stitched_segment(
            "docs",
            "Safari",
            "Documentation",
            "docs-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotApplicable,
            30,
        );

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(docs.clone()),
                vec![primary, docs],
            ),
        );

        assert_eq!(result.recent_detours[0].role, ActivityDetourRole::Unclear);
        assert_eq!(
            result.recent_detours[0].confidence,
            ActivityEvidenceConfidence::Low
        );
        assert!(!result.missing_evidence.is_empty());
        assert!(!result.recent_detours[0]
            .reason
            .contains("did not become the continuation target"));
        assert!(result.supporting_context.is_empty());
    }

    #[test]
    fn no_origin_docs_branch_is_a_detour_not_connected_support() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let docs = stitched_segment(
            "docs-no-origin",
            "Neutral Browser",
            "Neutral surface",
            "docs-no-origin-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        inputs.branch_contexts.push(branch(
            "docs-no-origin",
            None,
            "docs-no-origin-artifact",
            "documentation_reference",
            "unpromoted",
            30,
        ));
        inputs.branch_contexts[0].origin_workstream_id = Some("ws-primary".to_string());
        inputs.branch_contexts[0].reason_code = Some("branch:no_origin".to_string());

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(docs.clone()),
                vec![primary, docs],
            ),
        );

        assert_eq!(result.recent_detours[0].role, ActivityDetourRole::Detour);
        assert!(result.recent_detours[0].reason.contains("no grounded link"));
        assert!(result.supporting_context.is_empty());
        assert!(result
            .missing_evidence
            .iter()
            .any(|value| value.contains("no grounded link")));
    }

    #[test]
    fn exact_upstream_candidate_promotion_can_survive_without_a_branch_row() {
        let mut inputs = empty_inputs();
        let promoted = stitched_segment(
            "candidate-promoted",
            "Neutral App",
            "Neutral work",
            "candidate-artifact",
            ActivitySegmentRole::PromotedPrimary,
            ActivitySegmentPromotionState::Promoted,
            30,
        );

        let missing = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(promoted.clone()),
                Some(promoted.clone()),
                vec![promoted.clone()],
            ),
        );

        assert_eq!(missing.recent_detours[0].role, ActivityDetourRole::Unclear);
        assert!(!missing.missing_evidence.is_empty());

        inputs.selected_candidate = Some(CandidateFact {
            candidate_id: "candidate-promoted".to_string(),
            workstream_id: "ws-primary".to_string(),
            candidate_kind: "artifact".to_string(),
            target_artifact_id: Some("candidate-artifact".to_string()),
            last_meaningful_action_id: None,
            open_loop_id: None,
            activity_segment_id: Some("candidate-promoted".to_string()),
            activity_intent: Some("editing".to_string()),
            task_phase: Some("in_progress".to_string()),
            continuation_role: Some("resume_target".to_string()),
            score: 0.9,
            evidence_sufficiency_score: 0.9,
            missing_evidence: Vec::new(),
            branch_promotion_state: Some("promoted_primary".to_string()),
            branch_public_return_eligible: Some(true),
        });
        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(promoted.clone()),
                Some(promoted.clone()),
                vec![promoted],
            ),
        );

        assert_eq!(
            result.recent_detours[0].role,
            ActivityDetourRole::PromotedPrimary
        );
        assert!(result.recent_detours[0]
            .reason
            .contains("explicit local promotion evidence"));
    }

    #[test]
    fn nonterminal_verification_branch_uses_neutral_support_copy() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let preview = stitched_segment(
            "preview",
            "Safari",
            "Preview",
            "preview-artifact",
            ActivitySegmentRole::Support,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        inputs.branch_contexts.push(branch(
            "preview",
            Some("chat-artifact"),
            "preview-artifact",
            "verification_branch",
            "unpromoted",
            30,
        ));

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(primary.clone()),
                vec![primary, preview],
            ),
        );

        assert_eq!(result.supporting_context.len(), 1);
        assert_eq!(
            result.supporting_context[0].role,
            ActivitySupportRole::Unknown
        );
        assert!(!result.supporting_context[0]
            .summary
            .to_ascii_lowercase()
            .contains("terminal"));

        inputs.branch_contexts[0].promotion_state = "promoted_blocker".to_string();
        let promoted_preview = stitched_segment(
            "preview",
            "Safari",
            "Build results",
            "preview-artifact",
            ActivitySegmentRole::PromotedPrimary,
            ActivitySegmentPromotionState::Promoted,
            30,
        );
        let promoted = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(promoted_preview.clone()),
                Some(promoted_preview.clone()),
                vec![promoted_preview],
            ),
        );
        assert_eq!(
            promoted.recent_detours[0].role,
            ActivityDetourRole::PromotedPrimary
        );
        assert!(!promoted.recent_detours[0]
            .reason
            .to_ascii_lowercase()
            .contains("terminal"));
    }

    #[test]
    fn only_exact_current_visit_gets_latest_wording_for_a_reused_artifact() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let first = stitched_segment(
            "gmail-first",
            "Gmail",
            "Inbox",
            "gmail-artifact",
            ActivitySegmentRole::Interrupt,
            ActivitySegmentPromotionState::NotPromoted,
            30,
        );
        let current = stitched_segment(
            "gmail-current",
            "Gmail",
            "Inbox",
            "gmail-artifact",
            ActivitySegmentRole::Interrupt,
            ActivitySegmentPromotionState::NotPromoted,
            50,
        );
        inputs.branch_contexts.push(branch(
            "gmail",
            Some("chat-artifact"),
            "gmail-artifact",
            "message_interrupt",
            "unpromoted",
            30,
        ));

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(primary.clone()),
                Some(current.clone()),
                vec![primary, first, current],
            ),
        );

        assert_eq!(result.recent_detours.len(), 1);
        assert!(result.recent_detours[0]
            .reason
            .starts_with("The latest surface was messages"));
    }

    #[test]
    fn latest_branch_context_uses_p2_timestamp_tie_breaking() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let terminal = stitched_segment(
            "terminal-tie",
            "Terminal",
            "Test output",
            "terminal-artifact",
            ActivitySegmentRole::PromotedPrimary,
            ActivitySegmentPromotionState::Promoted,
            30,
        );
        let mut old_promoted = branch(
            "old-promoted",
            Some("chat-artifact"),
            "terminal-artifact",
            "terminal_support_output",
            "promoted_blocker",
            30,
        );
        old_promoted.last_branch_seen_at_ms = 100;
        old_promoted.updated_at_ms = 100;
        let mut newer_unpromoted = branch(
            "newer-unpromoted",
            Some("chat-artifact"),
            "terminal-artifact",
            "terminal_support_output",
            "unpromoted",
            30,
        );
        newer_unpromoted.last_branch_seen_at_ms = 100;
        newer_unpromoted.updated_at_ms = 200;
        inputs.branch_contexts = vec![old_promoted, newer_unpromoted];

        let result = summarize_activity_detours(
            &inputs,
            &timeline(
                Some(terminal.clone()),
                Some(terminal.clone()),
                vec![primary, terminal],
            ),
        );

        assert!(result
            .recent_detours
            .iter()
            .all(|item| item.role != ActivityDetourRole::PromotedPrimary));
        assert!(result
            .supporting_context
            .iter()
            .all(|item| item.role != ActivitySupportRole::Blocker));
    }

    #[test]
    fn public_items_are_capped_and_apply_preserves_non_p5_05_fields() {
        let mut inputs = empty_inputs();
        let primary = primary_chat();
        let mut segments = vec![primary.clone()];
        for index in 0..5 {
            let artifact_id = format!("finder-artifact-{index}");
            let id = format!("finder-{index}");
            segments.push(stitched_segment(
                &id,
                "Finder",
                "Recent files",
                &artifact_id,
                ActivitySegmentRole::Detour,
                ActivitySegmentPromotionState::NotPromoted,
                30 + index * 20,
            ));
            inputs.branch_contexts.push(branch(
                &id,
                Some("chat-artifact"),
                &artifact_id,
                "unrelated_browsing",
                "unpromoted",
                30 + index * 20,
            ));
        }
        let current = segments.last().cloned();
        let detours =
            summarize_activity_detours(&inputs, &timeline(Some(primary), current, segments));
        assert_eq!(detours.recent_detours.len(), MAX_PUBLIC_DETOURS);

        let mut support_inputs = empty_inputs();
        let mut support_segments = vec![primary_chat()];
        for index in 0..5 {
            let artifact_id = format!("docs-artifact-{index}");
            let id = format!("docs-{index}");
            support_segments.push(stitched_segment(
                &id,
                "Neutral Browser",
                "Neutral support",
                &artifact_id,
                ActivitySegmentRole::Support,
                ActivitySegmentPromotionState::NotPromoted,
                30 + index * 20,
            ));
            support_inputs.branch_contexts.push(branch(
                &id,
                Some("chat-artifact"),
                &artifact_id,
                "documentation_reference",
                "unpromoted",
                30 + index * 20,
            ));
        }
        let support_primary = support_segments[0].clone();
        let support = summarize_activity_detours(
            &support_inputs,
            &timeline(
                Some(support_primary.clone()),
                Some(support_primary),
                support_segments,
            ),
        );
        assert_eq!(support.supporting_context.len(), MAX_PUBLIC_SUPPORT_ITEMS);
        assert!(support
            .supporting_context
            .iter()
            .all(|item| !item.evidence_anchor_ids.is_empty()));

        let base = ContinueActivityRecap {
            primary_work_summary: Some("Writing the P5 plan".to_string()),
            primary_work_label: Some("Writing the P5 plan".to_string()),
            target_confidence: ActivityConfidence::High,
            why_this_target: Some("Fresh direct work identifies this target.".to_string()),
            validation_status: ActivityRecapValidationStatus::Valid,
            ..ContinueActivityRecap::default()
        };
        let applied = apply_activity_detour_recap(base, detours);

        assert_eq!(
            applied.primary_work_label.as_deref(),
            Some("Writing the P5 plan")
        );
        assert_eq!(applied.target_confidence, ActivityConfidence::High);
        assert_eq!(
            applied.why_this_target.as_deref(),
            Some("Fresh direct work identifies this target.")
        );
        assert_eq!(
            applied.validation_status,
            ActivityRecapValidationStatus::Valid
        );
    }
}
