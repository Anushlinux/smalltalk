//! Evidence-sufficiency capture arbiter.
//!
//! This is the central control layer for the memory-first capture loop. Instead
//! of "an event happened, so capture a screenshot unless dedupe says no", the
//! arbiter scores what evidence is *missing* for a good resume card and then
//! chooses the cheapest useful capture. A screenshot is only allowed when the
//! work state changed meaningfully, the existing evidence is not enough, privacy
//! permits it, and visual proof is the cheapest remaining way to improve resume
//! quality.
//!
//! Everything here is pure and deterministic (no DB, no OS calls) so it can be
//! unit-tested directly. `capture.rs` gathers the signals into [`EvidenceInputs`]
//! and the budget counts into [`BudgetState`], then calls [`arbitrate`].

use serde::Serialize;

/// The eight 0.0..=1.0 evidence scores plus the overall roll-up and the list of
/// signals we know are missing. Stored per observation in `evidence_sufficiency`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EvidenceSufficiency {
    pub location_score: f32,
    pub content_score: f32,
    pub action_score: f32,
    pub progress_score: f32,
    pub unresolved_score: f32,
    pub reopenability_score: f32,
    pub visual_proof_score: f32,
    pub privacy_risk_score: f32,
    pub overall_score: f32,
    pub missing_signals: Vec<String>,
}

/// Every event resolves to exactly one of these decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CaptureDecision {
    IgnoreNoise,
    MetadataOnly,
    AccessibilityFocusedNode,
    AccessibilityBoundedTree,
    SemanticCheckpoint,
    TinyVisualProbeDiscarded,
    ActiveWindowVisualProof,
    FullScreenVisualProof,
    SkipPrivate,
}

impl CaptureDecision {
    /// Whether this decision results in a stored screenshot / visual proof.
    pub fn saves_visual_proof(self) -> bool {
        matches!(
            self,
            CaptureDecision::ActiveWindowVisualProof | CaptureDecision::FullScreenVisualProof
        )
    }

    /// Stable machine label used for storage and the debug UI.
    pub fn as_label(self) -> &'static str {
        match self {
            CaptureDecision::IgnoreNoise => "ignore_noise",
            CaptureDecision::MetadataOnly => "metadata_only",
            CaptureDecision::AccessibilityFocusedNode => "accessibility_focused_node",
            CaptureDecision::AccessibilityBoundedTree => "accessibility_bounded_tree",
            CaptureDecision::SemanticCheckpoint => "semantic_checkpoint",
            CaptureDecision::TinyVisualProbeDiscarded => "tiny_visual_probe_discarded",
            CaptureDecision::ActiveWindowVisualProof => "active_window_visual_proof",
            CaptureDecision::FullScreenVisualProof => "full_screen_visual_proof",
            CaptureDecision::SkipPrivate => "skip_private",
        }
    }
}

/// The only reasons a visual proof is allowed to be saved. `event_happened` is
/// deliberately absent — a screenshot without a precise reason must not be saved.
pub const ALLOWED_VISUAL_PROOF_REASONS: &[&str] = &[
    "manual",
    "thread_start",
    "new_work_item",
    "before_idle",
    "resume_anchor",
    "thin_accessibility",
    "visual_surface",
    "large_surface_delta",
    "error_state",
    "unresolved_work_changed",
    "reopenability_anchor",
];

/// The forbidden reason.
pub const FORBIDDEN_REASON: &str = "event_happened";

/// Returns true if `reason` is a valid visual-proof reason.
pub fn is_valid_visual_proof_reason(reason: &str) -> bool {
    reason != FORBIDDEN_REASON && ALLOWED_VISUAL_PROOF_REASONS.contains(&reason)
}

/// Hard per-hour capture limits. V1 defaults match the spec.
#[derive(Debug, Clone, Copy)]
pub struct CaptureBudget {
    pub max_visual_proofs_per_hour: u32,
    pub max_fullscreen_proofs_per_hour: u32,
    pub max_ocr_runs_per_hour: u32,
    pub min_seconds_between_visual_proofs: u32,
}

impl Default for CaptureBudget {
    fn default() -> Self {
        Self {
            max_visual_proofs_per_hour: 12,
            max_fullscreen_proofs_per_hour: 3,
            max_ocr_runs_per_hour: 20,
            min_seconds_between_visual_proofs: 180,
        }
    }
}

impl CaptureBudget {
    /// Higher ceiling for canvas/visual work where AX is unreliable.
    pub fn for_work_type(visual_work: bool) -> Self {
        if visual_work {
            Self {
                max_visual_proofs_per_hour: 24,
                ..Self::default()
            }
        } else {
            Self::default()
        }
    }

    pub fn can_save_visual_proof(&self, state: &BudgetState) -> bool {
        state.visual_proofs_last_hour < self.max_visual_proofs_per_hour
            && state
                .seconds_since_last_visual_proof
                .map_or(true, |secs| secs >= self.min_seconds_between_visual_proofs)
    }

    pub fn can_save_fullscreen(&self, state: &BudgetState) -> bool {
        state.fullscreen_proofs_last_hour < self.max_fullscreen_proofs_per_hour
    }

    pub fn can_run_ocr(&self, state: &BudgetState) -> bool {
        state.ocr_runs_last_hour < self.max_ocr_runs_per_hour
    }
}

/// Current spend against the budget, populated from the DB by `capture.rs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BudgetState {
    pub visual_proofs_last_hour: u32,
    pub fullscreen_proofs_last_hour: u32,
    pub ocr_runs_last_hour: u32,
    pub seconds_since_last_visual_proof: Option<u32>,
}

/// How unfinished the current work looks. Maps to `unresolved_score`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedLevel {
    /// unsent composer, dirty editor, failing terminal, pre-idle active work.
    Strong,
    /// active task suspended after idle/app switch.
    Suspended,
    /// likely ongoing work but no precise unresolved state.
    Likely,
    None,
}

/// How good a visual anchor we already have for this work item/surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualProofLevel {
    /// recent visual proof exists for this work item and surface.
    RecentSameSurface,
    /// older visual proof exists but state changed since.
    OlderStateChanged,
    /// probe says the screen changed but no proof stored.
    ProbeChangedNoProof,
    None,
}

/// All the signals the arbiter needs, gathered by `capture.rs` from the work
/// state draft, delta, accessibility context, privacy decision, and DB recency.
#[derive(Debug, Clone)]
pub struct EvidenceInputs {
    pub trigger: String,
    pub work_type: String,
    pub activity: String,

    // location
    pub app_known: bool,
    pub window_known: bool,
    pub location_ref_known: bool,

    // content (pre-capture, AX only)
    pub text_is_thin: bool,
    pub text_chars: usize,
    pub has_focused_value: bool,
    pub has_selection: bool,
    pub has_title: bool,

    // progress
    pub meaningfully_changed: bool,
    pub state_fingerprint_changed: bool,
    pub visible_text_changed: bool,
    pub delta_score: f64,

    // unresolved
    pub unresolved: UnresolvedLevel,

    // reopenability
    pub reopen_path_known: bool,

    // visual proof
    pub visual_proof: VisualProofLevel,

    // privacy
    pub privacy_status: String,
    pub privacy_skip: bool,
    pub sensitive_surface: bool,

    // surface / accessibility
    pub ax_available: bool,
    pub surface_is_visual_or_thin_ax: bool,
}

impl Default for EvidenceInputs {
    fn default() -> Self {
        Self {
            trigger: "idle".to_string(),
            work_type: "unknown".to_string(),
            activity: "working".to_string(),
            app_known: false,
            window_known: false,
            location_ref_known: false,
            text_is_thin: true,
            text_chars: 0,
            has_focused_value: false,
            has_selection: false,
            has_title: false,
            meaningfully_changed: false,
            state_fingerprint_changed: false,
            visible_text_changed: false,
            delta_score: 0.0,
            unresolved: UnresolvedLevel::None,
            reopen_path_known: false,
            visual_proof: VisualProofLevel::None,
            privacy_status: "normal".to_string(),
            privacy_skip: false,
            sensitive_surface: false,
            ax_available: false,
            surface_is_visual_or_thin_ax: false,
        }
    }
}

/// The final outcome: scores, the chosen decision, and the (validated) reason.
#[derive(Debug, Clone, Serialize)]
pub struct ArbiterOutcome {
    pub sufficiency: EvidenceSufficiency,
    #[serde(serialize_with = "serialize_decision_label")]
    pub decision: CaptureDecision,
    pub reason: String,
}

fn serialize_decision_label<S>(decision: &CaptureDecision, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(decision.as_label())
}

// -------- scoring (deterministic, per spec tables) --------

fn score_location(i: &EvidenceInputs) -> f32 {
    if i.app_known && i.window_known && i.location_ref_known {
        1.0
    } else if i.app_known && i.window_known {
        0.7
    } else if i.app_known {
        0.4
    } else {
        0.0
    }
}

fn score_content(i: &EvidenceInputs) -> f32 {
    if !i.text_is_thin && (i.text_chars >= 200 || i.has_focused_value) {
        1.0
    } else if i.has_selection || i.text_chars >= 40 {
        0.7
    } else if i.text_chars > 0 {
        0.5
    } else if i.has_title {
        0.2
    } else {
        0.0
    }
}

fn score_action(i: &EvidenceInputs) -> f32 {
    match i.activity.as_str() {
        "composing" | "editing" | "reading" | "debugging" | "searching" => 1.0,
        "navigating" | "working" => 0.6,
        "idle" | "starting" => 0.2,
        _ => 0.0,
    }
}

fn score_progress(i: &EvidenceInputs) -> f32 {
    if i.meaningfully_changed && i.state_fingerprint_changed {
        1.0
    } else if i.visible_text_changed {
        0.7
    } else if i.delta_score >= 0.25 {
        0.5
    } else if i.delta_score > 0.0 {
        0.2
    } else {
        0.0
    }
}

fn score_unresolved(i: &EvidenceInputs) -> f32 {
    match i.unresolved {
        UnresolvedLevel::Strong => 1.0,
        UnresolvedLevel::Suspended => 0.7,
        UnresolvedLevel::Likely => 0.4,
        UnresolvedLevel::None => 0.0,
    }
}

fn score_reopenability(i: &EvidenceInputs) -> f32 {
    if i.reopen_path_known {
        1.0
    } else if i.app_known && i.window_known {
        0.7
    } else if i.app_known {
        0.4
    } else {
        0.0
    }
}

fn score_visual_proof(i: &EvidenceInputs) -> f32 {
    match i.visual_proof {
        VisualProofLevel::RecentSameSurface => 1.0,
        VisualProofLevel::OlderStateChanged => 0.7,
        VisualProofLevel::ProbeChangedNoProof => 0.4,
        VisualProofLevel::None => 0.0,
    }
}

fn score_privacy_risk(i: &EvidenceInputs) -> f32 {
    if i.privacy_skip || i.privacy_status == "skipped_sensitive" {
        1.0
    } else if i.sensitive_surface {
        0.7
    } else if i.privacy_status == "redacted" {
        0.4
    } else {
        0.0
    }
}

/// Compute all eight scores plus the overall roll-up and the missing-signal list.
pub fn compute_sufficiency(i: &EvidenceInputs) -> EvidenceSufficiency {
    let location = score_location(i);
    let content = score_content(i);
    let action = score_action(i);
    let progress = score_progress(i);
    let unresolved = score_unresolved(i);
    let reopenability = score_reopenability(i);
    let visual_proof = score_visual_proof(i);
    let privacy_risk = score_privacy_risk(i);

    // Weighted mean of the "do we understand the work" scores. Privacy risk is a
    // gate, not part of the roll-up.
    let overall = location * 0.20
        + content * 0.15
        + action * 0.10
        + progress * 0.15
        + unresolved * 0.15
        + reopenability * 0.15
        + visual_proof * 0.10;

    let mut missing = Vec::new();
    if location < 0.7 {
        missing.push("location_unknown".to_string());
    }
    if content < 0.5 {
        missing.push("content_thin".to_string());
    }
    if action < 0.6 {
        missing.push("action_mode_unknown".to_string());
    }
    if reopenability < 0.7 {
        missing.push("reopen_target_unclear".to_string());
    }
    if visual_proof < 0.5 {
        missing.push("no_visual_anchor".to_string());
    }
    if i.privacy_skip || i.privacy_status == "skipped_sensitive" {
        missing.push("private_surface".to_string());
        missing.push("content_redacted".to_string());
    } else if i.privacy_status == "redacted" {
        missing.push("content_redacted".to_string());
    }

    EvidenceSufficiency {
        location_score: location,
        content_score: content,
        action_score: action,
        progress_score: progress,
        unresolved_score: unresolved,
        reopenability_score: reopenability,
        visual_proof_score: visual_proof,
        privacy_risk_score: privacy_risk,
        overall_score: overall,
        missing_signals: missing,
    }
}

/// The core capture decision. Follows the spec's `decide_capture` pseudocode,
/// with the visual-proof branches gated on the budget and enriched with a precise
/// reason. Never returns `event_happened`.
pub fn decide_capture(
    inputs: &EvidenceInputs,
    sufficiency: &EvidenceSufficiency,
    budget: &CaptureBudget,
    budget_state: &BudgetState,
) -> (CaptureDecision, String, Vec<String>) {
    let mut extra_missing: Vec<String> = Vec::new();

    // Privacy gate: never recover a private surface with a screenshot.
    if sufficiency.privacy_risk_score >= 0.8 {
        return (
            CaptureDecision::SkipPrivate,
            "private_surface_skipped".to_string(),
            extra_missing,
        );
    }

    // Explicit user request always wins and is allowed full screen.
    if inputs.trigger == "manual" {
        return (
            CaptureDecision::FullScreenVisualProof,
            "manual".to_string(),
            extra_missing,
        );
    }

    // Session start is the resume anchor for the run.
    if inputs.trigger == "session_start" && budget.can_save_visual_proof(budget_state) {
        return (
            CaptureDecision::ActiveWindowVisualProof,
            "thread_start".to_string(),
            extra_missing,
        );
    }

    // Nothing changed and we already understand the work well enough.
    if !inputs.meaningfully_changed && sufficiency.overall_score >= 0.75 {
        return (
            CaptureDecision::MetadataOnly,
            "saved_state_is_enough".to_string(),
            extra_missing,
        );
    }

    // We do not even know where the user is — a screenshot will not fix that.
    if sufficiency.location_score < 0.7 {
        return (
            CaptureDecision::MetadataOnly,
            "location_unknown".to_string(),
            extra_missing,
        );
    }

    // Content is weak but the accessibility tree can still supply it cheaply.
    if sufficiency.content_score < 0.6 && inputs.ax_available {
        return (
            CaptureDecision::AccessibilityBoundedTree,
            "content_thin_use_ax".to_string(),
            extra_missing,
        );
    }

    // No clear state change yet — record a semantic checkpoint, not pixels.
    if sufficiency.progress_score < 0.6 {
        return (
            CaptureDecision::SemanticCheckpoint,
            "progress_unclear".to_string(),
            extra_missing,
        );
    }

    // Unresolved work with a weak visual anchor is the strongest case for pixels.
    if sufficiency.visual_proof_score < 0.5 && sufficiency.unresolved_score >= 0.6 {
        if budget.can_save_visual_proof(budget_state) {
            return (
                CaptureDecision::ActiveWindowVisualProof,
                "unresolved_work_changed".to_string(),
                extra_missing,
            );
        }
        extra_missing.push("visual_budget_exhausted".to_string());
        return (
            CaptureDecision::SemanticCheckpoint,
            "visual_budget_exhausted".to_string(),
            extra_missing,
        );
    }

    // Canvas / thin-AX surfaces where structured text cannot capture the work.
    if inputs.surface_is_visual_or_thin_ax {
        if budget.can_save_visual_proof(budget_state) {
            let reason = if is_visual_work_type(&inputs.work_type) {
                "visual_surface"
            } else {
                "thin_accessibility"
            };
            return (
                CaptureDecision::ActiveWindowVisualProof,
                reason.to_string(),
                extra_missing,
            );
        }
        extra_missing.push("visual_budget_exhausted".to_string());
        return (
            CaptureDecision::SemanticCheckpoint,
            "visual_budget_exhausted".to_string(),
            extra_missing,
        );
    }

    // Default: the saved work state is enough.
    (
        CaptureDecision::MetadataOnly,
        "saved_state_is_enough".to_string(),
        extra_missing,
    )
}

/// True for surfaces where the accessibility tree is unreliable and visuals carry
/// the meaning (mirrors `capture.rs::is_visual_work_type`).
pub fn is_visual_work_type(work_type: &str) -> bool {
    matches!(work_type, "canvas" | "pdf" | "media") || work_type.contains("design")
}

/// End-to-end: score the evidence, choose a capture decision, and return the
/// combined outcome with a validated reason and any budget-driven missing signals.
pub fn arbitrate(
    inputs: &EvidenceInputs,
    budget: &CaptureBudget,
    budget_state: &BudgetState,
) -> ArbiterOutcome {
    let mut sufficiency = compute_sufficiency(inputs);
    let (decision, reason, extra_missing) =
        decide_capture(inputs, &sufficiency, budget, budget_state);
    for signal in extra_missing {
        if !sufficiency.missing_signals.contains(&signal) {
            sufficiency.missing_signals.push(signal);
        }
    }
    // Safety net: a visual proof must always carry a valid reason.
    debug_assert!(
        !decision.saves_visual_proof() || is_valid_visual_proof_reason(&reason),
        "visual proof reason must be in the allowed set, got {reason}"
    );
    ArbiterOutcome {
        sufficiency,
        decision,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strong_text_inputs() -> EvidenceInputs {
        EvidenceInputs {
            trigger: "typing_pause".to_string(),
            work_type: "code_editor".to_string(),
            activity: "editing".to_string(),
            app_known: true,
            window_known: true,
            location_ref_known: true,
            text_is_thin: false,
            text_chars: 800,
            has_focused_value: true,
            has_selection: true,
            has_title: true,
            meaningfully_changed: false,
            state_fingerprint_changed: false,
            visible_text_changed: false,
            delta_score: 0.0,
            unresolved: UnresolvedLevel::Likely,
            reopen_path_known: true,
            visual_proof: VisualProofLevel::RecentSameSurface,
            privacy_status: "normal".to_string(),
            privacy_skip: false,
            sensitive_surface: false,
            ax_available: true,
            surface_is_visual_or_thin_ax: false,
        }
    }

    // Test 1: 100 repeated same-surface events with strong state → <=2 visual proofs.
    #[test]
    fn repeated_strong_state_events_do_not_screenshot() {
        let budget = CaptureBudget::default();
        let budget_state = BudgetState::default();
        let inputs = strong_text_inputs();
        let mut proofs = 0;
        let mut cheap = 0;
        for _ in 0..100 {
            let outcome = arbitrate(&inputs, &budget, &budget_state);
            if outcome.decision.saves_visual_proof() {
                proofs += 1;
            }
            if matches!(
                outcome.decision,
                CaptureDecision::MetadataOnly | CaptureDecision::SemanticCheckpoint
            ) {
                cheap += 1;
            }
        }
        assert!(proofs <= 2, "expected <=2 visual proofs, got {proofs}");
        assert!(cheap >= 98, "expected mostly cheap decisions, got {cheap}");
    }

    // Test 2: weak content + AX available → AccessibilityBoundedTree, not a screenshot.
    #[test]
    fn weak_content_escalates_to_ax() {
        let mut inputs = strong_text_inputs();
        inputs.meaningfully_changed = true;
        inputs.text_is_thin = true;
        inputs.text_chars = 0;
        inputs.has_focused_value = false;
        inputs.has_selection = false;
        inputs.ax_available = true;
        let outcome = arbitrate(&inputs, &CaptureBudget::default(), &BudgetState::default());
        assert!(outcome.sufficiency.content_score < 0.6);
        assert_eq!(outcome.decision, CaptureDecision::AccessibilityBoundedTree);
    }

    // Test 3: thin AX surface + budget → ActiveWindowVisualProof with allowed reason.
    #[test]
    fn thin_ax_permits_visual_proof() {
        let mut inputs = strong_text_inputs();
        inputs.meaningfully_changed = true;
        inputs.state_fingerprint_changed = true;
        inputs.visible_text_changed = true;
        inputs.delta_score = 0.9;
        inputs.surface_is_visual_or_thin_ax = true;
        inputs.visual_proof = VisualProofLevel::None;
        inputs.unresolved = UnresolvedLevel::None;
        let outcome = arbitrate(&inputs, &CaptureBudget::default(), &BudgetState::default());
        assert_eq!(outcome.decision, CaptureDecision::ActiveWindowVisualProof);
        assert!(
            outcome.reason == "thin_accessibility" || outcome.reason == "visual_surface",
            "unexpected reason {}",
            outcome.reason
        );
        assert!(is_valid_visual_proof_reason(&outcome.reason));
    }

    // Test 4: private surface → SkipPrivate, no visual proof, private_surface signal.
    #[test]
    fn private_surface_never_screenshots() {
        let mut inputs = strong_text_inputs();
        inputs.meaningfully_changed = true;
        inputs.privacy_skip = true;
        inputs.privacy_status = "skipped_sensitive".to_string();
        let outcome = arbitrate(&inputs, &CaptureBudget::default(), &BudgetState::default());
        assert_eq!(outcome.decision, CaptureDecision::SkipPrivate);
        assert!(!outcome.decision.saves_visual_proof());
        assert!(outcome
            .sufficiency
            .missing_signals
            .contains(&"private_surface".to_string()));
    }

    // Test 6: any produced visual-proof reason is valid and never event_happened.
    #[test]
    fn visual_proof_reason_is_always_valid() {
        let budget = CaptureBudget::default();
        let budget_state = BudgetState::default();
        let triggers = ["manual", "session_start", "typing_pause", "idle", "scroll_stop"];
        for trigger in triggers {
            let mut inputs = strong_text_inputs();
            inputs.trigger = trigger.to_string();
            inputs.meaningfully_changed = true;
            inputs.state_fingerprint_changed = true;
            inputs.visible_text_changed = true;
            inputs.delta_score = 0.9;
            inputs.surface_is_visual_or_thin_ax = true;
            inputs.visual_proof = VisualProofLevel::None;
            let outcome = arbitrate(&inputs, &budget, &budget_state);
            if outcome.decision.saves_visual_proof() {
                assert!(
                    is_valid_visual_proof_reason(&outcome.reason),
                    "invalid reason {} for trigger {trigger}",
                    outcome.reason
                );
                assert_ne!(outcome.reason, FORBIDDEN_REASON);
            }
        }
    }

    // Test 7: budget exhausted → falls back to a checkpoint and flags the signal.
    #[test]
    fn budget_exhaustion_blocks_visual_proof() {
        let budget = CaptureBudget::default();
        let budget_state = BudgetState {
            visual_proofs_last_hour: budget.max_visual_proofs_per_hour,
            ..BudgetState::default()
        };
        let mut inputs = strong_text_inputs();
        inputs.meaningfully_changed = true;
        inputs.state_fingerprint_changed = true;
        inputs.visible_text_changed = true;
        inputs.delta_score = 0.9;
        inputs.visual_proof = VisualProofLevel::None;
        inputs.unresolved = UnresolvedLevel::Strong;
        let outcome = arbitrate(&inputs, &budget, &budget_state);
        assert!(!outcome.decision.saves_visual_proof());
        assert!(matches!(
            outcome.decision,
            CaptureDecision::SemanticCheckpoint | CaptureDecision::MetadataOnly
        ));
        assert!(outcome
            .sufficiency
            .missing_signals
            .contains(&"visual_budget_exhausted".to_string()));
    }

    // min_seconds_between_visual_proofs also blocks a proof even under count budget.
    #[test]
    fn min_interval_blocks_visual_proof() {
        let budget = CaptureBudget::default();
        let budget_state = BudgetState {
            visual_proofs_last_hour: 1,
            seconds_since_last_visual_proof: Some(10),
            ..BudgetState::default()
        };
        let mut inputs = strong_text_inputs();
        inputs.meaningfully_changed = true;
        inputs.state_fingerprint_changed = true;
        inputs.visible_text_changed = true;
        inputs.delta_score = 0.9;
        inputs.surface_is_visual_or_thin_ax = true;
        inputs.visual_proof = VisualProofLevel::None;
        let outcome = arbitrate(&inputs, &budget, &budget_state);
        assert!(!outcome.decision.saves_visual_proof());
    }
}
