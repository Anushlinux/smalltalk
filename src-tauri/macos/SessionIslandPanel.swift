import AppKit
import Combine
import Foundation
import QuartzCore
import SwiftUI

public typealias SmalltalkIslandActionCallback = @convention(c) (UnsafePointer<CChar>) -> Void
private var gActionCallback: SmalltalkIslandActionCallback?

private struct IslandSnapshot: Decodable {
    var state: String = "hidden"
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

private let kWhisperFlowReadyPanelW: CGFloat = 187
private let kWhisperFlowReadyPanelH: CGFloat = 49
private let kWhisperFlowReadyW: CGFloat = 152
private let kWhisperFlowReadyH: CGFloat = 30
private let kWhisperFlowReadyActionW: CGFloat = 28
private let kWhisperFlowReadyActionH: CGFloat = 24
private let kWhisperFlowRecordingContentW: CGFloat = 107
private let kWhisperFlowEvidenceTapeW: CGFloat = 92
private let kWhisperFlowEvidenceTapeH: CGFloat = 18
private let kWhisperFlowCapturePreviewEnabled = true
private let kWhisperFlowRecorderCycleInterval: TimeInterval = 3.0
private let kWhisperFlowAnswerSummaryPanelW: CGFloat = 187
private let kWhisperFlowAnswerSummaryPanelH: CGFloat = 49
private let kWhisperFlowAnswerSummaryW: CGFloat = 152
private let kWhisperFlowAnswerSummaryH: CGFloat = 30
private let kWhisperFlowAnswerExpandedW: CGFloat = 520
private let kWhisperFlowAnswerExpandedH: CGFloat = 152
private let kWhisperFlowAnswerRevealDelay: TimeInterval = 0.35
private let kWhisperFlowAnswerReturnDelay: TimeInterval = 8.0
private let kWhisperFlowMicroPulseDuration: TimeInterval = 3.2
private let kWhisperFlowMicroPulseScale: CGFloat = 1.018
private let kWhisperFlowMicroPulseOutlineMinOpacity = 0.72
private let kWhisperFlowMorphDuration: TimeInterval = 0.18

private enum WhisperFlowPresentation: Equatable {
    case micro
    case recordingTape
    case answerSummary
    case answerExpanded
}

private enum WhisperFlowCaptureStatus: Equatable {
    case recording
    case starting
    case processing
    case inactive
    case suppressed
    case error
}

private enum WhisperFlowPreview {
    static let title = "Island ready."
    static let answer = "Continue refining the native floating island. Match the micro hover control, compact answer pill, and expanded answer layout to the WhisperFlow references before reconnecting memory or Continue behavior."
    static let surface = Color(red: 0, green: 0, blue: 0)
    static let outline = Color(red: 48 / 255, green: 48 / 255, blue: 47 / 255)
    static let accent = Color(red: 245 / 255, green: 191 / 255, blue: 239 / 255)
}

@available(macOS 13.0, *)
private final class WhisperFlowIslandModel: ObservableObject {
    @Published var presentation: WhisperFlowPresentation = .recordingTape
    @Published var memoryActive = false
    @Published var microPulseActive = false
    @Published var captureFrameCount: Int64 = 0
    @Published var capturePulseNonce: UInt64?
    @Published var captureIndicationActive = kWhisperFlowCapturePreviewEnabled
    @Published var captureStatus: WhisperFlowCaptureStatus = kWhisperFlowCapturePreviewEnabled
        ? .recording
        : .inactive
}

@available(macOS 13.0, *)
private struct WhisperFlowIslandView: View {
    let scale: CGFloat
    @ObservedObject var model: WhisperFlowIslandModel
    let onRevealRecorder: () -> Void
    let onReadyAction: () -> Void
    let onAnswerSummaryHover: (Bool) -> Void
    let onExpandAnswer: () -> Void
    let onCollapseAnswer: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var microPulseExpanded = false
    @State private var recordingActionHovered = false
    @State private var recorderCycleTextVisible = false
    @State private var lastTapePulseNonce: UInt64?
    @State private var tapePulseActive = false
    @State private var tapePulseGeneration = 0
    private let recorderCycleTimer = Timer.publish(
        every: kWhisperFlowRecorderCycleInterval,
        on: .main,
        in: .common
    ).autoconnect()

    private func s(_ value: CGFloat) -> CGFloat { value * scale }

    var body: some View {
        ZStack(alignment: .top) {
            switch model.presentation {
            case .micro:
                microView
                    .transition(stateTransition(scale: 0.96))
            case .recordingTape:
                recordingTapeView
                    .transition(stateTransition(scale: 0.96))
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

    private var microView: some View {
        Button(action: onRevealRecorder) {
            Capsule()
                .fill(WhisperFlowPreview.surface)
                .overlay(
                    Capsule()
                        .stroke(
                            WhisperFlowPreview.outline.opacity(microOutlineOpacity),
                            lineWidth: s(1)
                        )
                )
                .frame(width: s(kBaseMicroVisualW), height: s(kBaseMicroVisualH))
                .scaleEffect(microScale)
                .frame(width: s(kBaseMicroHitW), height: s(kBaseMicroHitH))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(model.memoryActive ? "Smalltalk memory is active" : "Show Smalltalk")
        .onHover { hovering in
            if hovering {
                onRevealRecorder()
            }
        }
        .frame(
            width: s(kWhisperFlowReadyPanelW),
            height: s(kWhisperFlowReadyPanelH),
            alignment: .center
        )
        .id("micro-\(model.microPulseActive)-\(reduceMotion)")
        .onAppear(perform: refreshMicroPulse)
        .onDisappear {
            microPulseExpanded = false
        }
    }

    private var recordingTapeView: some View {
        HStack(spacing: s(5)) {
            ZStack(alignment: .leading) {
                tapeTransportView
                    .opacity(recorderTextVisible ? 0 : 1)

                Text("What was I doing?")
                    .font(Brand.swiftUIFont(size: s(10.5), weight: .semibold))
                    .foregroundColor(.white)
                    .lineLimit(1)
                    .fixedSize()
                    .opacity(recorderTextVisible ? 1 : 0)
            }
            .frame(
                width: s(kWhisperFlowRecordingContentW),
                height: s(kWhisperFlowReadyActionH),
                alignment: .leading
            )
            .allowsHitTesting(false)
            .animation(recordingContentAnimation, value: recorderTextVisible)

            Button(action: onReadyAction) {
                Image(systemName: "arrow.right")
                    .font(.system(size: s(11), weight: .semibold))
                    .foregroundColor(.white)
                    .frame(
                        width: s(kWhisperFlowReadyActionW),
                        height: s(kWhisperFlowReadyActionH)
                    )
                    .background(
                        Capsule()
                            .fill(
                                recordingActionHovered
                                    ? Color(red: 58 / 255, green: 58 / 255, blue: 56 / 255)
                                    : WhisperFlowPreview.outline
                            )
                    )
                    .contentShape(Capsule())
            }
            .buttonStyle(WhisperFlowPressButtonStyle(reduceMotion: reduceMotion))
            .accessibilityLabel("Show what I was doing")
            .onHover { hovering in
                withAnimation(recordingContentAnimation) {
                    recordingActionHovered = hovering
                }
            }
        }
        .padding(.leading, s(8))
        .padding(.trailing, s(4))
        .frame(width: s(kWhisperFlowReadyW), height: s(kWhisperFlowReadyH))
        .background(WhisperFlowPreview.surface)
        .overlay(
            Capsule()
                .stroke(WhisperFlowPreview.outline, lineWidth: s(1))
        )
        .clipShape(Capsule())
        .frame(
            width: s(kWhisperFlowReadyPanelW),
            height: s(kWhisperFlowReadyPanelH),
            alignment: .center
        )
        .contentShape(Rectangle())
        .accessibilityElement(children: .contain)
        .onAppear {
            lastTapePulseNonce = model.capturePulseNonce
            recorderCycleTextVisible = false
        }
        .onDisappear {
            recordingActionHovered = false
            recorderCycleTextVisible = false
        }
        .onChange(of: model.capturePulseNonce) { next in
            handleTapePulse(next)
        }
        .onReceive(recorderCycleTimer) { _ in
            guard !recordingActionHovered else { return }
            withAnimation(recordingContentAnimation) {
                recorderCycleTextVisible.toggle()
            }
        }
    }

    private var recorderTextVisible: Bool {
        recordingActionHovered || recorderCycleTextVisible
    }

    private var tapeTransportView: some View {
        evidenceTapeGlyph
        .frame(
            width: s(kWhisperFlowRecordingContentW),
            height: s(kWhisperFlowReadyActionH),
            alignment: .center
        )
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(captureStatusText)
    }

    private var evidenceTapeGlyph: some View {
        TimelineView(
            .animation(
                minimumInterval: 1.0 / 24.0,
                paused: !model.captureIndicationActive || reduceMotion || recordingActionHovered
            )
        ) { timeline in
            Canvas { context, size in
                drawEvidenceTape(
                    context: &context,
                    size: size,
                    scanPhase: tapeScanPhase(at: timeline.date)
                )
            }
            .frame(
                width: s(kWhisperFlowEvidenceTapeW),
                height: s(kWhisperFlowEvidenceTapeH)
            )
        }
    }

    private func drawEvidenceTape(
        context: inout GraphicsContext,
        size: CGSize,
        scanPhase: Double
    ) {
        let lineWidth = max(1, s(1))
        let railOpacity = model.captureIndicationActive ? 0.22 : 0.12
        let railInset = max(1, s(1))

        for railFraction in [0.28, 0.72] {
            let y = round(size.height * railFraction)
            context.fill(
                Path(
                    CGRect(
                        x: railInset,
                        y: y,
                        width: max(0, size.width - railInset * 2),
                        height: lineWidth
                    )
                ),
                with: .color(Color.white.opacity(railOpacity))
            )
        }

        drawFrameTicks(context: &context, size: size)

        let scanX = round(max(railInset, min(size.width - railInset, scanPhase * size.width)))
        context.fill(
            Path(
                CGRect(
                    x: scanX,
                    y: railInset,
                    width: lineWidth,
                    height: max(lineWidth, size.height - railInset * 2)
                )
            ),
            with: .color(
                Color.white.opacity(model.captureIndicationActive ? 0.78 : 0.28)
            )
        )
    }

    private func drawFrameTicks(context: inout GraphicsContext, size: CGSize) {
        let previewFrameCount: Int64 = kWhisperFlowCapturePreviewEnabled ? 9 : 0
        let count = max(previewFrameCount, model.captureFrameCount)
        let tickCount = Int(min(count, 9))
        guard tickCount > 0 else { return }

        let first = count - Int64(tickCount) + 1
        for offset in 0..<tickCount {
            let frameOrdinal = first + Int64(offset)
            let fraction = tickFraction(frameOrdinal)
            let tickHeight = size.height * (0.30 + 0.08 * Double((frameOrdinal % 3 + 3) % 3))
            let x = round(max(1, min(size.width - 2, fraction * size.width)))
            let y = round((size.height - tickHeight) / 2)
            let isNewest = offset == tickCount - 1
            let opacity: Double
            if isNewest {
                opacity = model.captureIndicationActive
                    ? tapePulseActive ? 0.98 : 0.68
                    : 0.42
            } else {
                opacity = model.captureIndicationActive ? 0.32 + 0.06 * Double(offset % 3) : 0.20
            }

            context.fill(
                Path(
                    CGRect(
                        x: x,
                        y: y,
                        width: max(1, s(1)),
                        height: tickHeight
                    )
                ),
                with: .color(Color.white.opacity(opacity))
            )
        }
    }

    private func tapeScanPhase(at date: Date) -> Double {
        guard model.captureIndicationActive else { return 0.38 }
        guard !reduceMotion else { return 0.62 }

        return (date.timeIntervalSinceReferenceDate / 5.5)
            .truncatingRemainder(dividingBy: 1)
    }

    private func tickFraction(_ value: Int64) -> Double {
        let raw = fmod(
            Double(value) * 0.618_033_988_75 +
                Double((value % 7 + 7) % 7) * 0.031,
            1
        )
        return 0.08 + raw * 0.84
    }

    private var captureStatusText: String {
        switch model.captureStatus {
        case .recording:
            return "Capturing screens"
        case .starting:
            return "Starting capture"
        case .processing:
            return "Saving capture"
        case .inactive:
            return "Capture paused"
        case .suppressed:
            return "Not capturing here"
        case .error:
            return "Capture issue"
        }
    }

    private var recordingContentAnimation: Animation {
        .timingCurve(0.23, 1, 0.32, 1, duration: reduceMotion ? 0.12 : 0.14)
    }

    private func handleTapePulse(_ next: UInt64?) {
        guard next != nil, next != lastTapePulseNonce else {
            lastTapePulseNonce = next
            return
        }

        lastTapePulseNonce = next
        guard model.captureIndicationActive else { return }
        tapePulseGeneration += 1
        let generation = tapePulseGeneration

        withAnimation(reduceMotion ? .linear(duration: 0.08) : IslandMotion.quick(false)) {
            tapePulseActive = true
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + 0.46) {
            guard generation == tapePulseGeneration else { return }
            withAnimation(reduceMotion ? .linear(duration: 0.12) : IslandMotion.quick(false)) {
                tapePulseActive = false
            }
        }
    }

    private var answerSummaryView: some View {
        HStack(spacing: s(2)) {
            Text(WhisperFlowPreview.title)
                .foregroundColor(.white)

            Button(action: onExpandAnswer) {
                Text("See more")
                    .foregroundColor(WhisperFlowPreview.accent)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("See more of the island answer")
        }
        .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
        .lineLimit(1)
        .fixedSize()
        .frame(width: s(kWhisperFlowAnswerSummaryW), height: s(kWhisperFlowAnswerSummaryH))
        .background(WhisperFlowPreview.surface)
        .overlay(
            Capsule()
                .stroke(WhisperFlowPreview.outline, lineWidth: s(1))
        )
        .clipShape(Capsule())
        .frame(
            width: s(kWhisperFlowAnswerSummaryPanelW),
            height: s(kWhisperFlowAnswerSummaryPanelH),
            alignment: .center
        )
        .contentShape(Rectangle())
        .onHover(perform: onAnswerSummaryHover)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Island ready. See more")
    }

    private var answerExpandedView: some View {
        VStack(alignment: .leading, spacing: s(18)) {
            HStack(spacing: s(12)) {
                Text(WhisperFlowPreview.title)
                    .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                    .foregroundColor(.white)

                Spacer(minLength: s(12))

                Button(action: onCollapseAnswer) {
                    Text("See less")
                        .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                        .foregroundColor(WhisperFlowPreview.accent)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("See less of the island answer")
            }

            Text(WhisperFlowPreview.answer)
                .font(Brand.swiftUIFont(size: s(14), weight: .regular))
                .foregroundColor(.white)
                .lineSpacing(s(3))
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)

            Spacer(minLength: 0)
        }
        .padding(.horizontal, s(24))
        .padding(.vertical, s(20))
        .frame(width: s(kWhisperFlowAnswerExpandedW), height: s(kWhisperFlowAnswerExpandedH))
        .background(WhisperFlowPreview.surface)
        .overlay(
            RoundedRectangle(cornerRadius: s(24), style: .continuous)
                .stroke(WhisperFlowPreview.outline, lineWidth: s(1))
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
            duration: reduceMotion ? 0.01 : kWhisperFlowMorphDuration
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
    private var presentation: WhisperFlowPresentation = .recordingTape
    private var visible = false
    private var snapshot = IslandSnapshot()
    private var metrics = OverlayMetrics()
    private var previousFrameCount: Int64?
    private var previewMemoryActiveOverride = false
    private var outsideGlobalClickMonitor: Any?
    private var outsideLocalClickMonitor: Any?
    private var answerRevealTimer: Timer?
    private var answerReturnTimer: Timer?
    private var answerReturnStartedAt: Date?
    private var answerReturnRemaining = kWhisperFlowAnswerReturnDelay

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

        if snapshot.state == "hidden" {
            hide()
            return
        }

        let recording = isRecording(snapshot.state)
        let captureControlActive = recording || snapshot.state == "starting" || snapshot.state == "processing"
        metrics.meetingActive = captureControlActive

        if let previousFrameCount {
            let delta = max(0, snapshot.frameCount - previousFrameCount)
            metrics.screenActive = recording && delta > 0
            metrics.captureFps = recording ? Double(delta) : 0
        } else {
            metrics.screenActive = recording
            metrics.captureFps = recording ? 1 : 0
        }
        previousFrameCount = snapshot.frameCount

        if presentation != .answerSummary,
           presentation != .answerExpanded,
           answerRevealTimer == nil {
            setPresentation(.recordingTape)
        }

        initializeIfNeeded()
        updateContent()
        show()
        positionPanel(preserveCurrentAnchor: true, animated: false)
    }

    func show() {
        initializeIfNeeded()
        visible = true
        updateOutsideClickMonitors()
        panel?.orderFrontRegardless()
    }

    func hide() {
        visible = false
        presentation = .recordingTape
        previewMemoryActiveOverride = false
        cancelPreviewTimers()
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
        positionPanel(preserveCurrentAnchor: false, animated: false)
    }

    func shutdown() {
        cancelPreviewTimers()
        removeOutsideClickMonitors()
        panel?.orderOut(nil)
        panel = nil
        hostingView = nil
        trackingView = nil
        visible = false
        presentation = .recordingTape
        previewMemoryActiveOverride = false
    }

    private func isRecording(_ state: String) -> Bool {
        state == "recording_compact" || state == "recording_expanded"
    }

    private var memoryActive: Bool {
        previewMemoryActiveOverride ||
            isRecording(snapshot.state) ||
            snapshot.state == "starting" ||
            snapshot.state == "processing"
    }

    private var microPulseActive: Bool {
        captureIndicationActive
    }

    private var captureIndicationActive: Bool {
        (
            isRecording(snapshot.state) ||
                snapshot.state == "starting" ||
                snapshot.state == "processing"
        ) && snapshotAllowsCaptureIndication
    }

    private var captureStatus: WhisperFlowCaptureStatus {
        if snapshot.state == "error" || snapshot.lastError != nil {
            return .error
        }
        if !snapshotAllowsCaptureIndication {
            return .suppressed
        }
        switch snapshot.state {
        case "recording_compact", "recording_expanded":
            return .recording
        case "starting":
            return .starting
        case "processing":
            return .processing
        default:
            return .inactive
        }
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
        presentation = nextPresentation

        if changed {
            updateOutsideClickMonitors()
            positionPanel(preserveCurrentAnchor: true, animated: visible)
            updateContent()
        }
    }

    private func revealRecorder() {
        setPresentation(.recordingTape)
    }

    private func readyActionPreview() {
        if memoryActive {
            continuePreview()
        } else {
            startMemoryPreview()
        }
    }

    private func startMemoryPreview() {
        cancelPreviewTimers()
        previewMemoryActiveOverride = true
        setPresentation(.recordingTape)
        updateContent()
    }

    private func continuePreview() {
        cancelPreviewTimers()
        setPresentation(.micro)

        let timer = Timer(timeInterval: kWhisperFlowAnswerRevealDelay, repeats: false) { [weak self] _ in
            DispatchQueue.main.async {
                guard let self else { return }
                self.answerRevealTimer = nil
                self.showAnswerSummary()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        answerRevealTimer = timer
    }

    private func showAnswerSummary() {
        setPresentation(.answerSummary)
        startAnswerReturnTimer()
    }

    private func expandAnswer() {
        cancelAnswerReturnTimer(resetRemaining: true)
        setPresentation(.answerExpanded)
    }

    private func returnToDefaultPresentation() {
        cancelAnswerReturnTimer(resetRemaining: true)
        setPresentation(.recordingTape)
    }

    private func answerSummaryHoverChanged(_ hovering: Bool) {
        if hovering {
            pauseAnswerReturnTimer()
        } else {
            resumeAnswerReturnTimer()
        }
    }

    private func markPanelDragBegan() {
        answerRevealTimer?.invalidate()
        answerRevealTimer = nil
    }

    private func startAnswerReturnTimer() {
        cancelAnswerReturnTimer(resetRemaining: true)
        resumeAnswerReturnTimer()
    }

    private func pauseAnswerReturnTimer() {
        guard presentation == .answerSummary else { return }
        if let answerReturnStartedAt {
            answerReturnRemaining = max(
                0,
                answerReturnRemaining - Date().timeIntervalSince(answerReturnStartedAt)
            )
        }
        answerReturnTimer?.invalidate()
        answerReturnTimer = nil
        answerReturnStartedAt = nil
    }

    private func resumeAnswerReturnTimer() {
        guard presentation == .answerSummary, answerReturnTimer == nil else { return }
        let delay = max(0.05, answerReturnRemaining)
        answerReturnStartedAt = Date()
        let timer = Timer(timeInterval: delay, repeats: false) { [weak self] _ in
            DispatchQueue.main.async {
                guard let self, self.presentation == .answerSummary else { return }
                self.answerReturnTimer = nil
                self.answerReturnStartedAt = nil
                self.returnToDefaultPresentation()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        answerReturnTimer = timer
    }

    private func cancelAnswerReturnTimer(resetRemaining: Bool) {
        answerReturnTimer?.invalidate()
        answerReturnTimer = nil
        answerReturnStartedAt = nil
        if resetRemaining {
            answerReturnRemaining = kWhisperFlowAnswerReturnDelay
        }
    }

    private func cancelPreviewTimers() {
        answerRevealTimer?.invalidate()
        answerRevealTimer = nil
        cancelAnswerReturnTimer(resetRemaining: true)
    }

    private var targetPanelSize: NSSize {
        switch presentation {
        case .micro, .recordingTape:
            return NSSize(
                width: kWhisperFlowReadyPanelW * gOverlayScale,
                height: kWhisperFlowReadyPanelH * gOverlayScale
            )
        case .answerSummary:
            return NSSize(
                width: kWhisperFlowAnswerSummaryPanelW * gOverlayScale,
                height: kWhisperFlowAnswerSummaryPanelH * gOverlayScale
            )
        case .answerExpanded:
            return NSSize(
                width: kWhisperFlowAnswerExpandedW * gOverlayScale,
                height: kWhisperFlowAnswerExpandedH * gOverlayScale
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
        if islandModel.microPulseActive != microPulseActive {
            islandModel.microPulseActive = microPulseActive
        }
        if islandModel.captureFrameCount != snapshot.frameCount {
            islandModel.captureFrameCount = snapshot.frameCount
        }
        if islandModel.capturePulseNonce != snapshot.capturePulseNonce {
            islandModel.capturePulseNonce = snapshot.capturePulseNonce
        }
        let displayedCaptureIndicationActive =
            kWhisperFlowCapturePreviewEnabled || captureIndicationActive
        if islandModel.captureIndicationActive != displayedCaptureIndicationActive {
            islandModel.captureIndicationActive = displayedCaptureIndicationActive
        }
        let displayedCaptureStatus: WhisperFlowCaptureStatus = kWhisperFlowCapturePreviewEnabled
            ? .recording
            : captureStatus
        if islandModel.captureStatus != displayedCaptureStatus {
            islandModel.captureStatus = displayedCaptureStatus
        }

        guard hostingView == nil else { return }

        let view = WhisperFlowIslandView(
            scale: gOverlayScale,
            model: islandModel,
            onRevealRecorder: { [weak self] in
                self?.revealRecorder()
            },
            onReadyAction: { [weak self] in
                self?.readyActionPreview()
            },
            onAnswerSummaryHover: { [weak self] hovering in
                self?.answerSummaryHoverChanged(hovering)
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
            setPresentation(.answerExpanded)
            sendAction("continue")
        case "start_memory":
            setPresentation(.recordingTape)
            sendAction("start_capture")
        case "pause_memory":
            setPresentation(.recordingTape)
            sendAction("stop_capture")
        case "reconstruct_trail":
            setPresentation(.answerExpanded)
            sendAction("continue")
        case "show_trail":
            sendAction("show_trail")
        case "open_resume_point":
            sendAction("open_resume_point", decisionId: snapshot.continueDecisionId)
        case "open_main_window":
            sendAction("open_main_window")
        case "open_expanded":
            setPresentation(.answerExpanded)
        case "toggle_meeting":
            setPresentation(.recordingTape)
            sendAction(metrics.meetingActive ? "stop_capture" : "start_capture")
        case "capture_once":
            setPresentation(.recordingTape)
            sendAction("capture_once")
        case "reveal_compact", "keep_compact":
            revealRecorder()
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

    private func sendAction(
        _ action: String,
        decisionId: String? = nil,
        continueActionKind: String? = nil,
        taskSnapshotId: String? = nil,
        taskSnapshotRevision: Int64? = nil,
        affectedTaskField: String? = nil,
        taskHypothesisId: String? = nil
    ) {
        guard let callback = gActionCallback else { return }
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

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        for trackingArea in trackingAreas {
            removeTrackingArea(trackingArea)
        }
        addTrackingArea(NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .mouseMoved, .activeAlways, .inVisibleRect],
            owner: self,
            userInfo: nil
        ))
    }

    override func mouseEntered(with event: NSEvent) {
        window?.disableCursorRects()
        NSCursor.pointingHand.set()
    }

    override func mouseMoved(with event: NSEvent) {
        NSCursor.pointingHand.set()
    }

    override func mouseExited(with event: NSEvent) {
        window?.enableCursorRects()
        NSCursor.arrow.set()
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
