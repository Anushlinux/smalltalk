import AppKit
import Combine
import Foundation
import QuartzCore
import SwiftUI

public typealias SmalltalkIslandActionCallback = @convention(c) (UnsafePointer<CChar>) -> Void
private var gActionCallback: SmalltalkIslandActionCallback?

private struct IslandSnapshot: Decodable {
    var state: String = "hidden"
    var memoryActive = false
    var elapsedMs: Int64 = 0
    var frameCount: Int64 = 0
    var eventCount: Int64?
    var trailAppCount: Int64?
    var trailMomentCount: Int64?
    var trailLabels: [String]?
    var lastFrameId: Int64?
    var currentApp: String?
    var currentWindow: String?
    var currentSurfaceKind: String?
    var lastTrigger: String?
    var lastCaptureAtMs: Int64?
    var capturePulseNonce: UInt64?
    var lastError: String?
    var resumeHeadline: String?
    var resumeDetail: String?
    var resumePoint: String?
    var resumeSource: String?
    var resumeModel: String?
    var resumeResponseId: String?
    var continueDecisionId: String?
    var continueFreshness: String?
    var evidenceUpdatedAtMs: Int64?
    var decisionUpdatedAtMs: Int64?
    var continueOpenable: Bool?
    var resumeWarning: String?
    var islandContinueState: IslandContinueState?
    var privacyLabel: String?
    var isSensitive: Bool = false

    enum CodingKeys: String, CodingKey {
        case state
        case memoryActive = "memory_active"
        case elapsedMs = "elapsed_ms"
        case frameCount = "frame_count"
        case eventCount = "event_count"
        case trailAppCount = "trail_app_count"
        case trailMomentCount = "trail_moment_count"
        case trailLabels = "trail_labels"
        case lastFrameId = "last_frame_id"
        case currentApp = "current_app"
        case currentWindow = "current_window"
        case currentSurfaceKind = "current_surface_kind"
        case lastTrigger = "last_trigger"
        case lastCaptureAtMs = "last_capture_at_ms"
        case capturePulseNonce = "capture_pulse_nonce"
        case lastError = "last_error"
        case resumeHeadline = "resume_headline"
        case resumeDetail = "resume_detail"
        case resumePoint = "resume_point"
        case resumeSource = "resume_source"
        case resumeModel = "resume_model"
        case resumeResponseId = "resume_response_id"
        case continueDecisionId = "continue_decision_id"
        case continueFreshness = "continue_freshness"
        case evidenceUpdatedAtMs = "evidence_updated_at_ms"
        case decisionUpdatedAtMs = "decision_updated_at_ms"
        case continueOpenable = "continue_openable"
        case resumeWarning = "resume_warning"
        case islandContinueState = "island_continue_state"
        case privacyLabel = "privacy_label"
        case isSensitive = "is_sensitive"
    }

    static func continueDecodeError() -> IslandSnapshot {
        var snapshot = IslandSnapshot()
        snapshot.state = "error"
        snapshot.lastError = "Continue state could not be decoded."
        snapshot.islandContinueState = IslandContinueState.errorFallback()
        return snapshot
    }
}

private struct IslandSemanticCurrentActivity: Decodable, Equatable {
    var observedSurface: String?
    var immediateUserOperation: String?
    var semanticEffectOfOperation: String?
    var currentSubtask: String?
    var relationshipToPrimary: String

    enum CodingKeys: String, CodingKey {
        case observedSurface = "observed_surface"
        case immediateUserOperation = "immediate_user_operation"
        case semanticEffectOfOperation = "semantic_effect_of_operation"
        case currentSubtask = "current_subtask"
        case relationshipToPrimary = "relationship_to_primary"
    }
}

private struct IslandSemanticAtomicIdentity: Decodable, Equatable {
    var sessionId: String?
    var taskThreadId: String?
    var taskThreadRevision: Int64?
    var taskSnapshotId: String
    var snapshotRevision: Int64
    var selectedHypothesisId: String?
    var modelRequestId: String?
    var modelResponseId: String?
    var observationPacketId: String
    var evidenceWatermark: String
    var correctionFingerprint: String

    enum CodingKeys: String, CodingKey {
        case sessionId = "session_id"
        case taskThreadId = "task_thread_id"
        case taskThreadRevision = "task_thread_revision"
        case taskSnapshotId = "task_snapshot_id"
        case snapshotRevision = "snapshot_revision"
        case selectedHypothesisId = "selected_hypothesis_id"
        case modelRequestId = "model_request_id"
        case modelResponseId = "model_response_id"
        case observationPacketId = "observation_packet_id"
        case evidenceWatermark = "evidence_watermark"
        case correctionFingerprint = "correction_fingerprint"
    }
}

private struct IslandSemanticAlternative: Decodable, Equatable {
    var hypothesisId: String
    var taskSummary: String
    var relation: String
    var confidence: Double
    var evidenceRefs: [String]
    var contradictingEvidenceRefs: [String]
    var taskThreadId: String?
    var taskThreadRevision: Int64?
    var lastSupportedAtMs: Int64?
    var disposition: String
    var reasonCodes: [String]

    enum CodingKeys: String, CodingKey {
        case hypothesisId = "hypothesis_id"
        case taskSummary = "task_summary"
        case relation
        case confidence
        case evidenceRefs = "evidence_refs"
        case contradictingEvidenceRefs = "contradicting_evidence_refs"
        case taskThreadId = "task_thread_id"
        case taskThreadRevision = "task_thread_revision"
        case lastSupportedAtMs = "last_supported_at_ms"
        case disposition
        case reasonCodes = "reason_codes"
    }
}

private struct IslandSemanticRecentContext: Decodable, Equatable {
    var sequenceIndex: Int
    var appLabel: String
    var siteHostname: String?
    var firstObservedAtMs: Int64
    var lastObservedAtMs: Int64
    var isCurrent: Bool
    var revisited: Bool
    var semanticRole: String?
    var roleConfidence: Double?
    var relationshipToPrimaryTask: String?

    enum CodingKeys: String, CodingKey {
        case sequenceIndex = "sequence_index"
        case appLabel = "app_label"
        case siteHostname = "site_hostname"
        case firstObservedAtMs = "first_observed_at_ms"
        case lastObservedAtMs = "last_observed_at_ms"
        case isCurrent = "is_current"
        case revisited
        case semanticRole = "semantic_role"
        case roleConfidence = "role_confidence"
        case relationshipToPrimaryTask = "relationship_to_primary_task"
    }
}

private struct IslandSemanticAnswer: Decodable, Equatable {
    var schema: String
    var taskResolutionStatus: String
    var taskSummary: String?
    var taskObject: String?
    var currentActivity: IslandSemanticCurrentActivity
    var lastMeaningfulProgress: String?
    var unfinishedState: String?
    var executionState: String
    var nextAction: String?
    var whereSummary: String?
    var relationshipToPrior: String
    var recentContext: [IslandSemanticRecentContext]?
    var alternativeHypotheses: [IslandSemanticAlternative]
    var inferenceStatus: String?
    var atomicIdentity: IslandSemanticAtomicIdentity

    enum CodingKeys: String, CodingKey {
        case schema
        case taskResolutionStatus = "task_resolution_status"
        case taskSummary = "task_summary"
        case taskObject = "task_object"
        case currentActivity = "current_activity"
        case lastMeaningfulProgress = "last_meaningful_progress"
        case unfinishedState = "unfinished_state"
        case executionState = "execution_state"
        case nextAction = "next_action"
        case whereSummary = "where_summary"
        case relationshipToPrior = "relationship_to_prior"
        case recentContext = "recent_context"
        case alternativeHypotheses = "alternative_hypotheses"
        case inferenceStatus = "inference_status"
        case atomicIdentity = "atomic_identity"
    }
}

private struct IslandContinueState: Decodable, Equatable {
    var schema: String
    var displayState: IslandDisplayState
    var decisionId: String?
    var targetState: String
    var targetReasonCodes: [String]
    var semanticAnswer: IslandSemanticAnswer?
    var currentFocus: IslandFocusSummary?
    var currentActivity: String?
    var activityLabel: String?
    var activitySummary: String?
    var activityWhere: String?
    var activityState: String?
    var activityConfidenceLabel: String?
    var targetConfidenceLabel: String?
    var recentContextSummary: String?
    var selectedWorkstreamTitle: String?
    var returnTarget: IslandTargetSummary?
    var resumeWorkTarget: IslandTargetSummary?
    var nextAction: String?
    var confidenceLabel: String?
    var missingEvidence: [String]
    var warnings: [String]
    var suppressionReasons: [String]
    var availableActions: [IslandAvailableAction]

    enum CodingKeys: String, CodingKey {
        case schema
        case displayState = "display_state"
        case decisionId = "decision_id"
        case targetState = "target_state"
        case targetReasonCodes = "target_reason_codes"
        case semanticAnswer = "semantic_answer"
        case currentFocus = "current_focus"
        case currentActivity = "current_activity"
        case activityLabel = "activity_label"
        case activitySummary = "activity_summary"
        case activityWhere = "activity_where"
        case activityState = "activity_state"
        case activityConfidenceLabel = "activity_confidence_label"
        case targetConfidenceLabel = "target_confidence_label"
        case recentContextSummary = "recent_context_summary"
        case selectedWorkstreamTitle = "selected_workstream_title"
        case returnTarget = "return_target"
        case resumeWorkTarget = "resume_work_target"
        case nextAction = "next_action"
        case confidenceLabel = "confidence_label"
        case missingEvidence = "missing_evidence"
        case warnings
        case suppressionReasons = "suppression_reasons"
        case availableActions = "available_actions"
    }

    init(
        schema: String = "smalltalk.island_continue_state.v1",
        displayState: IslandDisplayState,
        decisionId: String? = nil,
        targetState: String = "no_clear_task",
        targetReasonCodes: [String] = [],
        semanticAnswer: IslandSemanticAnswer? = nil,
        currentFocus: IslandFocusSummary? = nil,
        currentActivity: String? = nil,
        activityLabel: String? = nil,
        activitySummary: String? = nil,
        activityWhere: String? = nil,
        activityState: String? = nil,
        activityConfidenceLabel: String? = nil,
        targetConfidenceLabel: String? = nil,
        recentContextSummary: String? = nil,
        selectedWorkstreamTitle: String? = nil,
        returnTarget: IslandTargetSummary? = nil,
        resumeWorkTarget: IslandTargetSummary? = nil,
        nextAction: String? = nil,
        confidenceLabel: String? = nil,
        missingEvidence: [String] = [],
        warnings: [String] = [],
        suppressionReasons: [String] = [],
        availableActions: [IslandAvailableAction] = []
    ) {
        self.schema = schema
        self.displayState = displayState
        self.decisionId = decisionId
        self.targetState = targetState
        self.targetReasonCodes = targetReasonCodes
        self.semanticAnswer = semanticAnswer
        self.currentFocus = currentFocus
        self.currentActivity = currentActivity
        self.activityLabel = activityLabel
        self.activitySummary = activitySummary
        self.activityWhere = activityWhere
        self.activityState = activityState
        self.activityConfidenceLabel = activityConfidenceLabel
        self.targetConfidenceLabel = targetConfidenceLabel
        self.recentContextSummary = recentContextSummary
        self.selectedWorkstreamTitle = selectedWorkstreamTitle
        self.returnTarget = returnTarget
        self.resumeWorkTarget = resumeWorkTarget
        self.nextAction = nextAction
        self.confidenceLabel = confidenceLabel
        self.missingEvidence = missingEvidence
        self.warnings = warnings
        self.suppressionReasons = suppressionReasons
        self.availableActions = availableActions
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        schema = try container.decodeIfPresent(String.self, forKey: .schema) ?? "unknown"
        displayState = try container.decodeIfPresent(IslandDisplayState.self, forKey: .displayState) ?? .error
        decisionId = try container.decodeIfPresent(String.self, forKey: .decisionId)
        targetState = try container.decodeIfPresent(String.self, forKey: .targetState) ?? "no_clear_task"
        targetReasonCodes = try container.decodeIfPresent([String].self, forKey: .targetReasonCodes) ?? []
        semanticAnswer = try container.decodeIfPresent(IslandSemanticAnswer.self, forKey: .semanticAnswer)
        currentFocus = try container.decodeIfPresent(IslandFocusSummary.self, forKey: .currentFocus)
        currentActivity = try container.decodeIfPresent(String.self, forKey: .currentActivity)
        activityLabel = try container.decodeIfPresent(String.self, forKey: .activityLabel)
        activitySummary = try container.decodeIfPresent(String.self, forKey: .activitySummary)
        activityWhere = try container.decodeIfPresent(String.self, forKey: .activityWhere)
        activityState = try container.decodeIfPresent(String.self, forKey: .activityState)
        activityConfidenceLabel = try container.decodeIfPresent(String.self, forKey: .activityConfidenceLabel)
        targetConfidenceLabel = try container.decodeIfPresent(String.self, forKey: .targetConfidenceLabel)
        recentContextSummary = try container.decodeIfPresent(String.self, forKey: .recentContextSummary)
        selectedWorkstreamTitle = try container.decodeIfPresent(String.self, forKey: .selectedWorkstreamTitle)
        returnTarget = try container.decodeIfPresent(IslandTargetSummary.self, forKey: .returnTarget)
        resumeWorkTarget = try container.decodeIfPresent(IslandTargetSummary.self, forKey: .resumeWorkTarget)
        nextAction = try container.decodeIfPresent(String.self, forKey: .nextAction)
        confidenceLabel = try container.decodeIfPresent(String.self, forKey: .confidenceLabel)
        missingEvidence = try container.decodeIfPresent([String].self, forKey: .missingEvidence) ?? []
        warnings = try container.decodeIfPresent([String].self, forKey: .warnings) ?? []
        suppressionReasons = try container.decodeIfPresent([String].self, forKey: .suppressionReasons) ?? []
        availableActions = try container.decodeIfPresent([IslandAvailableAction].self, forKey: .availableActions) ?? []
    }

    static func errorFallback() -> IslandContinueState {
        IslandContinueState(
            displayState: .error,
            nextAction: "Open Smalltalk to inspect local memory",
            availableActions: [
                IslandAvailableAction(kind: .openSmalltalk, label: "Open Smalltalk", enabled: true)
            ]
        )
    }

    static func fallback(from snapshot: IslandSnapshot) -> IslandContinueState {
        if snapshot.lastError != nil || snapshot.state == "error" {
            return errorFallback()
        }
        if snapshot.state == "starting" || snapshot.state == "processing" {
            return IslandContinueState(
                displayState: .checkingContinue,
                nextAction: "Checking task and location",
                availableActions: [
                    IslandAvailableAction(kind: .openSmalltalk, label: "Open Smalltalk", enabled: true)
                ]
            )
        }
        if snapshot.state == "recording_compact" || snapshot.state == "recording_expanded" {
            return IslandContinueState(
                displayState: .localMemoryWarming,
                nextAction: "Collecting local evidence",
                availableActions: [
                    IslandAvailableAction(kind: .refreshContinue, label: "Continue", enabled: true),
                    IslandAvailableAction(kind: .openSmalltalk, label: "Open Smalltalk", enabled: true),
                    IslandAvailableAction(kind: .captureEvidenceNow, label: "Update local evidence", enabled: true),
                ]
            )
        }
        return IslandContinueState(
            displayState: .noLocalMemory,
            nextAction: "Local memory is off",
            availableActions: [
                IslandAvailableAction(kind: .openSmalltalk, label: "Open Smalltalk", enabled: true),
                IslandAvailableAction(kind: .startLocalMemory, label: "Start local memory", enabled: true),
            ]
        )
    }
}

private enum IslandDisplayState: String, Decodable, Equatable {
    case noLocalMemory = "no_local_memory"
    case localMemoryWarming = "local_memory_warming"
    case checkingContinue = "checking_continue"
    case continueReady = "continue_ready"
    case thinCurrentWork = "thin_current_work"
    case targetSuppressed = "target_suppressed"
    case supportBlocked = "support_blocked"
    case needsRefresh = "needs_refresh"
    case inspectOnly = "inspect_only"
    case noClearContinuation = "no_clear_continuation"
    case error

    init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer().decode(String.self)
        self = IslandDisplayState(rawValue: value) ?? .error
    }
}

private struct IslandFocusSummary: Decodable, Equatable {
    var title: String
    var subtitle: String?
    var appName: String?
    var windowTitle: String?
    var openability: String?

    enum CodingKeys: String, CodingKey {
        case title
        case subtitle
        case appName = "app_name"
        case windowTitle = "window_title"
        case openability
    }
}

private struct IslandTargetSummary: Decodable, Equatable {
    var title: String
    var subtitle: String?
    var artifactKind: String?
    var openability: String
    var openable: Bool

    enum CodingKeys: String, CodingKey {
        case title
        case subtitle
        case artifactKind = "artifact_kind"
        case openability
        case openable
    }
}

private struct IslandAvailableAction: Decodable, Equatable {
    var kind: IslandActionKind
    var label: String
    var enabled: Bool
    var reason: String?
    var decisionId: String?
    var taskSnapshotId: String?
    var taskSnapshotRevision: Int64?
    var affectedTaskField: String?
    var taskHypothesisId: String?

    enum CodingKeys: String, CodingKey {
        case kind
        case label
        case enabled
        case reason
        case decisionId = "decision_id"
        case taskSnapshotId = "task_snapshot_id"
        case taskSnapshotRevision = "task_snapshot_revision"
        case affectedTaskField = "affected_task_field"
        case taskHypothesisId = "task_hypothesis_id"
    }

    init(
        kind: IslandActionKind,
        label: String,
        enabled: Bool,
        reason: String? = nil,
        decisionId: String? = nil,
        taskSnapshotId: String? = nil,
        taskSnapshotRevision: Int64? = nil,
        affectedTaskField: String? = nil,
        taskHypothesisId: String? = nil
    ) {
        self.kind = kind
        self.label = label
        self.enabled = enabled
        self.reason = reason
        self.decisionId = decisionId
        self.taskSnapshotId = taskSnapshotId
        self.taskSnapshotRevision = taskSnapshotRevision
        self.affectedTaskField = affectedTaskField
        self.taskHypothesisId = taskHypothesisId
    }
}

private enum IslandActionKind: String, Decodable, Equatable {
    case refreshContinue = "refresh_continue"
    case openContinueTarget = "open_continue_target"
    case markWrongTarget = "mark_wrong_target"
    case markNotUseful = "mark_not_useful"
    case chooseTaskAlternative = "choose_task_alternative"
    case rejectSelectedTask = "reject_selected_task"
    case rejectTaskAlternative = "reject_task_alternative"
    case markSupportingWork = "mark_supporting_work"
    case markUnrelatedActivity = "mark_unrelated_activity"
    case markTaskCompleted = "mark_task_completed"
    case reactivateTask = "reactivate_task"
    case inspectEvidence = "inspect_evidence"
    case openSmalltalk = "open_smalltalk"
    case startLocalMemory = "start_local_memory"
    case captureEvidenceNow = "capture_evidence_now"
    case unknown

    init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer().decode(String.self)
        self = IslandActionKind(rawValue: value) ?? .unknown
    }
}

private extension String {
    var nonEmpty: String? {
        isEmpty ? nil : self
    }
}

private struct OverlayMetrics {
    var screenActive = false
    var captureFps = 0.0
    var meetingActive = false
}

private enum Brand {
    static func swiftUIFont(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        Font.system(size: size, weight: weight, design: .default)
    }
}

private enum IslandMotion {
    static func quick(_ reduceMotion: Bool) -> Animation {
        .timingCurve(0.23, 1, 0.32, 1, duration: reduceMotion ? 0.01 : 0.14)
    }

    static func memoryContinuityMorph(_ reduceMotion: Bool) -> Animation? {
        guard !reduceMotion else { return nil }
        return .timingCurve(
            0.77,
            0,
            0.175,
            1,
            duration: kWhisperFlowMicroAmbientTransitionDuration
        )
    }

    static func panelTimingFunction() -> CAMediaTimingFunction {
        CAMediaTimingFunction(controlPoints: 0.23, 1, 0.32, 1)
    }
}

@available(macOS 13.0, *)
private struct IslandMorphModifier: ViewModifier {
    let opacity: Double
    let scale: CGFloat

    func body(content: Content) -> some View {
        content
            .opacity(opacity)
            .scaleEffect(scale, anchor: .top)
    }
}

@available(macOS 13.0, *)
private struct TopAnchoredCapsuleShape: Shape {
    var width: CGFloat
    var height: CGFloat

    var animatableData: AnimatablePair<CGFloat, CGFloat> {
        get { AnimatablePair(width, height) }
        set {
            width = newValue.first
            height = newValue.second
        }
    }

    func path(in rect: CGRect) -> Path {
        let clampedWidth = min(max(0, width), rect.width)
        let clampedHeight = min(max(0, height), rect.height)
        let capsuleRect = CGRect(
            x: rect.midX - clampedWidth / 2,
            y: rect.minY,
            width: clampedWidth,
            height: clampedHeight
        )
        return Path(
            roundedRect: capsuleRect,
            cornerRadius: clampedHeight / 2,
            style: .continuous
        )
    }
}

private let kBaseMicroHitW: CGFloat = 86
private let kBaseMicroHitH: CGFloat = 24
private let kBaseMicroVisualW: CGFloat = 58
private let kBaseMicroVisualH: CGFloat = 10

@available(macOS 13.0, *)
private struct WhisperFlowPressButtonStyle: ButtonStyle {
    let reduceMotion: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.96 : 1)
            .animation(IslandMotion.quick(reduceMotion), value: configuration.isPressed)
    }
}

private let kWhisperFlowCapturePanelW: CGFloat = 187
private let kWhisperFlowCapturePanelH: CGFloat = 49
private let kWhisperFlowCaptureW: CGFloat = 168
private let kWhisperFlowCaptureH: CGFloat = 34
private let kWhisperFlowCaptureActionW: CGFloat = 28
private let kWhisperFlowCaptureActionH: CGFloat = 24
private let kWhisperFlowCaptureContentW: CGFloat = 123
private let kWhisperFlowDotMatrixSize: CGFloat = 11
private let kWhisperFlowCaptureStatusLabelW: CGFloat = 108
private let kWhisperFlowNotificationPanelW: CGFloat = 255
private let kWhisperFlowNotificationPanelH: CGFloat = 61
private let kWhisperFlowNotificationW: CGFloat = 236
private let kWhisperFlowNotificationH: CGFloat = 46
private let kWhisperFlowNotificationActionW: CGFloat = 32
private let kWhisperFlowNotificationActionH: CGFloat = 28
private let kWhisperFlowNotificationContentW: CGFloat = 175
private let kWhisperFlowNotificationDotMatrixSize: CGFloat = 13
private let kWhisperFlowNotificationStatusLabelW: CGFloat = 156
private let kWhisperFlowNotificationFontSize: CGFloat = 14
private let kWhisperFlowNotificationCountdownLineH: CGFloat = 2
private let kWhisperFlowMemoryTransitionDuration: TimeInterval = 3.0
private let kWhisperFlowAmbientHoverReturnDelay: TimeInterval = 1.0
private let kWhisperFlowCountdownLineH: CGFloat = 1
private let kWhisperFlowAmbientBodyHoverArmDelay: TimeInterval = 0.20
private let kWhisperFlowAnswerSummaryPanelH: CGFloat = 49
private let kWhisperFlowAnswerSummaryMinW: CGFloat = 152
private let kWhisperFlowAnswerSummaryH: CGFloat = 30
private let kWhisperFlowAnswerSummaryPanelMarginW: CGFloat = 35
private let kWhisperFlowAnswerExpandedMinW: CGFloat = 320
private let kWhisperFlowAnswerExpandedMaxW: CGFloat = 640
private let kWhisperFlowAnswerExpandedMinH: CGFloat = 104
private let kWhisperFlowAnswerExpandedMaxScreenFraction: CGFloat = 0.70
private let kWhisperFlowAnswerHorizontalPadding: CGFloat = 24
private let kWhisperFlowAnswerVerticalPadding: CGFloat = 20
private let kWhisperFlowAnswerHeaderSpacing: CGFloat = 18
private let kWhisperFlowAnswerRowSpacing: CGFloat = 14
private let kWhisperFlowAnswerLabelValueSpacing: CGFloat = 5
private let kWhisperFlowMicroPulseDuration: TimeInterval = 3.2
private let kWhisperFlowMicroPulseScale: CGFloat = 1.018
private let kWhisperFlowMicroPulseOutlineMinOpacity = 0.72
private let kWhisperFlowMorphDuration: TimeInterval = 0.18
private let kWhisperFlowMicroAmbientTransitionDuration: TimeInterval = 0.18
private let kWhisperFlowReducedMotionFadeDuration: TimeInterval = 0.12

private enum WhisperFlowPresentation: Equatable {
    case micro
    case ambientMemory
    case generating
    case answerSummary
    case answerExpanded
}

private enum WhisperFlowCaptureStatus: Equatable {
    case active
    case starting
    case processing
    case generating
    case inactive
    case suppressed
    case error
}

private enum WhisperFlowMemoryLifecyclePhase: Equatable {
    case paused
    case starting
    case active
    case stopping
    case unavailable
}

private enum WhisperFlowStyle {
    static let surface = Color(red: 0, green: 0, blue: 0)
    static let outline = Color(red: 48 / 255, green: 48 / 255, blue: 47 / 255)
    static let accent = Color(red: 245 / 255, green: 191 / 255, blue: 239 / 255)
}

private struct WhisperFlowAnswerRow: Equatable {
    let label: String
    let value: String
}

private struct WhisperFlowAnswerContent: Equatable {
    let decisionId: String?
    let title: String
    let rows: [WhisperFlowAnswerRow]

    static let unavailable = WhisperFlowAnswerContent(
        decisionId: nil,
        title: "Continue unavailable",
        rows: []
    )

    private init(decisionId: String?, title: String, rows: [WhisperFlowAnswerRow]) {
        self.decisionId = decisionId
        self.title = title
        self.rows = rows
    }

    init(snapshot: IslandSnapshot) {
        let state = snapshot.islandContinueState ?? IslandContinueState.fallback(from: snapshot)
        let answer = state.semanticAnswer
        decisionId = state.decisionId ?? snapshot.continueDecisionId
        title = Self.verbatim(answer?.taskSummary) ?? Self.fallbackTitle(for: state.displayState)

        var nextRows: [WhisperFlowAnswerRow] = []
        Self.append(&nextRows, label: "Task object", value: answer?.taskObject)
        Self.append(
            &nextRows,
            label: "Current activity — observed surface",
            value: answer?.currentActivity.observedSurface
        )
        Self.append(
            &nextRows,
            label: "Current activity — immediate operation",
            value: answer?.currentActivity.immediateUserOperation
        )
        Self.append(
            &nextRows,
            label: "Current activity — operation effect",
            value: answer?.currentActivity.semanticEffectOfOperation
        )
        Self.append(
            &nextRows,
            label: "Current activity — current subtask",
            value: answer?.currentActivity.currentSubtask
        )
        Self.append(
            &nextRows,
            label: "Current activity — relationship to primary",
            value: answer?.currentActivity.relationshipToPrimary
        )
        Self.append(&nextRows, label: "Last meaningful progress", value: answer?.lastMeaningfulProgress)
        Self.append(&nextRows, label: "Unfinished state", value: answer?.unfinishedState)
        Self.append(&nextRows, label: "Next action", value: answer?.nextAction ?? state.nextAction)
        Self.append(&nextRows, label: "Where summary", value: answer?.whereSummary)
        rows = nextRows
    }

    private static func append(
        _ rows: inout [WhisperFlowAnswerRow],
        label: String,
        value: String?
    ) {
        guard let value = verbatim(value) else { return }
        rows.append(WhisperFlowAnswerRow(label: label, value: value))
    }

    private static func verbatim(_ value: String?) -> String? {
        guard let value,
              !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return nil }
        return value
    }

    private static func fallbackTitle(for state: IslandDisplayState) -> String {
        switch state {
        case .error:
            return "Continue unavailable"
        case .noLocalMemory, .localMemoryWarming:
            return "Not enough local memory"
        case .needsRefresh:
            return "Continue needs refreshing"
        case .checkingContinue:
            return "Generating answer…"
        case .continueReady,
             .thinCurrentWork,
             .targetSuppressed,
             .supportBlocked,
             .inspectOnly,
             .noClearContinuation:
            return "Couldn’t recover the task"
        }
    }
}

private struct WhisperFlowAnswerLayout: Equatable {
    var summaryWidth: CGFloat = kWhisperFlowAnswerSummaryMinW
    var summaryPanelWidth: CGFloat = kWhisperFlowAnswerSummaryMinW
        + kWhisperFlowAnswerSummaryPanelMarginW
    var expandedWidth: CGFloat = kWhisperFlowAnswerExpandedMinW
    var expandedHeight: CGFloat = kWhisperFlowAnswerExpandedMinH
    var contentViewportHeight: CGFloat = kWhisperFlowAnswerExpandedMinH
        - kWhisperFlowAnswerVerticalPadding * 2
}

@available(macOS 13.0, *)
private final class WhisperFlowIslandModel: ObservableObject {
    @Published var presentation: WhisperFlowPresentation = .micro
    @Published var isPanelVisible = false
    @Published var memoryActive = false
    @Published var memoryHasStarted = false
    @Published var microPulseActive = false
    @Published var capturePulseNonce: UInt64?
    @Published var startingFeedbackNonce: UInt64?
    @Published var captureStatus: WhisperFlowCaptureStatus = .inactive
    @Published var memoryTransitionCountdownActive = false
    @Published var memoryTransitionCountdownNonce: UInt64 = 0
    @Published var continueGenerating = false
    @Published var answer: WhisperFlowAnswerContent?
    @Published var answerLayout = WhisperFlowAnswerLayout()
}

@available(macOS 13.0, *)
private final class DotMatrixIndicatorView: NSView {
    private struct Configuration: Equatable {
        let status: WhisperFlowCaptureStatus
        let capturePulseNonce: UInt64?
        let startingFeedbackNonce: UInt64?
        let panelVisible: Bool
        let reduceMotion: Bool
        let restartInvited: Bool
    }

    private static let captureAnimationKey = "smalltalk.dot-matrix-capture"
    private static let startingAnimationKey = "smalltalk.dot-matrix-starting"
    private static let generatingAnimationKey = "smalltalk.dot-matrix-generating"
    private static let inactiveOpacity: Float = 0.15
    private static let activeOpacity: Float = 0.82
    private static let capturePulseDuration: TimeInterval = 0.72
    private static let capturePulseCooldown: TimeInterval = 1.75
    private static let startingDuration: TimeInterval = 0.60
    private static let reducedMotionDuration: TimeInterval = 0.40
    private static let gatherFraction: CGFloat = 0.32
    private static let activePattern = Set([7, 11, 12, 13, 17])
    private static let pausedPattern = Set([6, 8, 11, 13, 16, 18])
    private static let filteredPattern = Set([1, 3, 5, 9, 10, 14, 16, 18, 22])
    private static let errorPattern = Set([2, 7, 12, 22])
    private static let generatingPattern = Set([2, 3, 4, 9, 14])
    private static let generatingPerimeter = [
        20, 21, 22, 23, 24, 19, 14, 9, 4, 3, 2, 1, 0, 5, 10, 15,
    ]
    private static let generatingDuration: TimeInterval = 0.82
    private static let restartPattern = Set([1, 6, 7, 11, 12, 13, 16, 17, 21])
    private static let restartTransitionDuration: TimeInterval = 0.12

    private let dotLayers: [CAShapeLayer]
    private var configuration: Configuration?
    private var restingPositions = Array(repeating: CGPoint.zero, count: 25)
    private var hasCapturePulseBaseline = false
    private var latestCapturePulseNonce: UInt64?
    private var latestStartingFeedbackNonce: UInt64?
    private var lastCapturePulseAt: CFTimeInterval?
    private var pendingCapturePulseNonce: UInt64?
    private var pendingCapturePulseWorkItem: DispatchWorkItem?
    private var startingAnimationPending = false
    private var startingAnimationHasRunForCurrentState = false
    private var startingAnimationEndsAt: CFTimeInterval?

    override init(frame frameRect: NSRect) {
        var layers: [CAShapeLayer] = []
        for _ in 0..<25 {
            let dot = CAShapeLayer()
            dot.backgroundColor = NSColor.white.cgColor
            dot.opacity = Self.inactiveOpacity
            layers.append(dot)
        }
        dotLayers = layers
        super.init(frame: frameRect)

        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
        for dot in dotLayers {
            layer?.addSublayer(dot)
        }
        setAccessibilityElement(false)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    deinit {
        cancelAnimationsAndPendingPulse()
    }

    override var isOpaque: Bool { false }

    override func layout() {
        super.layout()
        let side = min(bounds.width, bounds.height)
        let dotDiameter = side * 0.145
        let gap = max(0, (side - dotDiameter * 5) / 4)
        let originX = (bounds.width - side) / 2
        let originY = (bounds.height - side) / 2

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for row in 0..<5 {
            for column in 0..<5 {
                let index = row * 5 + column
                dotLayers[index].frame = CGRect(
                    x: originX + CGFloat(column) * (dotDiameter + gap),
                    y: originY + CGFloat(row) * (dotDiameter + gap),
                    width: dotDiameter,
                    height: dotDiameter
                )
                dotLayers[index].cornerRadius = dotDiameter / 2
                dotLayers[index].contentsScale = window?.backingScaleFactor
                    ?? NSScreen.main?.backingScaleFactor
                    ?? 2
                restingPositions[index] = dotLayers[index].position
            }
        }
        CATransaction.commit()
        applyStaticState()
        if startingAnimationPending {
            runStartingAnimation()
        } else if configuration?.status == .generating {
            runGeneratingAnimation()
        }
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard window != nil else {
            cancelAnimationsAndPendingPulse()
            return
        }
        applyStaticState()
        if configuration?.status == .starting, !startingAnimationHasRunForCurrentState {
            startingAnimationPending = true
            runStartingAnimation()
        } else if configuration?.status == .generating {
            runGeneratingAnimation()
        }
    }

    override func viewDidMoveToSuperview() {
        super.viewDidMoveToSuperview()
        if superview == nil {
            cancelAnimationsAndPendingPulse()
        }
    }

    func configure(
        status: WhisperFlowCaptureStatus,
        capturePulseNonce: UInt64?,
        startingFeedbackNonce: UInt64?,
        panelVisible: Bool,
        reduceMotion: Bool,
        restartInvited: Bool
    ) {
        let next = Configuration(
            status: status,
            capturePulseNonce: capturePulseNonce,
            startingFeedbackNonce: startingFeedbackNonce,
            panelVisible: panelVisible,
            reduceMotion: reduceMotion,
            restartInvited: restartInvited
        )
        guard next != configuration else { return }
        let previous = configuration
        configuration = next

        let statusChanged = previous?.status != status
        let panelVisibilityChanged = previous?.panelVisible != panelVisible
        let reduceMotionChanged = previous?.reduceMotion != reduceMotion
        let preservesStartingAnimation = previous?.status == .starting &&
            status == .active &&
            !panelVisibilityChanged &&
            !reduceMotionChanged

        if panelVisibilityChanged || (statusChanged && !preservesStartingAnimation) {
            removeLayerAnimations()
            if statusChanged {
                startingAnimationHasRunForCurrentState = false
            }
            startingAnimationEndsAt = nil
            startingAnimationPending = status == .starting &&
                panelVisible &&
                !startingAnimationHasRunForCurrentState
            if status != .active {
                cancelPendingCapturePulse()
            }
            applyStaticState()
            if startingAnimationPending, window != nil {
                runStartingAnimation()
            } else if status == .generating, window != nil {
                runGeneratingAnimation()
            }
        } else if reduceMotionChanged {
            removeLayerAnimations()
            startingAnimationEndsAt = nil
            applyStaticState()
            if status == .generating, window != nil {
                runGeneratingAnimation()
            }
        } else if preservesStartingAnimation,
                  startingAnimationPending,
                  window != nil {
            runStartingAnimation()
        } else if previous?.restartInvited != restartInvited {
            applyRestartPatternTransition()
        }

        // A confirmed start event is controller-owned so it survives the
        // matrix view being absent in micro. This lets a newly created active
        // matrix play the activation even when `starting` already became
        // `active` before SwiftUI constructed the view.
        if let startingFeedbackNonce,
           startingFeedbackNonce != latestStartingFeedbackNonce {
            latestStartingFeedbackNonce = startingFeedbackNonce
            if (status == .starting || status == .active),
               panelVisible,
               !startingAnimationHasRunForCurrentState {
                startingAnimationPending = true
                if window != nil {
                    runStartingAnimation()
                }
            }
        }

        // Process capture events only after status cleanup. Otherwise a pulse
        // started above can be removed later in this same configuration pass.
        if !hasCapturePulseBaseline, let capturePulseNonce {
            hasCapturePulseBaseline = true
            latestCapturePulseNonce = capturePulseNonce
        } else if let capturePulseNonce, capturePulseNonce != latestCapturePulseNonce {
            latestCapturePulseNonce = capturePulseNonce
            requestCapturePulse(capturePulseNonce)
        }
    }

    private var shouldReduceMotion: Bool {
        configuration?.reduceMotion == true ||
            NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
    }

    private func pattern(for status: WhisperFlowCaptureStatus) -> Set<Int> {
        if status == .inactive, configuration?.restartInvited == true {
            return Self.restartPattern
        }
        switch status {
        case .active, .starting:
            return Self.activePattern
        case .processing, .inactive:
            return Self.pausedPattern
        case .generating:
            return Self.generatingPattern
        case .suppressed:
            return Self.filteredPattern
        case .error:
            return Self.errorPattern
        }
    }

    private func requestCapturePulse(_ nonce: UInt64) {
        guard let configuration,
              configuration.status == .active,
              configuration.panelVisible,
              window != nil else { return }

        let now = CACurrentMediaTime()
        if startingAnimationPending {
            pendingCapturePulseNonce = nonce
            return
        }

        let cooldownReadyAt = lastCapturePulseAt.map {
            $0 + Self.capturePulseCooldown
        } ?? now
        let startingReadyAt = startingAnimationEndsAt ?? now
        let readyAt = max(cooldownReadyAt, startingReadyAt)
        if now >= readyAt {
            runCapturePulse(nonce)
            return
        }

        pendingCapturePulseNonce = nonce
        guard pendingCapturePulseWorkItem == nil else { return }
        let delay = readyAt - now
        let workItem = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.pendingCapturePulseWorkItem = nil
            guard let pendingNonce = self.pendingCapturePulseNonce else { return }
            self.pendingCapturePulseNonce = nil
            guard self.configuration?.status == .active,
                  self.configuration?.panelVisible == true,
                  self.window != nil else { return }
            self.requestCapturePulse(pendingNonce)
        }
        pendingCapturePulseWorkItem = workItem
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: workItem)
    }

    private func runCapturePulse(_ nonce: UInt64) {
        guard latestCapturePulseNonce == nonce || pendingCapturePulseNonce == nil else { return }
        lastCapturePulseAt = CACurrentMediaTime()
        removeLayerAnimations()
        applyStaticState()

        if shouldReduceMotion {
            let center = dotLayers[12]
            let animation = CAKeyframeAnimation(keyPath: "opacity")
            animation.values = [Self.activeOpacity, 1.0, Self.activeOpacity]
            animation.keyTimes = [0, 0.5, 1]
            animation.duration = Self.reducedMotionDuration
            animation.timingFunctions = [
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
            ]
            center.add(animation, forKey: Self.captureAnimationKey)
            return
        }

        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let center = restingPositions[12]
        for (index, dot) in dotLayers.enumerated() {
            let rest = restingPositions[index]
            let gathered = CGPoint(
                x: rest.x + (center.x - rest.x) * Self.gatherFraction,
                y: rest.y + (center.y - rest.y) * Self.gatherFraction
            )
            let baseOpacity = staticOpacity(for: index)
            let peakOpacity: Float = index == 12
                ? 1.0
                : Self.activePattern.contains(index) ? 0.95 : 0.55

            let position = CAKeyframeAnimation(keyPath: "position")
            position.values = [rest, gathered, rest]
            position.keyTimes = [0, 0.45, 1]
            position.timingFunctions = [
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
            ]

            let opacity = CAKeyframeAnimation(keyPath: "opacity")
            opacity.values = [baseOpacity, peakOpacity, baseOpacity]
            opacity.keyTimes = [0, 0.45, 1]
            opacity.timingFunctions = [
                CAMediaTimingFunction(name: .easeInEaseOut),
                CAMediaTimingFunction(name: .easeInEaseOut),
            ]

            let group = CAAnimationGroup()
            group.animations = [position, opacity]
            group.duration = Self.capturePulseDuration
            dot.position = rest
            dot.opacity = baseOpacity
            dot.add(group, forKey: Self.captureAnimationKey)
        }
        CATransaction.commit()
    }

    private func runStartingAnimation() {
        guard !startingAnimationHasRunForCurrentState else {
            startingAnimationPending = false
            return
        }
        guard bounds.width > 0, bounds.height > 0 else {
            startingAnimationPending = true
            return
        }
        startingAnimationPending = false
        startingAnimationHasRunForCurrentState = true
        removeLayerAnimations()
        applyStaticState()
        if shouldReduceMotion {
            startingAnimationEndsAt = CACurrentMediaTime() + Self.reducedMotionDuration
            let center = dotLayers[12]
            let animation = CAKeyframeAnimation(keyPath: "opacity")
            animation.values = [Self.inactiveOpacity, 1.0, Self.activeOpacity]
            animation.keyTimes = [0, 0.5, 1]
            animation.duration = Self.reducedMotionDuration
            center.add(animation, forKey: Self.startingAnimationKey)
            schedulePendingCapturePulseAfterStartingIfNeeded()
            return
        }

        let center = restingPositions[12]
        let sharedStartTime = CACurrentMediaTime() + 0.02
        startingAnimationEndsAt = sharedStartTime + Self.startingDuration
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for row in 0..<5 {
            for column in 0..<5 {
                let index = row * 5 + column
                let dot = dotLayers[index]
                let rest = restingPositions[index]
                let gathered = CGPoint(
                    x: rest.x + (center.x - rest.x) * 0.18,
                    y: rest.y + (center.y - rest.y) * 0.18
                )
                let distance = abs(row - 2) + abs(column - 2)
                let inwardDelay = TimeInterval(max(0, 4 - distance)) * 0.045
                let baseOpacity = staticOpacity(for: index)

                let position = CAKeyframeAnimation(keyPath: "position")
                position.values = [rest, gathered, rest]
                position.keyTimes = [0, 0.45, 1]
                position.timingFunctions = [
                    CAMediaTimingFunction(name: .easeInEaseOut),
                    CAMediaTimingFunction(name: .easeInEaseOut),
                ]

                let opacity = CAKeyframeAnimation(keyPath: "opacity")
                opacity.values = [Self.inactiveOpacity, 0.72, baseOpacity]
                opacity.keyTimes = [0, 0.45, 1]
                opacity.timingFunctions = [
                    CAMediaTimingFunction(name: .easeInEaseOut),
                    CAMediaTimingFunction(name: .easeInEaseOut),
                ]

                let group = CAAnimationGroup()
                group.animations = [position, opacity]
                group.duration = Self.startingDuration - inwardDelay
                group.beginTime = sharedStartTime + inwardDelay
                dot.position = rest
                dot.opacity = baseOpacity
                dot.add(group, forKey: Self.startingAnimationKey)
            }
        }
        CATransaction.commit()
        schedulePendingCapturePulseAfterStartingIfNeeded()
    }

    private func runGeneratingAnimation() {
        guard configuration?.status == .generating,
              configuration?.panelVisible == true,
              window != nil else { return }
        removeLayerAnimations()
        applyStaticState()
        guard !shouldReduceMotion else { return }

        let sharedStartTime = CACurrentMediaTime()
        let phaseStep = Self.generatingDuration
            / TimeInterval(Self.generatingPerimeter.count)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (position, index) in Self.generatingPerimeter.enumerated() {
            let dot = dotLayers[index]
            let animation = CAKeyframeAnimation(keyPath: "opacity")
            animation.values = [
                Self.inactiveOpacity,
                Self.inactiveOpacity,
                1.0,
                0.42,
                Self.inactiveOpacity,
            ]
            animation.keyTimes = [0, 0.48, 0.58, 0.72, 1]
            animation.duration = Self.generatingDuration
            animation.beginTime = sharedStartTime
            animation.timeOffset = TimeInterval(position) * phaseStep
            animation.repeatCount = .infinity
            animation.timingFunctions = [
                CAMediaTimingFunction(name: .linear),
                CAMediaTimingFunction(name: .linear),
                CAMediaTimingFunction(name: .easeOut),
                CAMediaTimingFunction(name: .linear),
            ]
            dot.opacity = Self.inactiveOpacity
            dot.add(animation, forKey: Self.generatingAnimationKey)
        }
        CATransaction.commit()
    }

    private func schedulePendingCapturePulseAfterStartingIfNeeded() {
        guard configuration?.status == .active,
              let pendingNonce = pendingCapturePulseNonce else { return }
        requestCapturePulse(pendingNonce)
    }

    private func staticOpacity(for index: Int) -> Float {
        let status = configuration?.status ?? .inactive
        return pattern(for: status).contains(index) ? Self.activeOpacity : Self.inactiveOpacity
    }

    private func removeLayerAnimations() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (index, dot) in dotLayers.enumerated() {
            dot.removeAllAnimations()
            dot.position = restingPositions[index]
        }
        CATransaction.commit()
    }

    private func applyStaticState() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (index, dot) in dotLayers.enumerated() {
            dot.position = restingPositions[index]
            dot.opacity = staticOpacity(for: index)
        }
        CATransaction.commit()
    }

    private func applyRestartPatternTransition() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (index, dot) in dotLayers.enumerated() {
            let targetOpacity = staticOpacity(for: index)
            let animation = CABasicAnimation(keyPath: "opacity")
            animation.fromValue = dot.presentation()?.opacity ?? dot.opacity
            animation.toValue = targetOpacity
            animation.duration = Self.restartTransitionDuration
            animation.timingFunction = CAMediaTimingFunction(name: .easeOut)
            dot.opacity = targetOpacity
            dot.add(animation, forKey: "smalltalk.dot-matrix-restart")
        }
        CATransaction.commit()
    }

    private func cancelPendingCapturePulse() {
        pendingCapturePulseWorkItem?.cancel()
        pendingCapturePulseWorkItem = nil
        pendingCapturePulseNonce = nil
    }

    private func cancelAnimationsAndPendingPulse() {
        cancelPendingCapturePulse()
        startingAnimationEndsAt = nil
        removeLayerAnimations()
    }
}

@available(macOS 13.0, *)
private struct DotMatrixIndicator: NSViewRepresentable {
    let status: WhisperFlowCaptureStatus
    let capturePulseNonce: UInt64?
    let startingFeedbackNonce: UInt64?
    let panelVisible: Bool
    let reduceMotion: Bool
    let restartInvited: Bool

    func makeNSView(context: Context) -> DotMatrixIndicatorView {
        DotMatrixIndicatorView(frame: .zero)
    }

    func updateNSView(_ nsView: DotMatrixIndicatorView, context: Context) {
        nsView.configure(
            status: status,
            capturePulseNonce: capturePulseNonce,
            startingFeedbackNonce: startingFeedbackNonce,
            panelVisible: panelVisible,
            reduceMotion: reduceMotion,
            restartInvited: restartInvited
        )
    }

    static func dismantleNSView(_ nsView: DotMatrixIndicatorView, coordinator: ()) {
        nsView.configure(
            status: .inactive,
            capturePulseNonce: nil,
            startingFeedbackNonce: nil,
            panelVisible: false,
            reduceMotion: true,
            restartInvited: false
        )
    }
}

@available(macOS 13.0, *)
private struct WhisperFlowIslandView: View {
    let scale: CGFloat
    @ObservedObject var model: WhisperFlowIslandModel
    let onRevealAmbientMemory: () -> Void
    let onMicroHover: (Bool) -> Void
    let onReadyAction: () -> Void
    let onStartMemory: () -> Void
    let onAmbientHover: (Bool) -> Void
    let onAmbientBodyHover: (Bool) -> Void
    let onExpandAnswer: () -> Void
    let onCollapseAnswer: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var microPulseExpanded = false
    @State private var ambientBodyHovered = false
    @State private var ambientBodyHoverArmed = false
    @State private var blockedAmbientBodyHoverObserved = false
    @State private var ambientCapsuleHovered = false
    @State private var pausedRestartHovered = false
    @State private var arrowHovered = false
    @State private var memoryTransitionCountdownProgress: CGFloat = 0

    private func s(_ value: CGFloat) -> CGFloat { value * scale }

    var body: some View {
        ZStack(alignment: .top) {
            switch model.presentation {
            case .micro, .ambientMemory, .generating:
                memoryContinuityView
            case .answerSummary:
                answerSummaryView
                    .transition(.opacity)
            case .answerExpanded:
                answerExpandedView
                    .transition(stateTransition(scale: 0.97))
            }
        }
        .fixedSize()
        .background(Color.clear)
        .animation(morphAnimation, value: model.presentation)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var memoryContinuityView: some View {
        let expanded = model.presentation == .ambientMemory
            || model.presentation == .generating
        let visualWidth = expanded
            ? ambientCapsuleWidth
            : kBaseMicroVisualW * microScale
        let visualHeight = expanded
            ? ambientCapsuleHeight
            : kBaseMicroVisualH * microScale

        return ZStack(alignment: .top) {
            TopAnchoredCapsuleShape(
                width: s(visualWidth),
                height: s(visualHeight)
            )
            .fill(WhisperFlowStyle.surface)
            .allowsHitTesting(false)

            TopAnchoredCapsuleShape(
                width: s(visualWidth),
                height: s(visualHeight)
            )
            .stroke(
                WhisperFlowStyle.outline.opacity(
                    expanded ? 1 : microOutlineOpacity
                ),
                lineWidth: s(1)
            )
            .allowsHitTesting(false)

            ambientMemoryContent
                .opacity(expanded ? 1 : 0)
                .allowsHitTesting(expanded)
                .accessibilityHidden(!expanded)
                .animation(ambientContentAnimation(expanded: expanded), value: expanded)

            microHitTarget
                .opacity(expanded ? 0 : 1)
                .allowsHitTesting(!expanded)
                .accessibilityHidden(expanded)
        }
        .frame(
            width: s(ambientPanelWidth),
            height: s(ambientPanelHeight),
            alignment: .top
        )
        .contentShape(Rectangle())
        .animation(
            IslandMotion.memoryContinuityMorph(shouldReduceMotion),
            value: visualWidth
        )
        .animation(
            IslandMotion.memoryContinuityMorph(shouldReduceMotion),
            value: visualHeight
        )
        .onAppear {
            handleMemoryPresentationChange(model.presentation)
        }
        .onChange(of: model.presentation) { presentation in
            handleMemoryPresentationChange(presentation)
        }
        .onDisappear {
            microPulseExpanded = false
            cleanUpAmbientInteractionState()
        }
    }

    private var microHitTarget: some View {
        Button(action: onRevealAmbientMemory) {
            Rectangle()
                .fill(Color.clear)
                .frame(
                    width: s(kBaseMicroHitW),
                    height: s(kBaseMicroHitH),
                    alignment: .top
                )
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(model.memoryActive ? "Smalltalk memory is active" : "Show Smalltalk")
        .onHover(perform: onMicroHover)
    }

    private var ambientMemoryContent: some View {
        HStack(spacing: s(ambientControlSpacing)) {
            ambientStatusRegion

            if model.continueGenerating {
                Color.clear
                    .frame(
                        width: s(ambientActionWidth),
                        height: s(ambientActionHeight)
                    )
                    .accessibilityHidden(true)
            } else {
                Button(action: onReadyAction) {
                    Image(systemName: "arrow.right")
                        .font(.system(size: s(ambientArrowFontSize), weight: .semibold))
                        .foregroundColor(.white)
                        .frame(
                            width: s(ambientActionWidth),
                            height: s(ambientActionHeight)
                        )
                        .background(
                            Capsule()
                                .fill(
                                    arrowHovered
                                        ? Color(red: 58 / 255, green: 58 / 255, blue: 56 / 255)
                                        : WhisperFlowStyle.outline
                                )
                        )
                        .contentShape(Capsule())
                }
                .buttonStyle(WhisperFlowPressButtonStyle(reduceMotion: reduceMotion))
                .accessibilityLabel("Show what I was doing")
                .help("Show what I was doing")
                .onHover { hovering in
                    arrowHovered = hovering
                    if hovering {
                        NSCursor.pointingHand.set()
                    } else {
                        NSCursor.arrow.set()
                    }
                }
            }
        }
        .padding(.leading, s(ambientLeadingPadding))
        .padding(.trailing, s(ambientTrailingPadding))
        .frame(
            width: s(ambientCapsuleWidth),
            height: s(ambientCapsuleHeight)
        )
        .overlay(alignment: .bottomLeading) {
            memoryTransitionCountdownLine
        }
        .clipShape(Capsule())
        .contentShape(Capsule())
        .onHover { hovering in
            ambientCapsuleHovered = hovering
            onAmbientHover(hovering)
        }
        .animation(
            IslandMotion.memoryContinuityMorph(shouldReduceMotion),
            value: model.memoryTransitionCountdownActive || model.continueGenerating
        )
        .accessibilityElement(children: .contain)
        .onChange(of: model.captureStatus) { status in
            if status != .inactive {
                pausedRestartHovered = false
            }
            ambientBodyHovered = false
            onAmbientBodyHover(false)
            if status == .active {
                resetAmbientBodyHoverGate()
            } else {
                ambientBodyHoverArmed = false
                blockedAmbientBodyHoverObserved = false
            }
        }
        .onChange(of: model.memoryTransitionCountdownNonce) { _ in
            refreshMemoryTransitionCountdown()
        }
        .onChange(of: model.memoryTransitionCountdownActive) { active in
            if !active {
                memoryTransitionCountdownProgress = 0
            }
        }
    }

    @ViewBuilder
    private var ambientStatusRegion: some View {
        if model.captureStatus == .inactive {
            Button(action: onStartMemory) {
                ambientStatusContent
            }
            .buttonStyle(WhisperFlowPressButtonStyle(reduceMotion: reduceMotion))
            .accessibilityLabel("Start memory")
            .help("Start memory")
            .onHover { hovering in
                if hovering {
                    pausedRestartHovered = true
                    NSCursor.pointingHand.set()
                } else {
                    NSCursor.arrow.set()
                    DispatchQueue.main.async {
                        guard model.presentation == .ambientMemory,
                              ambientCapsuleHovered,
                              model.captureStatus == .inactive else { return }
                        pausedRestartHovered = false
                    }
                }
            }
        } else {
            ambientStatusContent
                .onHover { hovering in
                    if hovering {
                        if ambientBodyHoverArmed {
                            ambientBodyHovered = true
                            onAmbientBodyHover(model.captureStatus == .active)
                        } else {
                            blockedAmbientBodyHoverObserved = true
                        }
                        NSCursor.arrow.set()
                    } else {
                        if blockedAmbientBodyHoverObserved {
                            ambientBodyHoverArmed = true
                            blockedAmbientBodyHoverObserved = false
                        }
                        // The parent capsule receives its hover-out in the same
                        // event turn. Defer contraction so a dismissal can move
                        // straight to micro without exposing normal medium first.
                        DispatchQueue.main.async {
                            guard model.presentation == .ambientMemory,
                                  ambientCapsuleHovered else { return }
                            ambientBodyHovered = false
                            onAmbientBodyHover(false)
                        }
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel(displayedStatusPhrase)
        }
    }

    private var ambientStatusContent: some View {
        HStack(spacing: s(ambientStatusSpacing)) {
            DotMatrixIndicator(
                status: model.captureStatus,
                capturePulseNonce: model.capturePulseNonce,
                startingFeedbackNonce: model.startingFeedbackNonce,
                panelVisible: model.isPanelVisible,
                reduceMotion: shouldReduceMotion,
                restartInvited: pausedRestartHovered && model.captureStatus == .inactive
            )
            .frame(
                width: s(ambientDotMatrixSize),
                height: s(ambientDotMatrixSize)
            )
            .accessibilityHidden(true)

            ZStack(alignment: .leading) {
                Text(displayedStatusPhrase)
                    .id(displayedStatusPhrase)
                    .font(Brand.swiftUIFont(size: s(ambientFontSize), weight: .semibold))
                    .foregroundColor(.white)
                    .lineLimit(1)
                    .fixedSize(horizontal: true, vertical: false)
                    .transition(.opacity)
            }
            .frame(
                width: s(ambientStatusLabelWidth),
                height: s(ambientActionHeight),
                alignment: .leading
            )
            .clipped()
            .animation(statusCopyAnimation, value: displayedStatusPhrase)
        }
        .frame(
            width: s(ambientContentWidth),
            height: s(ambientActionHeight),
            alignment: .leading
        )
        .contentShape(Rectangle())
    }

    private var memoryTransitionCountdownLine: some View {
        Rectangle()
            .fill(Color.white.opacity(0.88))
            .frame(maxWidth: .infinity)
            .frame(height: s(ambientCountdownLineHeight))
            .scaleEffect(
                x: model.memoryTransitionCountdownActive
                    ? memoryTransitionCountdownProgress
                    : 0,
                y: 1,
                anchor: .leading
            )
            .allowsHitTesting(false)
    }

    private var ambientCapsuleWidth: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationW
            : kWhisperFlowCaptureW
    }

    private var ambientCapsuleHeight: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationH
            : kWhisperFlowCaptureH
    }

    private var ambientPanelWidth: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationPanelW
            : kWhisperFlowCapturePanelW
    }

    private var ambientPanelHeight: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationPanelH
            : kWhisperFlowCapturePanelH
    }

    private var ambientContentWidth: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationContentW
            : kWhisperFlowCaptureContentW
    }

    private var ambientStatusLabelWidth: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationStatusLabelW
            : kWhisperFlowCaptureStatusLabelW
    }

    private var ambientActionWidth: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationActionW
            : kWhisperFlowCaptureActionW
    }

    private var ambientActionHeight: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationActionH
            : kWhisperFlowCaptureActionH
    }

    private var ambientDotMatrixSize: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationDotMatrixSize
            : kWhisperFlowDotMatrixSize
    }

    private var ambientFontSize: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationFontSize
            : 12
    }

    private var ambientCountdownLineHeight: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating
            ? kWhisperFlowNotificationCountdownLineH
            : kWhisperFlowCountdownLineH
    }

    private var ambientControlSpacing: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating ? 7 : 5
    }

    private var ambientStatusSpacing: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating ? 5 : 4
    }

    private var ambientLeadingPadding: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating ? 10 : 8
    }

    private var ambientTrailingPadding: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating ? 6 : 4
    }

    private var ambientArrowFontSize: CGFloat {
        model.memoryTransitionCountdownActive || model.continueGenerating ? 13 : 11
    }

    private func refreshMemoryTransitionCountdown() {
        guard model.memoryTransitionCountdownActive else {
            memoryTransitionCountdownProgress = 0
            return
        }

        var resetTransaction = Transaction()
        resetTransaction.animation = nil
        withTransaction(resetTransaction) {
            memoryTransitionCountdownProgress = 1
        }
        DispatchQueue.main.async {
            guard model.memoryTransitionCountdownActive else { return }
            withAnimation(.linear(duration: kWhisperFlowMemoryTransitionDuration)) {
                memoryTransitionCountdownProgress = 0
            }
        }
    }

    private func prepareAmbientAppearance() {
        var resetTransaction = Transaction()
        resetTransaction.animation = nil
        withTransaction(resetTransaction) {
            ambientBodyHovered = false
            ambientBodyHoverArmed = false
            blockedAmbientBodyHoverObserved = false
            ambientCapsuleHovered = false
            pausedRestartHovered = false
            arrowHovered = false
        }
        resetAmbientBodyHoverGate()
        refreshMemoryTransitionCountdown()
    }

    private func ambientContentAnimation(expanded: Bool) -> Animation? {
        if shouldReduceMotion {
            return .easeOut(duration: kWhisperFlowReducedMotionFadeDuration)
        }
        return expanded
            ? .easeOut(duration: 0.12).delay(0.04)
            : .easeOut(duration: 0.10)
    }

    private func handleMemoryPresentationChange(
        _ presentation: WhisperFlowPresentation
    ) {
        switch presentation {
        case .micro:
            arrowHovered = false
            onAmbientHover(false)
            NSCursor.arrow.set()
            scheduleAmbientLocalStateCleanup()
            refreshMicroPulse()
        case .ambientMemory:
            microPulseExpanded = false
            prepareAmbientAppearance()
        case .generating:
            microPulseExpanded = false
            arrowHovered = false
            prepareAmbientAppearance()
        case .answerSummary, .answerExpanded:
            break
        }
    }

    private func cleanUpAmbientInteractionState() {
        arrowHovered = false
        ambientBodyHovered = false
        ambientBodyHoverArmed = false
        blockedAmbientBodyHoverObserved = false
        ambientCapsuleHovered = false
        pausedRestartHovered = false
        onAmbientHover(false)
        onAmbientBodyHover(false)
        NSCursor.arrow.set()
    }

    private func resetAmbientBodyHoverGate() {
        ambientBodyHoverArmed = false
        blockedAmbientBodyHoverObserved = false
        DispatchQueue.main.asyncAfter(
            deadline: .now() + kWhisperFlowAmbientBodyHoverArmDelay
        ) {
            guard model.presentation == .ambientMemory,
                  model.captureStatus == .active,
                  !blockedAmbientBodyHoverObserved else { return }
            ambientBodyHoverArmed = true
        }
    }

    private func scheduleAmbientLocalStateCleanup() {
        let delay = shouldReduceMotion ? 0 : kWhisperFlowMicroAmbientTransitionDuration
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
            guard model.presentation != .ambientMemory else { return }
            ambientBodyHovered = false
            ambientBodyHoverArmed = false
            blockedAmbientBodyHoverObserved = false
            ambientCapsuleHovered = false
            pausedRestartHovered = false
        }
    }

    private var shouldReduceMotion: Bool {
        reduceMotion || NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
    }

    private var displayedStatusPhrase: String {
        switch model.captureStatus {
        case .active:
            return "Capturing context"
        case .starting:
            return "Starting memory…"
        case .processing:
            return "Pausing memory…"
        case .generating:
            return "Generating answer…"
        case .inactive:
            return model.memoryHasStarted ? "Memory paused" : "Start memory"
        case .suppressed:
            return "Not saving this app"
        case .error:
            return "Memory needs attention"
        }
    }

    private var ambientMorphAnimation: Animation? {
        guard !shouldReduceMotion else { return nil }
        return .timingCurve(
            0.23,
            1,
            0.32,
            1,
            duration: kWhisperFlowMorphDuration
        )
    }

    private var statusCopyAnimation: Animation? {
        if shouldReduceMotion {
            return .easeOut(duration: 0.12)
        }
        return ambientMorphAnimation
    }

    private var answerSummaryView: some View {
        let answer = model.answer ?? .unavailable

        return HStack(spacing: s(2)) {
            Text(answer.title)
                .foregroundColor(.white)
                .lineLimit(1)
                .truncationMode(.tail)
                .layoutPriority(1)

            Button(action: onExpandAnswer) {
                Text("See more")
                    .foregroundColor(WhisperFlowStyle.accent)
                    .fixedSize(horizontal: true, vertical: false)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("See more of the island answer")
        }
        .padding(.horizontal, s(10))
        .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
        .frame(
            width: s(model.answerLayout.summaryWidth),
            height: s(kWhisperFlowAnswerSummaryH)
        )
        .background(WhisperFlowStyle.surface)
        .overlay(
            Capsule()
                .stroke(WhisperFlowStyle.outline, lineWidth: s(1))
        )
        .clipShape(Capsule())
        .frame(
            width: s(model.answerLayout.summaryPanelWidth),
            height: s(kWhisperFlowAnswerSummaryPanelH),
            alignment: .top
        )
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(answer.title). See more")
    }

    private var answerExpandedView: some View {
        let answer = model.answer ?? .unavailable

        return ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: s(kWhisperFlowAnswerHeaderSpacing)) {
                HStack(spacing: s(12)) {
                    Text(answer.title)
                        .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                        .foregroundColor(.white)
                        .fixedSize(horizontal: false, vertical: true)
                        .layoutPriority(1)

                    Spacer(minLength: s(12))

                    Button(action: onCollapseAnswer) {
                        Text("See less")
                            .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                            .foregroundColor(WhisperFlowStyle.accent)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("See less of the island answer")
                }

                if !answer.rows.isEmpty {
                    VStack(alignment: .leading, spacing: s(kWhisperFlowAnswerRowSpacing)) {
                        ForEach(Array(answer.rows.enumerated()), id: \.offset) { _, row in
                            VStack(alignment: .leading, spacing: s(kWhisperFlowAnswerLabelValueSpacing)) {
                                Text(row.label)
                                    .font(Brand.swiftUIFont(size: s(11), weight: .semibold))
                                    .foregroundColor(WhisperFlowStyle.accent)

                                Text(row.value)
                                    .font(Brand.swiftUIFont(size: s(14), weight: .regular))
                                    .foregroundColor(.white)
                                    .lineSpacing(s(3))
                                    .fixedSize(horizontal: false, vertical: true)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(height: s(model.answerLayout.contentViewportHeight))
        .padding(.horizontal, s(kWhisperFlowAnswerHorizontalPadding))
        .padding(.vertical, s(kWhisperFlowAnswerVerticalPadding))
        .frame(
            width: s(model.answerLayout.expandedWidth),
            height: s(model.answerLayout.expandedHeight),
            alignment: .top
        )
        .background(WhisperFlowStyle.surface)
        .overlay(
            RoundedRectangle(cornerRadius: s(24), style: .continuous)
                .stroke(WhisperFlowStyle.outline, lineWidth: s(1))
        )
        .clipShape(RoundedRectangle(cornerRadius: s(24), style: .continuous))
        .accessibilityElement(children: .contain)
    }

    private var morphAnimation: Animation {
        .timingCurve(
            0.23,
            1,
            0.32,
            1,
            duration: reduceMotion
                ? kWhisperFlowReducedMotionFadeDuration
                : kWhisperFlowMorphDuration
        )
    }

    private var microScale: CGFloat {
        guard model.microPulseActive, !reduceMotion else { return 1 }
        return microPulseExpanded ? kWhisperFlowMicroPulseScale : 1
    }

    private var microOutlineOpacity: Double {
        guard model.microPulseActive, !reduceMotion else { return 1 }
        return microPulseExpanded ? 1 : kWhisperFlowMicroPulseOutlineMinOpacity
    }

    private func refreshMicroPulse() {
        microPulseExpanded = false
        guard model.microPulseActive, !reduceMotion else { return }

        DispatchQueue.main.async {
            guard model.microPulseActive, !reduceMotion else { return }
            withAnimation(
                .timingCurve(
                    0.77,
                    0,
                    0.175,
                    1,
                    duration: kWhisperFlowMicroPulseDuration / 2
                )
                    .repeatForever(autoreverses: true)
            ) {
                microPulseExpanded = true
            }
        }
    }

    private func stateTransition(scale: CGFloat) -> AnyTransition {
        guard !reduceMotion else { return .opacity }
        return .asymmetric(
            insertion: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: scale),
                identity: IslandMorphModifier(opacity: 1, scale: 1)
            ),
            removal: .opacity
        )
    }
}

private var gOverlayScale: CGFloat = 1.0

@available(macOS 13.0, *)
private final class SessionIslandController: NSObject {
    static let shared = SessionIslandController()

    private var panel: NSPanel?
    private var hostingView: NSHostingView<AnyView>?
    private var trackingView: IslandTrackingView?
    private let islandModel = WhisperFlowIslandModel()
    private var presentation: WhisperFlowPresentation = .micro
    private var visible = false
    private var snapshot = IslandSnapshot()
    private var metrics = OverlayMetrics()
    private var previousFrameCount: Int64?
    private var previousMemoryLifecyclePhase: WhisperFlowMemoryLifecyclePhase?
    private var memoryHasStarted = false
    private var ambientHovered = false
    private var ambientBodyHovered = false
    private var hoverRevealArmed = true
    private var blockedMicroHoverObserved = false
    private var outsideGlobalClickMonitor: Any?
    private var outsideLocalClickMonitor: Any?
    private var continueRequestInFlight = false
    private var latchedAnswer: WhisperFlowAnswerContent?
    private var latchedDecisionId: String?
    private var answerLayout = WhisperFlowAnswerLayout()
    private var memoryTransitionTimer: Timer?
    private var ambientHoverReturnTimer: Timer?
    private var memoryTransitionCountdownActive = false
    private var memoryTransitionCountdownNonce: UInt64 = 0
    private var startingFeedbackNonceCounter: UInt64 = 0
    private var activeStartingFeedbackNonce: UInt64?

    func initializeIfNeeded() {
        if panel == nil {
            createPanel()
            updateContent()
            positionPanel(preserveCurrentAnchor: false, animated: false)
        }
    }

    func update(json: String) {
        if let data = json.data(using: .utf8) {
            do {
                snapshot = try JSONDecoder().decode(IslandSnapshot.self, from: data)
            } catch {
                NSLog("[smalltalk_island] failed to decode island snapshot: \(error)")
                snapshot = IslandSnapshot.continueDecodeError()
            }
        }

        if snapshot.memoryActive ||
            snapshot.state == "starting" ||
            snapshot.state == "processing" {
            memoryHasStarted = true
        }

        if snapshot.state == "hidden" {
            hide()
            return
        }

        if snapshot.state == "trail_reconstructing" {
            continueRequestInFlight = true
            setPresentation(.generating)
        } else if continueRequestInFlight,
                  snapshot.state == "resume_ready" || snapshot.state == "error" {
            finishContinueRequest(with: snapshot)
        }

        let captureControlActive = snapshot.memoryActive || snapshot.state == "starting"
        metrics.meetingActive = captureControlActive

        if let previousFrameCount {
            let delta = max(0, snapshot.frameCount - previousFrameCount)
            metrics.screenActive = snapshot.memoryActive && delta > 0
            metrics.captureFps = snapshot.memoryActive ? Double(delta) : 0
        } else {
            metrics.screenActive = snapshot.memoryActive
            metrics.captureFps = snapshot.memoryActive ? 1 : 0
        }
        previousFrameCount = snapshot.frameCount
        observeMemoryLifecycleTransition()

        initializeIfNeeded()
        updateContent()
        show()
        positionPanel(preserveCurrentAnchor: true, animated: false)
    }

    func show() {
        initializeIfNeeded()
        visible = true
        if !islandModel.isPanelVisible {
            islandModel.isPanelVisible = true
        }
        updateOutsideClickMonitors()
        panel?.orderFrontRegardless()
    }

    func hide() {
        visible = false
        if islandModel.isPanelVisible {
            islandModel.isPanelVisible = false
        }
        presentation = .micro
        previousMemoryLifecyclePhase = nil
        activeStartingFeedbackNonce = nil
        ambientHovered = false
        ambientBodyHovered = false
        hoverRevealArmed = true
        blockedMicroHoverObserved = false
        continueRequestInFlight = false
        latchedAnswer = nil
        latchedDecisionId = nil
        answerLayout = WhisperFlowAnswerLayout()
        cancelPresentationTimers()
        updateOutsideClickMonitors()
        DispatchQueue.main.async { [self] in
            panel?.orderOut(nil)
            updateContent()
        }
    }

    func setExpanded(_ expanded: Bool) {
        if expanded {
            expandAnswer()
        } else {
            showAnswerSummary()
        }
    }

    func reposition() {
        initializeIfNeeded()
        refreshAnswerLayout()
        positionPanel(preserveCurrentAnchor: false, animated: false)
    }

    func shutdown() {
        cancelPresentationTimers()
        removeOutsideClickMonitors()
        islandModel.isPanelVisible = false
        panel?.orderOut(nil)
        panel = nil
        hostingView = nil
        trackingView = nil
        visible = false
        presentation = .micro
        previousMemoryLifecyclePhase = nil
        memoryHasStarted = false
        activeStartingFeedbackNonce = nil
        ambientHovered = false
        ambientBodyHovered = false
        hoverRevealArmed = true
        blockedMicroHoverObserved = false
        continueRequestInFlight = false
        latchedAnswer = nil
        latchedDecisionId = nil
        answerLayout = WhisperFlowAnswerLayout()
    }

    private var memoryActive: Bool {
        snapshot.memoryActive
    }

    private var microPulseActive: Bool {
        captureStatus == .active || captureStatus == .starting
    }

    private var captureStatus: WhisperFlowCaptureStatus {
        if continueRequestInFlight {
            return .generating
        }
        if snapshot.state == "error" || snapshot.lastError != nil {
            return .error
        }
        if !snapshotAllowsCaptureIndication {
            return .suppressed
        }
        if snapshot.state == "starting" {
            return .starting
        }
        if snapshot.state == "processing" {
            return .processing
        }
        return snapshot.memoryActive ? .active : .inactive
    }

    private var memoryLifecyclePhase: WhisperFlowMemoryLifecyclePhase {
        if snapshot.state == "error" || snapshot.lastError != nil {
            return .unavailable
        }
        if snapshot.state == "starting" {
            return .starting
        }
        if snapshot.state == "processing" {
            return .stopping
        }
        return snapshot.memoryActive ? .active : .paused
    }

    private var snapshotAllowsCaptureIndication: Bool {
        guard snapshot.state != "hidden", snapshot.state != "error", !snapshot.isSensitive else {
            return false
        }
        let privacyLabel = snapshot.privacyLabel?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() ?? ""
        return privacyLabel.isEmpty || ["normal", "ok", "allowed"].contains(privacyLabel)
    }

    private func setPresentation(_ nextPresentation: WhisperFlowPresentation) {
        let changed = presentation != nextPresentation
        if nextPresentation != .ambientMemory {
            cancelAmbientHoverReturn()
            ambientHovered = false
            ambientBodyHovered = false
        }
        presentation = nextPresentation

        if changed {
            updateOutsideClickMonitors()
            positionPanel(preserveCurrentAnchor: true, animated: visible)
            updateContent()
        }
    }

    private func revealAmbientMemory() {
        guard !continueRequestInFlight else {
            setPresentation(.generating)
            return
        }
        setPresentation(.ambientMemory)
    }

    private func microHoverChanged(_ hovering: Bool) {
        if !hoverRevealArmed {
            if hovering {
                blockedMicroHoverObserved = true
            } else if blockedMicroHoverObserved {
                hoverRevealArmed = true
                blockedMicroHoverObserved = false
            }
            return
        }
        if !hovering {
            return
        }
        revealAmbientMemory()
    }

    private func requestContinue() {
        guard !continueRequestInFlight else { return }
        cancelPresentationTimers()
        continueRequestInFlight = true
        latchedAnswer = nil
        latchedDecisionId = nil
        answerLayout = WhisperFlowAnswerLayout()
        setPresentation(.generating)
        updateContent()

        guard sendAction("continue") else {
            snapshot = IslandSnapshot.continueDecodeError()
            finishContinueRequest(with: snapshot)
            return
        }
    }

    private func finishContinueRequest(with snapshot: IslandSnapshot) {
        continueRequestInFlight = false
        cancelMemoryTransitionCountdown()
        let answer = WhisperFlowAnswerContent(snapshot: snapshot)
        latchedAnswer = answer
        latchedDecisionId = answer.decisionId
        refreshAnswerLayout()
        setPresentation(.answerSummary)
        updateContent()
    }

    private func refreshAnswerLayout() {
        guard let answer = latchedAnswer else {
            answerLayout = WhisperFlowAnswerLayout()
            return
        }

        let screen = panel?.screen
            ?? screenContaining(NSEvent.mouseLocation)
            ?? NSScreen.main
            ?? NSScreen.screens.first
        let visibleSize = screen?.visibleFrame.size ?? NSSize(width: 1280, height: 800)
        let usableWidth = max(1, visibleSize.width / gOverlayScale)
        let usableHeight = max(1, visibleSize.height / gOverlayScale)
        let summaryFont = NSFont.systemFont(ofSize: 12, weight: .semibold)
        let summaryActionFont = NSFont.systemFont(ofSize: 12, weight: .semibold)
        let summaryMaximum = max(
            kWhisperFlowAnswerSummaryMinW,
            usableWidth - 32 - kWhisperFlowAnswerSummaryPanelMarginW
        )
        let summaryContentWidth = measuredTextWidth(answer.title, font: summaryFont)
            + measuredTextWidth("See more", font: summaryActionFont)
            + 2
            + 20
        let summaryWidth = min(
            max(kWhisperFlowAnswerSummaryMinW, ceil(summaryContentWidth)),
            summaryMaximum
        )

        let titleFont = NSFont.systemFont(ofSize: 12, weight: .semibold)
        let labelFont = NSFont.systemFont(ofSize: 11, weight: .semibold)
        let valueFont = NSFont.systemFont(ofSize: 14, weight: .regular)
        let collapseFont = NSFont.systemFont(ofSize: 12, weight: .semibold)
        let longestTextWidth = ([measuredTextWidth(answer.title, font: titleFont)]
            + answer.rows.flatMap { row in
                [
                    measuredTextWidth(row.label, font: labelFont),
                    measuredTextWidth(row.value, font: valueFont),
                ]
            }).max() ?? 0
        let expandedMaximum = max(
            kWhisperFlowAnswerExpandedMinW,
            min(kWhisperFlowAnswerExpandedMaxW, usableWidth - 32)
        )
        let expandedWidth = min(
            max(
                kWhisperFlowAnswerExpandedMinW,
                ceil(longestTextWidth + kWhisperFlowAnswerHorizontalPadding * 2)
            ),
            expandedMaximum
        )
        let contentWidth = max(1, expandedWidth - kWhisperFlowAnswerHorizontalPadding * 2)
        let collapseWidth = measuredTextWidth("See less", font: collapseFont)
        let titleWidth = max(1, contentWidth - collapseWidth - 18)
        let titleHeight = measuredTextHeight(answer.title, font: titleFont, width: titleWidth)
        let headerHeight = max(20, titleHeight)

        var bodyHeight: CGFloat = 0
        for (index, row) in answer.rows.enumerated() {
            if index > 0 {
                bodyHeight += kWhisperFlowAnswerRowSpacing
            }
            bodyHeight += measuredTextHeight(row.label, font: labelFont, width: contentWidth)
            bodyHeight += kWhisperFlowAnswerLabelValueSpacing
            bodyHeight += measuredTextHeight(
                row.value,
                font: valueFont,
                width: contentWidth,
                lineSpacing: 3
            )
        }

        let bodySpacing = answer.rows.isEmpty ? 0 : kWhisperFlowAnswerHeaderSpacing
        let naturalHeight = max(
            kWhisperFlowAnswerExpandedMinH,
            ceil(
                kWhisperFlowAnswerVerticalPadding * 2
                    + headerHeight
                    + bodySpacing
                    + bodyHeight
            )
        )
        let maximumHeight = max(
            kWhisperFlowAnswerExpandedMinH,
            usableHeight * kWhisperFlowAnswerExpandedMaxScreenFraction
        )
        let expandedHeight = min(naturalHeight, maximumHeight)
        let contentViewportHeight = max(
            0,
            expandedHeight - kWhisperFlowAnswerVerticalPadding * 2
        )

        answerLayout = WhisperFlowAnswerLayout(
            summaryWidth: summaryWidth,
            summaryPanelWidth: summaryWidth + kWhisperFlowAnswerSummaryPanelMarginW,
            expandedWidth: expandedWidth,
            expandedHeight: expandedHeight,
            contentViewportHeight: contentViewportHeight
        )
    }

    private func measuredTextWidth(_ text: String, font: NSFont) -> CGFloat {
        ceil((text as NSString).size(withAttributes: [.font: font]).width)
    }

    private func measuredTextHeight(
        _ text: String,
        font: NSFont,
        width: CGFloat,
        lineSpacing: CGFloat = 0
    ) -> CGFloat {
        let paragraph = NSMutableParagraphStyle()
        paragraph.lineBreakMode = .byWordWrapping
        paragraph.lineSpacing = lineSpacing
        let bounds = (text as NSString).boundingRect(
            with: NSSize(width: width, height: .greatestFiniteMagnitude),
            options: [.usesLineFragmentOrigin, .usesFontLeading],
            attributes: [.font: font, .paragraphStyle: paragraph]
        )
        return max(ceil(font.ascender - font.descender), ceil(bounds.height))
    }

    private func showAnswerSummary() {
        guard latchedAnswer != nil else {
            returnToDefaultPresentation()
            return
        }
        setPresentation(.answerSummary)
    }

    private func expandAnswer() {
        guard latchedAnswer != nil else { return }
        setPresentation(.answerExpanded)
    }

    private func returnToDefaultPresentation() {
        cancelMemoryTransitionCountdown()
        setPresentation(.micro)
    }

    private func ambientBodyHoverChanged(_ hovering: Bool) {
        let next = hovering && presentation == .ambientMemory && captureStatus == .active
        guard ambientBodyHovered != next else { return }
        ambientBodyHovered = next
    }

    private func ambientHoverChanged(_ hovering: Bool) {
        ambientHovered = hovering
        if hovering {
            cancelAmbientHoverReturn()
            return
        }
        guard presentation == .ambientMemory,
              !memoryTransitionCountdownActive else { return }
        scheduleAmbientHoverReturn()
    }

    private func scheduleAmbientHoverReturn() {
        cancelAmbientHoverReturn()
        let timer = Timer(
            timeInterval: kWhisperFlowAmbientHoverReturnDelay,
            repeats: false
        ) { [weak self] _ in
            DispatchQueue.main.async {
                guard let self,
                      self.presentation == .ambientMemory,
                      !self.ambientHovered,
                      !self.memoryTransitionCountdownActive else { return }
                self.ambientHoverReturnTimer = nil
                self.setPresentation(.micro)
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        ambientHoverReturnTimer = timer
    }

    private func cancelAmbientHoverReturn() {
        ambientHoverReturnTimer?.invalidate()
        ambientHoverReturnTimer = nil
    }

    private func observeMemoryLifecycleTransition() {
        let current = memoryLifecyclePhase
        guard let previous = previousMemoryLifecyclePhase else {
            previousMemoryLifecyclePhase = current
            return
        }
        previousMemoryLifecyclePhase = current

        guard !continueRequestInFlight,
              current != .unavailable,
              captureStatus != .error,
              captureStatus != .suppressed else { return }

        let beganStarting = previous == .paused &&
            (current == .starting || current == .active)
        let beganStopping = (previous == .starting || previous == .active) &&
            (current == .stopping || current == .paused)
        guard beganStarting || beganStopping else { return }
        beginMemoryTransitionCountdown(forMemoryStart: beganStarting)
    }

    private func beginMemoryTransitionCountdown(forMemoryStart: Bool) {
        cancelAmbientHoverReturn()
        cancelMemoryTransitionCountdown()
        memoryTransitionCountdownActive = true
        memoryTransitionCountdownNonce &+= 1
        if forMemoryStart {
            startingFeedbackNonceCounter &+= 1
            activeStartingFeedbackNonce = startingFeedbackNonceCounter
        }
        let ambientWasAlreadyVisible = presentation == .ambientMemory
        setPresentation(.ambientMemory)
        updateContent()
        if ambientWasAlreadyVisible {
            positionPanel(preserveCurrentAnchor: true, animated: visible)
        }

        let nonce = memoryTransitionCountdownNonce
        let timer = Timer(
            timeInterval: kWhisperFlowMemoryTransitionDuration,
            repeats: false
        ) { [weak self] _ in
            DispatchQueue.main.async {
                guard let self,
                      self.memoryTransitionCountdownActive,
                      self.memoryTransitionCountdownNonce == nonce else { return }
                self.memoryTransitionTimer = nil
                self.memoryTransitionCountdownActive = false
                self.activeStartingFeedbackNonce = nil
                if self.ambientHovered {
                    self.hoverRevealArmed = false
                    self.blockedMicroHoverObserved = false
                }
                self.setPresentation(.micro)
                self.updateContent()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        memoryTransitionTimer = timer
    }

    private func cancelMemoryTransitionCountdown() {
        memoryTransitionTimer?.invalidate()
        memoryTransitionTimer = nil
        memoryTransitionCountdownActive = false
        activeStartingFeedbackNonce = nil
        if islandModel.memoryTransitionCountdownActive {
            islandModel.memoryTransitionCountdownActive = false
        }
        if islandModel.startingFeedbackNonce != nil {
            islandModel.startingFeedbackNonce = nil
        }
    }

    private func markPanelDragBegan() {}

    private func cancelPresentationTimers() {
        cancelAmbientHoverReturn()
        cancelMemoryTransitionCountdown()
    }

    private var targetPanelSize: NSSize {
        switch presentation {
        case .micro:
            return NSSize(
                width: kWhisperFlowCapturePanelW * gOverlayScale,
                height: kWhisperFlowCapturePanelH * gOverlayScale
            )
        case .ambientMemory, .generating:
            return NSSize(
                width: (
                    memoryTransitionCountdownActive || continueRequestInFlight
                        ? kWhisperFlowNotificationPanelW
                        : kWhisperFlowCapturePanelW
                ) * gOverlayScale,
                height: (
                    memoryTransitionCountdownActive || continueRequestInFlight
                        ? kWhisperFlowNotificationPanelH
                        : kWhisperFlowCapturePanelH
                ) * gOverlayScale
            )
        case .answerSummary:
            return NSSize(
                width: answerLayout.summaryPanelWidth * gOverlayScale,
                height: kWhisperFlowAnswerSummaryPanelH * gOverlayScale
            )
        case .answerExpanded:
            return NSSize(
                width: answerLayout.expandedWidth * gOverlayScale,
                height: answerLayout.expandedHeight * gOverlayScale
            )
        }
    }

    private func createPanel() {
        let size = targetPanelSize
        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: Int(size.width), height: Int(size.height)),
            styleMask: [.nonactivatingPanel, .borderless],
            backing: .buffered,
            defer: false
        )
        panel.isFloatingPanel = true
        panel.level = NSWindow.Level(rawValue: Int(CGWindowLevelForKey(.floatingWindow)) + 2)
        panel.collectionBehavior = [.canJoinAllSpaces, .ignoresCycle, .fullScreenAuxiliary]
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.hidesOnDeactivate = false
        panel.isMovableByWindowBackground = true
        panel.acceptsMouseMovedEvents = true
        panel.isReleasedWhenClosed = false
        panel.sharingType = .readOnly

        let tracking = IslandTrackingView(frame: NSRect(x: 0, y: 0, width: Int(size.width), height: Int(size.height)))
        tracking.configureTransparentLayer()
        tracking.autoresizingMask = [.width, .height]
        panel.contentView = tracking
        trackingView = tracking
        self.panel = panel
    }

    private func positionPanel(preserveCurrentAnchor: Bool, animated: Bool) {
        guard panel != nil else { return }
        setPanelFrame(
            resolvedPanelFrame(preserveCurrentAnchor: preserveCurrentAnchor),
            animated: animated
        )
    }

    private func resolvedPanelFrame(preserveCurrentAnchor: Bool) -> NSRect {
        let size = targetPanelSize
        let currentFrame = panel?.frame ?? .zero
        let anchor = preserveCurrentAnchor && currentFrame.width > 0 && currentFrame.height > 0
            ? NSPoint(x: currentFrame.midX, y: currentFrame.maxY)
            : initialTopCenterAnchor(for: size)
        let screen = screenContaining(anchor)
            ?? screenContaining(NSEvent.mouseLocation)
            ?? NSScreen.main
            ?? NSScreen.screens.first
        guard let screen else {
            return NSRect(x: anchor.x - size.width / 2, y: anchor.y - size.height, width: size.width, height: size.height)
        }

        let visibleFrame = screen.visibleFrame
        let proposed = NSRect(
            x: anchor.x - size.width / 2,
            y: anchor.y - size.height,
            width: size.width,
            height: size.height
        )
        return clampFrame(proposed, to: visibleFrame)
    }

    private func initialTopCenterAnchor(for size: NSSize) -> NSPoint {
        let mouseLocation = NSEvent.mouseLocation
        let screen = screenContaining(mouseLocation)
            ?? NSScreen.main
            ?? NSScreen.screens.first
        guard let screen else {
            return NSPoint(x: size.width / 2, y: size.height)
        }

        let visibleFrame = screen.visibleFrame
        return NSPoint(
            x: screen.frame.origin.x + screen.frame.width / 2,
            y: visibleFrame.origin.y + visibleFrame.height - 4
        )
    }

    private func screenContaining(_ point: NSPoint) -> NSScreen? {
        NSScreen.screens.first(where: { NSMouseInRect(point, $0.frame, false) })
    }

    private func clampFrame(_ frame: NSRect, to visibleFrame: NSRect) -> NSRect {
        var next = frame
        next.origin.x = min(
            max(next.origin.x, visibleFrame.minX),
            visibleFrame.maxX - next.width
        )
        next.origin.y = min(
            max(next.origin.y, visibleFrame.minY),
            visibleFrame.maxY - next.height
        )
        return next
    }

    private func setPanelFrame(_ frame: NSRect, animated: Bool) {
        guard let panel else { return }
        if animated && !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion {
            NSAnimationContext.runAnimationGroup { context in
                context.duration = kWhisperFlowMorphDuration
                context.timingFunction = IslandMotion.panelTimingFunction()
                panel.animator().setFrame(frame, display: visible)
            }
        } else {
            panel.setFrame(frame, display: visible)
        }
    }

    private func updateContent() {
        guard let panel, let contentView = panel.contentView else { return }
        if islandModel.presentation != presentation {
            islandModel.presentation = presentation
        }
        if islandModel.memoryActive != memoryActive {
            islandModel.memoryActive = memoryActive
        }
        if islandModel.memoryHasStarted != memoryHasStarted {
            islandModel.memoryHasStarted = memoryHasStarted
        }
        if islandModel.microPulseActive != microPulseActive {
            islandModel.microPulseActive = microPulseActive
        }
        let resolvedCaptureStatus = captureStatus
        if resolvedCaptureStatus != .active {
            ambientBodyHovered = false
        }
        if islandModel.captureStatus != resolvedCaptureStatus {
            islandModel.captureStatus = resolvedCaptureStatus
        }
        if islandModel.startingFeedbackNonce != activeStartingFeedbackNonce {
            islandModel.startingFeedbackNonce = activeStartingFeedbackNonce
        }
        if islandModel.capturePulseNonce != snapshot.capturePulseNonce {
            islandModel.capturePulseNonce = snapshot.capturePulseNonce
        }
        if islandModel.memoryTransitionCountdownActive != memoryTransitionCountdownActive {
            islandModel.memoryTransitionCountdownActive = memoryTransitionCountdownActive
        }
        if islandModel.memoryTransitionCountdownNonce != memoryTransitionCountdownNonce {
            islandModel.memoryTransitionCountdownNonce = memoryTransitionCountdownNonce
        }
        if islandModel.continueGenerating != continueRequestInFlight {
            islandModel.continueGenerating = continueRequestInFlight
        }
        if islandModel.answer != latchedAnswer {
            islandModel.answer = latchedAnswer
        }
        if islandModel.answerLayout != answerLayout {
            islandModel.answerLayout = answerLayout
        }

        guard hostingView == nil else { return }

        let view = WhisperFlowIslandView(
            scale: gOverlayScale,
            model: islandModel,
            onRevealAmbientMemory: { [weak self] in
                self?.revealAmbientMemory()
            },
            onMicroHover: { [weak self] hovering in
                self?.microHoverChanged(hovering)
            },
            onReadyAction: { [weak self] in
                self?.requestContinue()
            },
            onStartMemory: { [weak self] in
                self?.handle(action: "start_memory")
            },
            onAmbientHover: { [weak self] hovering in
                self?.ambientHoverChanged(hovering)
            },
            onAmbientBodyHover: { [weak self] hovering in
                self?.ambientBodyHoverChanged(hovering)
            },
            onExpandAnswer: { [weak self] in
                self?.expandAnswer()
            },
            onCollapseAnswer: { [weak self] in
                self?.showAnswerSummary()
            }
        )

        let hosting = DraggableHostingView(rootView: AnyView(view))
        hosting.onDragBegan = { [weak self] in
            self?.markPanelDragBegan()
        }
        hosting.configureTransparentLayer()
        hosting.frame = contentView.bounds
        hosting.autoresizingMask = [.width, .height]
        contentView.addSubview(hosting)
        hostingView = hosting
    }

    private func handle(action: String) {
        switch action {
        case "open_timeline":
            sendAction("open_main_window")
        case "open_chat":
            sendAction("resume_me")
        case "open_search":
            sendAction("open_main_window")
        case "continue":
            requestContinue()
        case "start_memory":
            setPresentation(.ambientMemory)
            sendAction("start_capture")
        case "pause_memory":
            setPresentation(.ambientMemory)
            sendAction("stop_capture")
        case "reconstruct_trail":
            requestContinue()
        case "show_trail":
            sendAction("show_trail")
        case "open_resume_point":
            sendAction("open_resume_point", decisionId: snapshot.continueDecisionId)
        case "open_main_window":
            sendAction("open_main_window")
        case "open_expanded":
            setPresentation(.answerExpanded)
        case "toggle_meeting":
            setPresentation(.ambientMemory)
            sendAction(metrics.meetingActive ? "stop_capture" : "start_capture")
        case "capture_once":
            setPresentation(.ambientMemory)
            sendAction("capture_once")
        case "reveal_compact", "keep_compact":
            revealAmbientMemory()
        case "close":
            setExpanded(false)
            sendAction("collapse")
        default:
            break
        }
    }

    private func handle(continueAction action: IslandAvailableAction) {
        switch action.kind {
        case .openContinueTarget:
            setPresentation(.answerExpanded)
        case .refreshContinue, .markWrongTarget, .markNotUseful, .startLocalMemory, .captureEvidenceNow:
            setPresentation(.answerSummary)
        case .chooseTaskAlternative,
             .rejectSelectedTask,
             .rejectTaskAlternative,
             .markSupportingWork,
             .markUnrelatedActivity,
             .markTaskCompleted,
             .reactivateTask:
            setPresentation(.answerSummary)
        case .inspectEvidence, .openSmalltalk:
            break
        case .unknown:
            sendAction("open_main_window")
            return
        }
        sendAction(
            "perform_continue_action",
            decisionId: action.decisionId ?? snapshot.islandContinueState?.decisionId ?? snapshot.continueDecisionId,
            continueActionKind: action.kind.rawValue,
            taskSnapshotId: action.taskSnapshotId,
            taskSnapshotRevision: action.taskSnapshotRevision,
            affectedTaskField: action.affectedTaskField,
            taskHypothesisId: action.taskHypothesisId
        )
    }

    private func updateOutsideClickMonitors() {
        if visible && (presentation == .answerSummary || presentation == .answerExpanded) {
            installOutsideClickMonitors()
        } else {
            removeOutsideClickMonitors()
        }
    }

    private func installOutsideClickMonitors() {
        guard outsideGlobalClickMonitor == nil, outsideLocalClickMonitor == nil else { return }

        outsideGlobalClickMonitor = NSEvent.addGlobalMonitorForEvents(
            matching: [.leftMouseDown, .rightMouseDown, .otherMouseDown]
        ) { [weak self] event in
            self?.collapseIfOutside(event)
        }

        outsideLocalClickMonitor = NSEvent.addLocalMonitorForEvents(
            matching: [.leftMouseDown, .rightMouseDown, .otherMouseDown]
        ) { [weak self] event in
            self?.collapseIfOutside(event)
            return event
        }
    }

    private func removeOutsideClickMonitors() {
        if let outsideGlobalClickMonitor {
            NSEvent.removeMonitor(outsideGlobalClickMonitor)
            self.outsideGlobalClickMonitor = nil
        }
        if let outsideLocalClickMonitor {
            NSEvent.removeMonitor(outsideLocalClickMonitor)
            self.outsideLocalClickMonitor = nil
        }
    }

    private func collapseIfOutside(_ event: NSEvent) {
        guard presentation == .answerSummary || presentation == .answerExpanded, let panel else { return }
        if event.window === panel {
            return
        }
        if NSMouseInRect(NSEvent.mouseLocation, panel.frame, false) {
            return
        }

        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            if self.presentation == .answerExpanded {
                self.showAnswerSummary()
            } else if self.presentation == .answerSummary {
                self.returnToDefaultPresentation()
            }
        }
    }

    @discardableResult
    private func sendAction(
        _ action: String,
        decisionId: String? = nil,
        continueActionKind: String? = nil,
        taskSnapshotId: String? = nil,
        taskSnapshotRevision: Int64? = nil,
        affectedTaskField: String? = nil,
        taskHypothesisId: String? = nil
    ) -> Bool {
        guard let callback = gActionCallback else { return false }
        var fields = ["\"action\":\"\(jsonEscaped(action))\""]
        if let decisionId, !decisionId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            fields.append("\"decision_id\":\"\(jsonEscaped(decisionId))\"")
        }
        if let continueActionKind, !continueActionKind.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            fields.append("\"action_kind\":\"\(jsonEscaped(continueActionKind))\"")
        }
        if let taskSnapshotId, !taskSnapshotId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            fields.append("\"task_snapshot_id\":\"\(jsonEscaped(taskSnapshotId))\"")
        }
        if let taskSnapshotRevision {
            fields.append("\"task_snapshot_revision\":\(taskSnapshotRevision)")
        }
        if let affectedTaskField, !affectedTaskField.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            fields.append("\"affected_task_field\":\"\(jsonEscaped(affectedTaskField))\"")
        }
        if let taskHypothesisId, !taskHypothesisId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            fields.append("\"task_hypothesis_id\":\"\(jsonEscaped(taskHypothesisId))\"")
        }
        fields.append("\"source\":\"native_island\"")
        let json = "{\(fields.joined(separator: ","))}"
        json.withCString { pointer in
            callback(pointer)
        }
        return true
    }

    private func jsonEscaped(_ value: String) -> String {
        var escaped = ""
        for scalar in value.unicodeScalars {
            switch scalar {
            case "\"":
                escaped += "\\\""
            case "\\":
                escaped += "\\\\"
            case "\n":
                escaped += "\\n"
            case "\r":
                escaped += "\\r"
            case "\t":
                escaped += "\\t"
            default:
                escaped.unicodeScalars.append(scalar)
            }
        }
        return escaped
    }
}

@available(macOS 13.0, *)
private final class IslandTrackingView: NSView {
    override var isOpaque: Bool { false }

    func configureTransparentLayer() {
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
        layer?.isOpaque = false
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        configureTransparentLayer()
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        true
    }
}

@available(macOS 13.0, *)
private final class DraggableHostingView<Content: View>: NSHostingView<Content> {
    var onDragBegan: (() -> Void)?
    private var dragMonitor: Any?
    private var dragStartLocation = NSPoint.zero

    override var isOpaque: Bool { false }

    deinit {
        if let dragMonitor {
            NSEvent.removeMonitor(dragMonitor)
        }
    }

    func configureTransparentLayer() {
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
        layer?.isOpaque = false
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        configureTransparentLayer()
    }

    override func mouseDown(with event: NSEvent) {
        super.mouseDown(with: event)
        guard let window else { return }

        if let dragMonitor {
            NSEvent.removeMonitor(dragMonitor)
            self.dragMonitor = nil
        }

        dragStartLocation = event.locationInWindow
        let dragThreshold: CGFloat = 4
        dragMonitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDragged, .leftMouseUp]) { [weak self] event in
            guard let self else { return event }
            switch event.type {
            case .leftMouseUp:
                if let dragMonitor = self.dragMonitor {
                    NSEvent.removeMonitor(dragMonitor)
                    self.dragMonitor = nil
                }
                return event
            case .leftMouseDragged:
                let dx = event.locationInWindow.x - self.dragStartLocation.x
                let dy = event.locationInWindow.y - self.dragStartLocation.y
                if hypot(dx, dy) > dragThreshold {
                    if let dragMonitor = self.dragMonitor {
                        NSEvent.removeMonitor(dragMonitor)
                        self.dragMonitor = nil
                    }
                    self.onDragBegan?()
                    window.performDrag(with: event)
                    return nil
                }
                return event
            default:
                return event
            }
        }
    }
}

@_cdecl("smalltalk_island_init")
public func smalltalkIslandInit() {
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.initializeIfNeeded()
        }
    }
}

@_cdecl("smalltalk_island_set_action_callback")
public func smalltalkIslandSetActionCallback(_ callback: @escaping SmalltalkIslandActionCallback) {
    gActionCallback = callback
}

@_cdecl("smalltalk_island_update_json")
public func smalltalkIslandUpdateJson(_ jsonPtr: UnsafePointer<CChar>?) {
    let json = jsonPtr.map { String(cString: $0) } ?? "{}"
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.update(json: json)
        }
    }
}

@_cdecl("smalltalk_island_show")
public func smalltalkIslandShow() {
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.show()
        }
    }
}

@_cdecl("smalltalk_island_hide")
public func smalltalkIslandHide() {
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.hide()
        }
    }
}

@_cdecl("smalltalk_island_set_expanded")
public func smalltalkIslandSetExpanded(_ expanded: Bool) {
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.setExpanded(expanded)
        }
    }
}

@_cdecl("smalltalk_island_reposition")
public func smalltalkIslandReposition() {
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.reposition()
        }
    }
}

@_cdecl("smalltalk_island_shutdown")
public func smalltalkIslandShutdown() {
    DispatchQueue.main.async {
        if #available(macOS 13.0, *) {
            SessionIslandController.shared.shutdown()
        }
    }
}
