import AppKit
import Combine
import CoreText
import Foundation
import ImageIO
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
    var visualCue: IslandVisualCue?
    var continueHistoryPage: ContinueHistoryPageV1?
    var continueHistoryOutput: ContinueHistoryOutputV1?
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
        case visualCue = "visual_cue"
        case continueHistoryPage = "continue_history_page"
        case continueHistoryOutput = "continue_history_output"
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

private struct ContinueHistoryCursorV1: Decodable, Equatable {
    var createdAtMs: Int64
    var decisionId: String

    enum CodingKeys: String, CodingKey {
        case createdAtMs = "created_at_ms"
        case decisionId = "decision_id"
    }
}

private struct ContinueHistorySummaryV1: Decodable, Equatable, Identifiable {
    var decisionId: String
    var createdAtMs: Int64
    var origin: String
    var title: String

    var id: String { decisionId }

    enum CodingKeys: String, CodingKey {
        case decisionId = "decision_id"
        case createdAtMs = "created_at_ms"
        case origin
        case title
    }
}

private struct ContinueHistoryPageV1: Decodable, Equatable {
    var schema: String?
    var items: [ContinueHistorySummaryV1]
    var nextCursor: ContinueHistoryCursorV1?
    var requestId: UInt64
    var error: String?

    enum CodingKeys: String, CodingKey {
        case schema
        case items
        case nextCursor = "next_cursor"
        case requestId = "request_id"
        case error
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        schema = try container.decodeIfPresent(String.self, forKey: .schema)
        items = try container.decodeIfPresent([ContinueHistorySummaryV1].self, forKey: .items) ?? []
        nextCursor = try container.decodeIfPresent(ContinueHistoryCursorV1.self, forKey: .nextCursor)
        requestId = try container.decodeIfPresent(UInt64.self, forKey: .requestId) ?? 0
        error = try container.decodeIfPresent(String.self, forKey: .error)
    }
}

private struct ContinueHistoryAnswerRowV1: Decodable, Equatable, Identifiable {
    var label: String
    var value: String

    var id: String { "\(label)\u{1f}\(value)" }
}

private struct ContinueHistoryOutputV1: Decodable, Equatable {
    var schema: String?
    var decisionId: String
    var createdAtMs: Int64
    var origin: String
    var title: String
    var rows: [ContinueHistoryAnswerRowV1]
    var requestId: UInt64
    var error: String?

    enum CodingKeys: String, CodingKey {
        case schema
        case decisionId = "decision_id"
        case createdAtMs = "created_at_ms"
        case origin
        case title
        case rows
        case requestId = "request_id"
        case error
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        schema = try container.decodeIfPresent(String.self, forKey: .schema)
        decisionId = try container.decodeIfPresent(String.self, forKey: .decisionId) ?? ""
        createdAtMs = try container.decodeIfPresent(Int64.self, forKey: .createdAtMs) ?? 0
        origin = try container.decodeIfPresent(String.self, forKey: .origin) ?? ""
        title = try container.decodeIfPresent(String.self, forKey: .title) ?? ""
        rows = try container.decodeIfPresent([ContinueHistoryAnswerRowV1].self, forKey: .rows) ?? []
        requestId = try container.decodeIfPresent(UInt64.self, forKey: .requestId) ?? 0
        error = try container.decodeIfPresent(String.self, forKey: .error)
    }
}

private struct IslandVisualCue: Decodable, Equatable {
    var imagePath: String

    enum CodingKeys: String, CodingKey {
        case imagePath = "image_path"
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

private func historyDate(_ timestampMs: Int64) -> Date {
    Date(timeIntervalSince1970: TimeInterval(timestampMs) / 1_000)
}

private func historyRelativeTimestamp(_ timestampMs: Int64) -> String {
    let formatter = RelativeDateTimeFormatter()
    formatter.locale = .current
    formatter.unitsStyle = .full
    return formatter.localizedString(for: historyDate(timestampMs), relativeTo: Date())
}

private func historyFullTimestamp(_ timestampMs: Int64) -> String {
    let formatter = DateFormatter()
    formatter.locale = .current
    formatter.dateStyle = .medium
    formatter.timeStyle = .short
    return formatter.string(from: historyDate(timestampMs))
}

private struct OverlayMetrics {
    var screenActive = false
    var captureFps = 0.0
    var meetingActive = false
}

private enum Brand {
    private static let instrumentSerifPostScriptName = "InstrumentSerif-Regular"

    private static let instrumentSerifRegistered: Bool = {
        if NSFont(name: instrumentSerifPostScriptName, size: 16) != nil {
            return true
        }

        let fileName = "InstrumentSerif-Regular.ttf"
        let resourceCandidates = [
            Bundle.main.url(
                forResource: "InstrumentSerif-Regular",
                withExtension: "ttf"
            ),
            Bundle.main.resourceURL?
                .appendingPathComponent("resources/fonts")
                .appendingPathComponent(fileName),
            Bundle.main.resourceURL?
                .appendingPathComponent("fonts")
                .appendingPathComponent(fileName),
            URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
                .appendingPathComponent("src-tauri/resources/fonts")
                .appendingPathComponent(fileName),
            URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
                .appendingPathComponent("resources/fonts")
                .appendingPathComponent(fileName),
        ].compactMap { $0 }

        for url in resourceCandidates where FileManager.default.fileExists(atPath: url.path) {
            var registrationError: Unmanaged<CFError>?
            if CTFontManagerRegisterFontsForURL(url as CFURL, .process, &registrationError)
                || NSFont(name: instrumentSerifPostScriptName, size: 16) != nil {
                return true
            }
        }
        return false
    }()

    static func swiftUIFont(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        Font.system(size: size, weight: weight, design: .default)
    }

    static func instrumentSerifFont(size: CGFloat) -> Font {
        _ = instrumentSerifRegistered
        return Font.custom(instrumentSerifPostScriptName, size: size)
    }

    static func instrumentSerifNSFont(size: CGFloat) -> NSFont {
        _ = instrumentSerifRegistered
        return NSFont(name: instrumentSerifPostScriptName, size: size)
            ?? NSFont.systemFont(ofSize: size, weight: .semibold)
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
private let kWhisperFlowVisualCueCardGap: CGFloat = 8
private let kWhisperFlowVisualCueCardRadius: CGFloat = 18
private let kWhisperFlowVisualCuePadding: CGFloat = 12
private let kWhisperFlowVisualCueTitleImageGap: CGFloat = 8
private let kWhisperFlowVisualCueImageRadius: CGFloat = 12
private let kWhisperFlowVisualCueImageMaxH: CGFloat = 220
private let kWhisperFlowMicroPulseDuration: TimeInterval = 3.2
private let kWhisperFlowMicroPulseScale: CGFloat = 1.018
private let kWhisperFlowMicroPulseOutlineMinOpacity = 0.72
private let kWhisperFlowMorphDuration: TimeInterval = 0.18
private let kWhisperFlowMicroAmbientTransitionDuration: TimeInterval = 0.18
private let kWhisperFlowReducedMotionFadeDuration: TimeInterval = 0.12
private let kWhisperFlowHistoryButtonVisualSize: CGFloat = 30
private let kWhisperFlowHistoryButtonHitSize: CGFloat = 40
private let kWhisperFlowHistoryButtonGap: CGFloat = 8
private let kWhisperFlowHistoryAccessoryAllowance: CGFloat =
    kWhisperFlowHistoryButtonHitSize + kWhisperFlowHistoryButtonGap
private let kWhisperFlowHistoryCardPreferredW: CGFloat = 360
private let kWhisperFlowHistoryCardMinW: CGFloat = 320
private let kWhisperFlowHistoryCardMaxW: CGFloat = 380
private let kWhisperFlowHistoryCardPreferredH: CGFloat = 420
private let kWhisperFlowHistoryCardGap: CGFloat = 8
private let kWhisperFlowHistoryCardRadius: CGFloat = 24
private let kWhisperFlowHistoryHeaderH: CGFloat = 54
private let kWhisperFlowHistoryRowMinH: CGFloat = 58
private let kWhisperFlowHistoryTransitionDuration: TimeInterval = 0.14

private enum WhisperFlowPresentation: Equatable {
    case micro
    case ambientMemory
    case generating
    case answerSummary
    case answerExpanded
    case historyLoading
    case historyList
    case historyDetail

    var isHistory: Bool {
        switch self {
        case .historyLoading, .historyList, .historyDetail:
            return true
        default:
            return false
        }
    }
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

    var isCheckpoint: Bool { label == "Last checkpoint" }
    var isContinuation: Bool { label == "Continue from here" }
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
        title = Self.verbatim(answer?.taskSummary)
            ?? Self.verbatim(answer?.currentActivity.currentSubtask)
            ?? Self.verbatim(answer?.nextAction)
            ?? Self.verbatim(answer?.unfinishedState)
            ?? Self.verbatim(answer?.lastMeaningfulProgress)
            ?? Self.fallbackTitle(for: state.displayState)

        var nextRows: [WhisperFlowAnswerRow] = []
        Self.append(
            &nextRows,
            label: "Current activity",
            value: Self.currentActivitySummary(answer, excluding: title)
        )
        Self.append(&nextRows, label: "Last checkpoint", value: answer?.lastMeaningfulProgress)
        Self.append(
            &nextRows,
            label: "Continue from here",
            value: Self.usefulContinuation(answer, fallback: state.nextAction)
        )
        Self.append(&nextRows, label: "Return to", value: answer?.whereSummary)
        rows = nextRows
    }

    private static func currentActivitySummary(
        _ answer: IslandSemanticAnswer?,
        excluding title: String
    ) -> String? {
        guard let answer else { return nil }
        var details: [String] = []
        for value in [
            answer.currentActivity.currentSubtask,
            answer.currentActivity.immediateUserOperation,
            answer.currentActivity.semanticEffectOfOperation,
            answer.taskObject,
        ] {
            guard let value = verbatim(value) else { continue }
            appendUnique(value, to: &details, excluding: title)
        }

        if let surface = verbatim(answer.currentActivity.observedSurface),
           !details.contains(where: { normalized($0).contains(normalized(surface)) }) {
            appendUnique("Working in \(surface).", to: &details, excluding: title)
        }
        return details.isEmpty ? nil : details.joined(separator: "\n\n")
    }

    private static func appendUnique(
        _ value: String,
        to values: inout [String],
        excluding title: String
    ) {
        let normalizedValue = normalized(value)
        guard !normalizedValue.isEmpty,
              normalizedValue != normalized(title),
              !values.contains(where: { normalized($0) == normalizedValue }) else { return }
        values.append(value)
    }

    private static func usefulContinuation(
        _ answer: IslandSemanticAnswer?,
        fallback: String?
    ) -> String? {
        for candidate in [answer?.nextAction, fallback, answer?.unfinishedState] {
            guard let value = verbatim(candidate), !isGenericUnresolvedCopy(value) else { continue }
            return value
        }
        return nil
    }

    private static func isGenericUnresolvedCopy(_ value: String) -> Bool {
        let value = normalized(value)
        return value.contains("task remains unresolved")
            || value.contains("no active task or test is visible")
            || value.contains("no unfinished step was clearly captured")
    }

    private static func normalized(_ value: String) -> String {
        value
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .split(whereSeparator: { $0.isWhitespace })
            .joined(separator: " ")
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
    var answerCardHeight: CGFloat = kWhisperFlowAnswerExpandedMinH
    var contentViewportHeight: CGFloat = kWhisperFlowAnswerExpandedMinH
        - kWhisperFlowAnswerVerticalPadding * 2
    var visualCueCardHeight: CGFloat = 0
    var visualCueImageHeight: CGFloat = 0
}

private struct WhisperFlowHistoryLayout: Equatable {
    var cardWidth: CGFloat = kWhisperFlowHistoryCardPreferredW
    var cardHeight: CGFloat = kWhisperFlowHistoryCardPreferredH
    var canvasWidth: CGFloat = kWhisperFlowHistoryCardPreferredW
    var canvasHeight: CGFloat = kWhisperFlowHistoryCardPreferredH
    var capsuleOffsetX: CGFloat = 0
    var controlOnLeft = true
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
    @Published var blockedContinueShakeNonce: UInt64 = 0
    @Published var answer: WhisperFlowAnswerContent?
    @Published var answerLayout = WhisperFlowAnswerLayout()
    @Published var visualCueImage: NSImage? = nil
    @Published var visualCuePresented = false
    @Published var historyOriginPresentation: WhisperFlowPresentation = .ambientMemory
    @Published var historyButtonVisible = false
    @Published var historyItems: [ContinueHistorySummaryV1] = []
    @Published var historyNextCursor: ContinueHistoryCursorV1? = nil
    @Published var historyError: String? = nil
    @Published var historyLoadingOlder = false
    @Published var historySelectedOutput: ContinueHistoryOutputV1? = nil
    @Published var historySelectedDecisionId: String? = nil
    @Published var historyDetailLoading = false
    @Published var historyDetailError: String? = nil
    @Published var currentDecisionId: String? = nil
    @Published var historyLayout = WhisperFlowHistoryLayout()
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
    let onToggleVisualCue: () -> Void
    let onToggleHistory: () -> Void
    let onHistoryButtonHover: (Bool) -> Void
    let onLoadOlderHistory: () -> Void
    let onRetryHistory: () -> Void
    let onRetryHistoryDetail: () -> Void
    let onSelectHistoryOutput: (String) -> Void
    let onBackFromHistoryDetail: () -> Void
    let onDismissOneLevel: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AccessibilityFocusState private var historyHeadingFocused: Bool
    @AccessibilityFocusState private var historyDetailFocused: Bool
    @AccessibilityFocusState private var historyButtonFocused: Bool
    @State private var microPulseExpanded = false
    @State private var ambientBodyHovered = false
    @State private var ambientBodyHoverArmed = false
    @State private var blockedAmbientBodyHoverObserved = false
    @State private var ambientCapsuleHovered = false
    @State private var pausedRestartHovered = false
    @State private var arrowHovered = false
    @State private var historyButtonHovered = false
    @State private var memoryTransitionCountdownProgress: CGFloat = 0
    @State private var blockedContinueShakeOffset: CGFloat = 0
    @State private var blockedContinueShakeSequence: UInt64 = 0

    private func s(_ value: CGFloat) -> CGFloat { value * scale }

    var body: some View {
        ZStack(alignment: .top) {
            currentIslandContent(for: renderedIslandPresentation)
                .offset(x: s(model.historyLayout.capsuleOffsetX))
                .allowsHitTesting(!model.presentation.isHistory)

            if model.historyButtonVisible || model.presentation.isHistory {
                historyButton
                    .offset(
                        x: s(historyButtonOffsetX),
                        y: s(historyButtonOffsetY)
                    )
            }

            if model.presentation.isHistory {
                historyCard
                    .padding(.top, s(historyCardTop))
                    .transition(stateTransition(scale: 0.97))
            }
        }
        .frame(
            width: s(model.historyLayout.canvasWidth),
            height: s(model.historyLayout.canvasHeight),
            alignment: .top
        )
        .background(Color.clear)
        .animation(presentationAnimation, value: model.presentation)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .onChange(of: model.presentation) { presentation in
            if presentation == .historyLoading || presentation == .historyList {
                DispatchQueue.main.async {
                    historyHeadingFocused = true
                }
            } else if presentation == .historyDetail {
                DispatchQueue.main.async {
                    historyDetailFocused = true
                }
            } else if !presentation.isHistory {
                DispatchQueue.main.async {
                    historyButtonFocused = true
                }
            }
        }
        .onExitCommand(perform: onDismissOneLevel)
    }

    @ViewBuilder
    private func currentIslandContent(
        for presentation: WhisperFlowPresentation
    ) -> some View {
        switch presentation {
        case .micro, .ambientMemory, .generating:
            memoryContinuityView
        case .answerSummary:
            answerSummaryView
                .transition(.opacity)
        case .answerExpanded:
            answerExpandedView
                .transition(stateTransition(scale: 0.97))
        case .historyLoading, .historyList, .historyDetail:
            EmptyView()
        }
    }

    private var renderedIslandPresentation: WhisperFlowPresentation {
        model.presentation.isHistory
            ? model.historyOriginPresentation
            : model.presentation
    }

    private var memoryContinuityView: some View {
        let expanded = renderedIslandPresentation == .ambientMemory
            || renderedIslandPresentation == .generating
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
        .offset(x: s(blockedContinueShakeOffset))
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
        .onChange(of: model.blockedContinueShakeNonce) { _ in
            runBlockedContinueShake()
        }
        .onDisappear {
            microPulseExpanded = false
            blockedContinueShakeSequence &+= 1
            blockedContinueShakeOffset = 0
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
                .accessibilityLabel(
                    model.memoryActive
                        ? "Show what I was doing"
                        : "Start memory before generating an answer"
                )
                .help(model.memoryActive ? "Show what I was doing" : "Start memory first")
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

    private func runBlockedContinueShake() {
        blockedContinueShakeSequence &+= 1
        let sequence = blockedContinueShakeSequence
        blockedContinueShakeOffset = 0

        let offsets: [CGFloat] = shouldReduceMotion
            ? [-2, 2, 0]
            : [-10, 10, -10, 10, -10, 10, -10, 8, -8, 0]
        let stepDuration = shouldReduceMotion ? 0.06 : 0.05

        for (index, offset) in offsets.enumerated() {
            DispatchQueue.main.asyncAfter(
                deadline: .now() + stepDuration * Double(index + 1)
            ) {
                guard blockedContinueShakeSequence == sequence else { return }
                withAnimation(.linear(duration: stepDuration)) {
                    blockedContinueShakeOffset = offset
                }
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
        case .historyLoading, .historyList, .historyDetail:
            microPulseExpanded = false
            cleanUpAmbientInteractionState()
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
        VStack(alignment: .leading, spacing: s(kWhisperFlowVisualCueCardGap)) {
            answerExpandedCard

            if model.visualCuePresented,
               let image = model.visualCueImage,
               model.answerLayout.visualCueCardHeight > 0 {
                visualCueCard(image)
                    .transition(.opacity)
            }
        }
        .frame(
            width: s(model.answerLayout.expandedWidth),
            height: s(model.answerLayout.expandedHeight),
            alignment: .top
        )
        .accessibilityElement(children: .contain)
    }

    private var answerExpandedCard: some View {
        let answer = model.answer ?? .unavailable

        return ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: s(kWhisperFlowAnswerHeaderSpacing)) {
                HStack(spacing: s(8)) {
                    Text(answer.title)
                        .font(Brand.instrumentSerifFont(size: s(24)))
                        .foregroundColor(.white)
                        .lineSpacing(s(1))
                        .fixedSize(horizontal: false, vertical: true)
                        .layoutPriority(1)
                        .accessibilityAddTraits(.isHeader)

                    Spacer(minLength: s(8))

                    if model.visualCueImage != nil {
                        Button(action: onToggleVisualCue) {
                            Text("Visual cue")
                                .font(Brand.swiftUIFont(size: s(11), weight: .semibold))
                                .foregroundColor(WhisperFlowStyle.accent)
                                .fixedSize(horizontal: true, vertical: false)
                                .padding(.horizontal, s(8))
                                .padding(.vertical, s(4))
                                .background(
                                    Capsule()
                                        .fill(
                                            WhisperFlowStyle.accent.opacity(
                                                model.visualCuePresented ? 0.18 : 0.07
                                            )
                                        )
                                )
                                .overlay(
                                    Capsule()
                                        .stroke(
                                            model.visualCuePresented
                                                ? WhisperFlowStyle.accent.opacity(0.48)
                                                : WhisperFlowStyle.outline,
                                            lineWidth: s(1)
                                        )
                                )
                        }
                        .buttonStyle(WhisperFlowPressButtonStyle(reduceMotion: reduceMotion))
                        .fixedSize(horizontal: true, vertical: false)
                        .layoutPriority(2)
                        .accessibilityLabel(
                            model.visualCuePresented ? "Hide visual cue" : "Show visual cue"
                        )
                    }

                    Button(action: onCollapseAnswer) {
                        Text("See less")
                            .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                            .foregroundColor(WhisperFlowStyle.accent)
                            .fixedSize(horizontal: true, vertical: false)
                    }
                    .buttonStyle(.plain)
                    .fixedSize(horizontal: true, vertical: false)
                    .layoutPriority(2)
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
                                    .font(
                                        Brand.swiftUIFont(
                                            size: s(row.isContinuation ? 17 : row.isCheckpoint ? 16 : 14),
                                            weight: row.isContinuation || row.isCheckpoint
                                                ? .semibold
                                                : .regular
                                        )
                                    )
                                    .foregroundColor(
                                        row.isContinuation
                                            ? .white
                                            : .white.opacity(row.isCheckpoint ? 0.96 : 0.88)
                                    )
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
            height: s(model.answerLayout.answerCardHeight),
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

    private func visualCueCard(_ image: NSImage) -> some View {
        VStack(alignment: .leading, spacing: s(kWhisperFlowVisualCueTitleImageGap)) {
            Text("Visual cue")
                .font(Brand.swiftUIFont(size: s(11), weight: .semibold))
                .foregroundColor(WhisperFlowStyle.accent)
                .accessibilityLabel("Visual cue")

            Image(nsImage: image)
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(
                    maxWidth: .infinity,
                    minHeight: s(model.answerLayout.visualCueImageHeight),
                    maxHeight: s(model.answerLayout.visualCueImageHeight),
                    alignment: .center
                )
                .clipShape(
                    RoundedRectangle(
                        cornerRadius: s(kWhisperFlowVisualCueImageRadius),
                        style: .continuous
                    )
                )
                .accessibilityLabel("Full-screen evidence used for this answer")
        }
        .padding(s(kWhisperFlowVisualCuePadding))
        .frame(
            width: s(model.answerLayout.expandedWidth),
            height: s(model.answerLayout.visualCueCardHeight),
            alignment: .topLeading
        )
        .background(WhisperFlowStyle.surface)
        .overlay(
            RoundedRectangle(
                cornerRadius: s(kWhisperFlowVisualCueCardRadius),
                style: .continuous
            )
            .stroke(WhisperFlowStyle.outline, lineWidth: s(1))
        )
        .clipShape(
            RoundedRectangle(
                cornerRadius: s(kWhisperFlowVisualCueCardRadius),
                style: .continuous
            )
        )
        .accessibilityElement(children: .contain)
    }

    private var historyButton: some View {
        Button(action: onToggleHistory) {
            Image(systemName: "clock.arrow.circlepath")
                .font(.system(size: s(13), weight: .semibold))
                .foregroundColor(
                    model.presentation.isHistory || historyButtonHovered
                        ? WhisperFlowStyle.accent
                        : .white.opacity(0.86)
                )
                .frame(
                    width: s(kWhisperFlowHistoryButtonVisualSize),
                    height: s(kWhisperFlowHistoryButtonVisualSize)
                )
                .background(
                    Circle()
                        .fill(WhisperFlowStyle.surface)
                )
                .overlay(
                    Circle()
                        .stroke(
                            model.presentation.isHistory
                                ? WhisperFlowStyle.accent.opacity(0.58)
                                : WhisperFlowStyle.outline,
                            lineWidth: s(1)
                        )
                )
                .offset(
                    y: s((renderedCapsuleHeight - kWhisperFlowHistoryButtonHitSize) / 2)
                )
                .frame(
                    width: s(kWhisperFlowHistoryButtonHitSize),
                    height: s(kWhisperFlowHistoryButtonHitSize)
                )
                .contentShape(Circle())
        }
        .buttonStyle(WhisperFlowPressButtonStyle(reduceMotion: shouldReduceMotion))
        .accessibilityLabel("Continue history")
        .accessibilityHint(
            model.presentation.isHistory
                ? "Closes Continue history"
                : "Shows previous Continue answers"
        )
        .accessibilityFocused($historyButtonFocused)
        .help("Continue history")
        .onHover { hovering in
            historyButtonHovered = hovering
            onHistoryButtonHover(hovering)
            if hovering {
                NSCursor.pointingHand.set()
            } else {
                NSCursor.arrow.set()
            }
        }
    }

    private var historyCard: some View {
        Group {
            switch model.presentation {
            case .historyLoading:
                historyLoadingCard
            case .historyList:
                historyListCard
            case .historyDetail:
                historyDetailCard
            default:
                EmptyView()
            }
        }
        .id(model.presentation)
        .transition(stateTransition(scale: 0.97))
        .frame(
            width: s(model.historyLayout.cardWidth),
            height: s(model.historyLayout.cardHeight),
            alignment: .top
        )
        .background(WhisperFlowStyle.surface)
        .overlay(
            RoundedRectangle(
                cornerRadius: s(kWhisperFlowHistoryCardRadius),
                style: .continuous
            )
            .stroke(WhisperFlowStyle.outline, lineWidth: s(1))
        )
        .clipShape(
            RoundedRectangle(
                cornerRadius: s(kWhisperFlowHistoryCardRadius),
                style: .continuous
            )
        )
        .accessibilityElement(children: .contain)
    }

    private var historyLoadingCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            historyHeader(title: "History")

            VStack(spacing: s(8)) {
                ForEach(0..<4, id: \.self) { index in
                    VStack(alignment: .leading, spacing: s(8)) {
                        RoundedRectangle(cornerRadius: s(3), style: .continuous)
                            .fill(Color.white.opacity(0.12))
                            .frame(
                                width: s(index == 3 ? 164 : 226),
                                height: s(10)
                            )
                        RoundedRectangle(cornerRadius: s(3), style: .continuous)
                            .fill(Color.white.opacity(0.07))
                            .frame(width: s(78), height: s(8))
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, s(18))
                    .padding(.vertical, s(10))
                    .accessibilityHidden(true)
                }
            }
            .padding(.top, s(4))

            Spacer(minLength: 0)
        }
        .accessibilityLabel("Loading Continue history")
    }

    private var historyListCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            historyHeader(title: "History")

            if let error = model.historyError?.nonEmpty {
                historyErrorState(error)
            } else if model.historyItems.isEmpty {
                historyEmptyState
            } else {
                ScrollView(.vertical, showsIndicators: true) {
                    LazyVStack(spacing: 0) {
                        ForEach(model.historyItems) { item in
                            historyRow(item)
                        }

                        if model.historyNextCursor != nil {
                            Button(action: onLoadOlderHistory) {
                                HStack(spacing: s(7)) {
                                    if model.historyLoadingOlder {
                                        ProgressView()
                                            .controlSize(.small)
                                            .tint(WhisperFlowStyle.accent)
                                    }
                                    Text(
                                        model.historyLoadingOlder
                                            ? "Loading older answers…"
                                            : "Load older answers"
                                    )
                                    .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                                    .foregroundColor(WhisperFlowStyle.accent)
                                }
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, s(15))
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .disabled(model.historyLoadingOlder)
                            .accessibilityLabel(
                                model.historyLoadingOlder
                                    ? "Loading older Continue answers"
                                    : "Load older Continue answers"
                            )
                        }
                    }
                }
            }
        }
    }

    private func historyHeader(title: String) -> some View {
        HStack(spacing: s(8)) {
            Text(title)
                .font(Brand.swiftUIFont(size: s(14), weight: .semibold))
                .foregroundColor(.white)
                .accessibilityAddTraits(.isHeader)
                .accessibilityFocused($historyHeadingFocused)

            Spacer(minLength: 0)

            Text("Latest 100")
                .font(Brand.swiftUIFont(size: s(10), weight: .medium))
                .foregroundColor(.white.opacity(0.58))
                .accessibilityHidden(true)
        }
        .padding(.horizontal, s(18))
        .frame(height: s(kWhisperFlowHistoryHeaderH))
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(WhisperFlowStyle.outline.opacity(0.74))
                .frame(height: s(1))
        }
    }

    private func historyRow(_ item: ContinueHistorySummaryV1) -> some View {
        let fullTimestamp = historyFullTimestamp(item.createdAtMs)
        let isCurrent = item.decisionId == model.currentDecisionId

        return Button {
            onSelectHistoryOutput(item.decisionId)
        } label: {
            HStack(alignment: .center, spacing: s(12)) {
                VStack(alignment: .leading, spacing: s(6)) {
                    Text(item.title)
                        .font(Brand.swiftUIFont(size: s(13), weight: .semibold))
                        .foregroundColor(.white)
                        .lineLimit(2)
                        .multilineTextAlignment(.leading)
                        .frame(maxWidth: .infinity, alignment: .leading)

                    Text(historyRelativeTimestamp(item.createdAtMs))
                        .font(Brand.swiftUIFont(size: s(11), weight: .regular))
                        .foregroundColor(.white.opacity(0.62))
                }

                if isCurrent {
                    Text("Current")
                        .font(Brand.swiftUIFont(size: s(10), weight: .semibold))
                        .foregroundColor(WhisperFlowStyle.accent)
                        .padding(.horizontal, s(7))
                        .padding(.vertical, s(4))
                        .background(
                            Capsule()
                                .fill(WhisperFlowStyle.accent.opacity(0.10))
                        )
                        .overlay(
                            Capsule()
                                .stroke(WhisperFlowStyle.accent.opacity(0.34), lineWidth: s(1))
                        )
                } else {
                    Image(systemName: "chevron.right")
                        .font(.system(size: s(9), weight: .semibold))
                        .foregroundColor(.white.opacity(0.36))
                        .accessibilityHidden(true)
                }
            }
            .padding(.horizontal, s(18))
            .padding(.vertical, s(11))
            .frame(minHeight: s(kWhisperFlowHistoryRowMinH))
            .contentShape(Rectangle())
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(WhisperFlowStyle.outline.opacity(0.64))
                    .frame(height: s(1))
                    .padding(.leading, s(18))
            }
        }
        .buttonStyle(.plain)
        .help(fullTimestamp)
        .accessibilityLabel(
            "\(item.title). \(fullTimestamp). \(isCurrent ? "Current answer" : "Past answer")"
        )
        .accessibilityHint("Opens this saved Continue answer")
    }

    private var historyEmptyState: some View {
        VStack(spacing: s(7)) {
            Spacer(minLength: 0)
            Image(systemName: "clock.arrow.circlepath")
                .font(.system(size: s(18), weight: .medium))
                .foregroundColor(.white.opacity(0.46))
                .accessibilityHidden(true)
            Text("No previous answers yet")
                .font(Brand.swiftUIFont(size: s(14), weight: .semibold))
                .foregroundColor(.white)
            Text("Use Continue to create one")
                .font(Brand.swiftUIFont(size: s(12), weight: .regular))
                .foregroundColor(.white.opacity(0.62))
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityElement(children: .combine)
    }

    private func historyErrorState(_ error: String) -> some View {
        VStack(spacing: s(8)) {
            Spacer(minLength: 0)
            Text("History unavailable")
                .font(Brand.swiftUIFont(size: s(14), weight: .semibold))
                .foregroundColor(.white)
            Text(error)
                .font(Brand.swiftUIFont(size: s(12), weight: .regular))
                .foregroundColor(.white.opacity(0.62))
                .multilineTextAlignment(.center)
                .lineLimit(3)
            Button("Retry", action: onRetryHistory)
                .buttonStyle(.plain)
                .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                .foregroundColor(WhisperFlowStyle.accent)
                .padding(.top, s(3))
            Spacer(minLength: 0)
        }
        .padding(.horizontal, s(28))
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityElement(children: .contain)
    }

    private var historyDetailCard: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: s(10)) {
                Button(action: onBackFromHistoryDetail) {
                    HStack(spacing: s(5)) {
                        Image(systemName: "chevron.left")
                            .font(.system(size: s(9), weight: .bold))
                        Text("Back")
                    }
                    .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                    .foregroundColor(WhisperFlowStyle.accent)
                    .frame(minWidth: s(54), minHeight: s(30), alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Back to Continue history")
                .accessibilityFocused($historyDetailFocused)

                Spacer(minLength: 0)

                if let output = model.historySelectedOutput {
                    Text("Past answer · \(historyFullTimestamp(output.createdAtMs))")
                        .font(Brand.swiftUIFont(size: s(10), weight: .medium))
                        .foregroundColor(.white.opacity(0.58))
                        .multilineTextAlignment(.trailing)
                        .lineLimit(2)
                        .help(historyFullTimestamp(output.createdAtMs))
                } else {
                    Text("Past answer")
                        .font(Brand.swiftUIFont(size: s(10), weight: .medium))
                        .foregroundColor(.white.opacity(0.58))
                }
            }
            .padding(.horizontal, s(18))
            .frame(height: s(kWhisperFlowHistoryHeaderH))
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(WhisperFlowStyle.outline.opacity(0.74))
                    .frame(height: s(1))
            }

            if model.historyDetailLoading {
                VStack(spacing: s(9)) {
                    ProgressView()
                        .controlSize(.small)
                        .tint(WhisperFlowStyle.accent)
                    Text("Loading saved answer…")
                        .font(Brand.swiftUIFont(size: s(12), weight: .regular))
                        .foregroundColor(.white.opacity(0.62))
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .accessibilityElement(children: .combine)
            } else if let error = model.historyDetailError?.nonEmpty {
                VStack(spacing: s(8)) {
                    Spacer(minLength: 0)
                    Text("Answer unavailable")
                        .font(Brand.swiftUIFont(size: s(14), weight: .semibold))
                        .foregroundColor(.white)
                    Text(error)
                        .font(Brand.swiftUIFont(size: s(12), weight: .regular))
                        .foregroundColor(.white.opacity(0.62))
                        .multilineTextAlignment(.center)
                    Button("Retry", action: onRetryHistoryDetail)
                        .buttonStyle(.plain)
                        .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                        .foregroundColor(WhisperFlowStyle.accent)
                    Spacer(minLength: 0)
                }
                .padding(.horizontal, s(28))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let output = model.historySelectedOutput {
                ScrollView(.vertical, showsIndicators: true) {
                    VStack(alignment: .leading, spacing: s(kWhisperFlowAnswerHeaderSpacing)) {
                        Text(output.title)
                            .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                            .foregroundColor(.white)
                            .fixedSize(horizontal: false, vertical: true)
                            .frame(maxWidth: .infinity, alignment: .leading)

                        if !output.rows.isEmpty {
                            VStack(alignment: .leading, spacing: s(kWhisperFlowAnswerRowSpacing)) {
                                ForEach(output.rows) { row in
                                    VStack(
                                        alignment: .leading,
                                        spacing: s(kWhisperFlowAnswerLabelValueSpacing)
                                    ) {
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
                        }
                    }
                    .padding(.horizontal, s(18))
                    .padding(.vertical, s(18))
                }
                .accessibilityLabel(
                    "Past Continue answer from \(historyFullTimestamp(output.createdAtMs))"
                )
            }
        }
    }

    private var historyButtonOffsetX: CGFloat {
        let direction: CGFloat = model.historyLayout.controlOnLeft ? -1 : 1
        return model.historyLayout.capsuleOffsetX
            + direction * (
                renderedCapsuleWidth / 2
                    + kWhisperFlowHistoryButtonGap
                    + kWhisperFlowHistoryButtonVisualSize / 2
            )
    }

    private var historyButtonOffsetY: CGFloat {
        max(0, (renderedCapsuleHeight - kWhisperFlowHistoryButtonHitSize) / 2)
    }

    private var renderedCapsuleWidth: CGFloat {
        switch renderedIslandPresentation {
        case .answerSummary:
            return model.answerLayout.summaryWidth
        case .ambientMemory:
            return ambientCapsuleWidth
        default:
            return 0
        }
    }

    private var renderedCapsuleHeight: CGFloat {
        switch renderedIslandPresentation {
        case .answerSummary:
            return kWhisperFlowAnswerSummaryH
        case .ambientMemory:
            return ambientCapsuleHeight
        default:
            return 0
        }
    }

    private var historyCardTop: CGFloat {
        renderedCapsuleHeight + kWhisperFlowHistoryCardGap
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

    private var presentationAnimation: Animation {
        guard model.presentation.isHistory else { return morphAnimation }
        return .timingCurve(
            0.23,
            1,
            0.32,
            1,
            duration: shouldReduceMotion
                ? kWhisperFlowReducedMotionFadeDuration
                : kWhisperFlowHistoryTransitionDuration
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
    private var blockedContinueShakeNonce: UInt64 = 0
    private var latchedAnswer: WhisperFlowAnswerContent?
    private var latchedDecisionId: String?
    private var latchedVisualCue: IslandVisualCue?
    private var visualCueImage: NSImage?
    private var visualCuePresented = false
    private var visualCueLoadNonce: UInt64 = 0
    private var answerLayout = WhisperFlowAnswerLayout()
    private var memoryTransitionTimer: Timer?
    private var ambientHoverReturnTimer: Timer?
    private var memoryTransitionCountdownActive = false
    private var memoryTransitionCountdownNonce: UInt64 = 0
    private var startingFeedbackNonceCounter: UInt64 = 0
    private var activeStartingFeedbackNonce: UInt64?
    private var historyOriginPresentation: WhisperFlowPresentation = .ambientMemory
    private var historyItems: [ContinueHistorySummaryV1] = []
    private var historyNextCursor: ContinueHistoryCursorV1?
    private var historyError: String?
    private var historyLoadingOlder = false
    private var historySelectedOutput: ContinueHistoryOutputV1?
    private var historySelectedDecisionId: String?
    private var historyDetailLoading = false
    private var historyDetailError: String?
    private var historyRequestId: UInt64 = 0
    private var activeHistoryPageRequestId: UInt64?
    private var activeHistoryDetailRequestId: UInt64?
    private var historyAnchor: NSPoint?
    private var historyLayout = WhisperFlowHistoryLayout()

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

        processHistoryResponses(from: snapshot)

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
            if presentation.isHistory {
                closeHistory()
            }
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
        clearVisualCue()
        clearHistoryState()
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
        clearVisualCue()
        clearHistoryState()
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
        guard memoryActive else {
            blockedContinueShakeNonce &+= 1
            updateContent()
            return
        }
        cancelPresentationTimers()
        continueRequestInFlight = true
        latchedAnswer = nil
        latchedDecisionId = nil
        clearVisualCue()
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
        clearVisualCue()
        let answer = WhisperFlowAnswerContent(snapshot: snapshot)
        latchedAnswer = answer
        latchedDecisionId = answer.decisionId
        latchedVisualCue = snapshot.visualCue
        refreshAnswerLayout()
        setPresentation(.answerSummary)
        updateContent()
        beginVisualCueLoad(for: snapshot.visualCue, decisionId: answer.decisionId)
    }

    private func clearVisualCue() {
        visualCueLoadNonce &+= 1
        latchedVisualCue = nil
        visualCueImage = nil
        visualCuePresented = false
        if islandModel.visualCueImage != nil {
            islandModel.visualCueImage = nil
        }
        if islandModel.visualCuePresented {
            islandModel.visualCuePresented = false
        }
    }

    private func beginVisualCueLoad(for cue: IslandVisualCue?, decisionId: String?) {
        guard let cue,
              !cue.imagePath.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return
        }

        visualCueLoadNonce &+= 1
        let loadNonce = visualCueLoadNonce
        let imagePath = cue.imagePath
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let data = try? Data(
                contentsOf: URL(fileURLWithPath: imagePath),
                options: [.mappedIfSafe]
            ),
            let source = CGImageSourceCreateWithData(data as CFData, nil),
            let decodedImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
                return
            }

            DispatchQueue.main.async {
                guard let self,
                      self.visualCueLoadNonce == loadNonce,
                      self.latchedDecisionId == decisionId,
                      self.latchedVisualCue?.imagePath == imagePath else { return }

                let image = NSImage(
                    cgImage: decodedImage,
                    size: NSSize(width: decodedImage.width, height: decodedImage.height)
                )
                self.visualCueImage = image
                self.refreshAnswerLayout()

                let reduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
                if reduceMotion {
                    self.updateContent()
                } else {
                    withAnimation(.easeOut(duration: kWhisperFlowReducedMotionFadeDuration)) {
                        self.islandModel.visualCueImage = image
                        self.islandModel.answerLayout = self.answerLayout
                    }
                }

                if self.presentation == .answerExpanded {
                    self.positionPanel(
                        preserveCurrentAnchor: true,
                        animated: self.visible && !reduceMotion
                    )
                }
            }
        }
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

        let titleFont = Brand.instrumentSerifNSFont(size: 24)
        let labelFont = NSFont.systemFont(ofSize: 11, weight: .semibold)
        let collapseFont = NSFont.systemFont(ofSize: 12, weight: .semibold)
        let measuredAnswerWidths: [CGFloat] = [
            measuredTextWidth(answer.title, font: titleFont),
        ] + answer.rows.flatMap { row -> [CGFloat] in
                let valueFont = answerValueNSFont(for: row)
                return [
                    measuredTextWidth(row.label, font: labelFont),
                    measuredTextWidth(row.value, font: valueFont),
                ]
            }
        let longestTextWidth = measuredAnswerWidths.max() ?? 0
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
        let cueButtonWidth = visualCueImage == nil
            ? 0
            : measuredTextWidth("Visual cue", font: labelFont) + 16 + 8
        let titleWidth = max(1, contentWidth - collapseWidth - cueButtonWidth - 16)
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
                font: answerValueNSFont(for: row),
                width: contentWidth,
                lineSpacing: 3
            )
        }

        let bodySpacing = answer.rows.isEmpty ? 0 : kWhisperFlowAnswerHeaderSpacing
        let naturalAnswerHeight = max(
            kWhisperFlowAnswerExpandedMinH,
            ceil(
                kWhisperFlowAnswerVerticalPadding * 2
                    + headerHeight
                    + bodySpacing
                    + bodyHeight
            )
        )
        let maximumStackHeight = max(
            kWhisperFlowAnswerExpandedMinH,
            usableHeight * kWhisperFlowAnswerExpandedMaxScreenFraction
        )
        var answerCardHeight = min(naturalAnswerHeight, maximumStackHeight)
        var visualCueCardHeight: CGFloat = 0
        var visualCueImageHeight: CGFloat = 0

        if visualCuePresented,
           let image = visualCueImage,
           image.size.width > 0,
           image.size.height > 0 {
            let cueContentWidth = max(
                1,
                expandedWidth - kWhisperFlowVisualCuePadding * 2
            )
            let titleHeight = measuredTextHeight(
                "Visual cue",
                font: labelFont,
                width: cueContentWidth
            )
            let cueChromeHeight = kWhisperFlowVisualCuePadding * 2
                + titleHeight
                + kWhisperFlowVisualCueTitleImageGap
            let naturalImageHeight = min(
                kWhisperFlowVisualCueImageMaxH,
                cueContentWidth * image.size.height / image.size.width
            )
            let imageRoomAfterMinimumAnswer = maximumStackHeight
                - kWhisperFlowAnswerExpandedMinH
                - kWhisperFlowVisualCueCardGap
                - cueChromeHeight

            if imageRoomAfterMinimumAnswer >= 1 {
                visualCueImageHeight = min(naturalImageHeight, imageRoomAfterMinimumAnswer)
                visualCueCardHeight = cueChromeHeight + visualCueImageHeight
                let answerRoom = maximumStackHeight
                    - kWhisperFlowVisualCueCardGap
                    - visualCueCardHeight
                answerCardHeight = min(
                    naturalAnswerHeight,
                    max(kWhisperFlowAnswerExpandedMinH, answerRoom)
                )
            }
        }

        let expandedHeight = answerCardHeight
            + (visualCueCardHeight > 0
                ? kWhisperFlowVisualCueCardGap + visualCueCardHeight
                : 0)
        let contentViewportHeight = max(
            0,
            answerCardHeight - kWhisperFlowAnswerVerticalPadding * 2
        )

        answerLayout = WhisperFlowAnswerLayout(
            summaryWidth: summaryWidth,
            summaryPanelWidth: summaryWidth + kWhisperFlowAnswerSummaryPanelMarginW,
            expandedWidth: expandedWidth,
            expandedHeight: expandedHeight,
            answerCardHeight: answerCardHeight,
            contentViewportHeight: contentViewportHeight,
            visualCueCardHeight: visualCueCardHeight,
            visualCueImageHeight: visualCueImageHeight
        )
    }

    private func answerValueNSFont(for row: WhisperFlowAnswerRow) -> NSFont {
        if row.isContinuation {
            return NSFont.systemFont(ofSize: 17, weight: .semibold)
        }
        if row.isCheckpoint {
            return NSFont.systemFont(ofSize: 16, weight: .semibold)
        }
        return NSFont.systemFont(ofSize: 14, weight: .regular)
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
        visualCuePresented = false
        refreshAnswerLayout()
        setPresentation(.answerSummary)
    }

    private func expandAnswer() {
        guard latchedAnswer != nil else { return }
        visualCuePresented = false
        refreshAnswerLayout()
        setPresentation(.answerExpanded)
    }

    private func toggleVisualCue() {
        guard presentation == .answerExpanded, visualCueImage != nil else { return }
        visualCuePresented.toggle()
        refreshAnswerLayout()

        let reduceMotion = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        withAnimation(
            .timingCurve(
                0.23,
                1,
                0.32,
                1,
                duration: reduceMotion
                    ? kWhisperFlowReducedMotionFadeDuration
                    : kWhisperFlowMorphDuration
            )
        ) {
            islandModel.visualCuePresented = visualCuePresented
            islandModel.answerLayout = answerLayout
        }
        positionPanel(
            preserveCurrentAnchor: true,
            animated: visible && !reduceMotion
        )
    }

    private func toggleHistory() {
        if presentation.isHistory {
            closeHistory()
        } else {
            openHistory()
        }
    }

    private func openHistory() {
        guard historyButtonShouldBeVisible,
              presentation == .ambientMemory || presentation == .answerSummary else { return }
        cancelPresentationTimers()
        historyOriginPresentation = presentation
        if let panel {
            historyAnchor = NSPoint(x: panel.frame.midX, y: panel.frame.maxY)
        }
        historyItems = []
        historyNextCursor = nil
        historyError = nil
        historyLoadingOlder = false
        historySelectedOutput = nil
        historySelectedDecisionId = nil
        historyDetailLoading = false
        historyDetailError = nil
        let requestId = nextHistoryRequestId()
        activeHistoryPageRequestId = requestId
        setPresentation(.historyLoading)
        if !sendAction(
            "open_continue_history",
            historyRequestId: requestId
        ) {
            activeHistoryPageRequestId = nil
            historyError = "Smalltalk could not request saved answers."
            setPresentation(.historyList)
        }
    }

    private func closeHistory() {
        guard presentation.isHistory else { return }
        let origin = historyOriginPresentation
        setPresentation(origin)
        historyAnchor = nil
        historySelectedOutput = nil
        historySelectedDecisionId = nil
        historyDetailLoading = false
        historyDetailError = nil
        activeHistoryPageRequestId = nil
        historyLoadingOlder = false
        activeHistoryDetailRequestId = nil
        refreshHistoryLayout()
        updateContent()
        if origin == .ambientMemory, !ambientHovered {
            scheduleAmbientHoverReturn()
        }
    }

    private func showHistoryList() {
        guard presentation == .historyDetail else { return }
        historySelectedOutput = nil
        historySelectedDecisionId = nil
        historyDetailLoading = false
        historyDetailError = nil
        activeHistoryDetailRequestId = nil
        setPresentation(.historyList)
    }

    private func loadOlderHistory() {
        guard presentation == .historyList,
              !historyLoadingOlder,
              let cursor = historyNextCursor else { return }
        historyLoadingOlder = true
        historyError = nil
        let requestId = nextHistoryRequestId()
        activeHistoryPageRequestId = requestId
        updateContent()
        if !sendAction(
            "load_older_continue_history",
            historyRequestId: requestId,
            historyCursor: cursor
        ) {
            activeHistoryPageRequestId = nil
            historyLoadingOlder = false
            historyError = "Smalltalk could not load older answers."
            updateContent()
        }
    }

    private func retryHistory() {
        guard presentation == .historyList || presentation == .historyLoading else { return }
        historyItems = []
        historyNextCursor = nil
        historyError = nil
        historyLoadingOlder = false
        let requestId = nextHistoryRequestId()
        activeHistoryPageRequestId = requestId
        setPresentation(.historyLoading)
        if !sendAction(
            "retry_continue_history",
            historyRequestId: requestId
        ) {
            activeHistoryPageRequestId = nil
            historyError = "Smalltalk could not retry saved answers."
            setPresentation(.historyList)
        }
    }

    private func selectHistoryOutput(_ decisionId: String) {
        let cleanDecisionId = decisionId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard presentation == .historyList, !cleanDecisionId.isEmpty else { return }
        historySelectedDecisionId = cleanDecisionId
        historySelectedOutput = nil
        historyDetailError = nil
        historyDetailLoading = true
        let requestId = nextHistoryRequestId()
        activeHistoryDetailRequestId = requestId
        setPresentation(.historyDetail)
        if !sendAction(
            "select_continue_history_output",
            decisionId: cleanDecisionId,
            historyRequestId: requestId
        ) {
            activeHistoryDetailRequestId = nil
            historyDetailLoading = false
            historyDetailError = "Smalltalk could not load this saved answer."
            updateContent()
        }
    }

    private func retryHistoryDetail() {
        guard presentation == .historyDetail,
              let decisionId = historySelectedDecisionId else { return }
        historySelectedOutput = nil
        historyDetailError = nil
        historyDetailLoading = true
        let requestId = nextHistoryRequestId()
        activeHistoryDetailRequestId = requestId
        updateContent()
        if !sendAction(
            "select_continue_history_output",
            decisionId: decisionId,
            historyRequestId: requestId
        ) {
            activeHistoryDetailRequestId = nil
            historyDetailLoading = false
            historyDetailError = "Smalltalk could not retry this saved answer."
            updateContent()
        }
    }

    private func nextHistoryRequestId() -> UInt64 {
        historyRequestId &+= 1
        return historyRequestId
    }

    private func processHistoryResponses(from snapshot: IslandSnapshot) {
        if let page = snapshot.continueHistoryPage,
           page.requestId == activeHistoryPageRequestId {
            activeHistoryPageRequestId = nil
            let wasLoadingOlder = historyLoadingOlder
            historyLoadingOlder = false

            if let error = page.error?.nonEmpty {
                historyError = error
            } else {
                historyError = nil
                if wasLoadingOlder {
                    var knownDecisionIds = Set(historyItems.map(\.decisionId))
                    for item in page.items where knownDecisionIds.insert(item.decisionId).inserted {
                        historyItems.append(item)
                    }
                } else {
                    historyItems = page.items
                }
                historyNextCursor = page.nextCursor
            }

            if presentation == .historyLoading {
                setPresentation(.historyList)
            } else {
                updateContent()
            }
        }

        if let output = snapshot.continueHistoryOutput,
           output.requestId == activeHistoryDetailRequestId {
            activeHistoryDetailRequestId = nil
            historyDetailLoading = false
            if let error = output.error?.nonEmpty {
                historySelectedOutput = nil
                historyDetailError = error
            } else if output.decisionId == historySelectedDecisionId {
                historySelectedOutput = output
                historyDetailError = nil
            }
            updateContent()
        }
    }

    private func clearHistoryState() {
        historyOriginPresentation = .ambientMemory
        historyItems = []
        historyNextCursor = nil
        historyError = nil
        historyLoadingOlder = false
        historySelectedOutput = nil
        historySelectedDecisionId = nil
        historyDetailLoading = false
        historyDetailError = nil
        activeHistoryPageRequestId = nil
        activeHistoryDetailRequestId = nil
        historyAnchor = nil
        historyLayout = WhisperFlowHistoryLayout()
    }

    private func dismissOnePresentationLevel() {
        if presentation == .historyDetail {
            showHistoryList()
        } else if presentation == .historyList || presentation == .historyLoading {
            closeHistory()
        } else if presentation == .answerExpanded {
            showAnswerSummary()
        } else if presentation == .answerSummary {
            returnToDefaultPresentation()
        }
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
        if presentation.isHistory {
            closeHistory()
        }
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

    private func markPanelDragEnded() {
        if presentation.isHistory, let panel {
            historyAnchor = NSPoint(
                x: panel.frame.midX
                    + historyLayout.capsuleOffsetX * gOverlayScale,
                y: panel.frame.maxY
            )
        }
        positionPanel(preserveCurrentAnchor: true, animated: false)
    }

    private func shouldBeginWindowDrag(at point: NSPoint) -> Bool {
        if historyControlRect.contains(point) {
            return false
        }
        if presentation.isHistory {
            let capsuleCenterX = targetPanelSize.width / 2
                + historyLayout.capsuleOffsetX * gOverlayScale
            let capsuleWidth = historyCapsuleWidth * gOverlayScale
            let capsuleHeight = historyCapsuleHeight * gOverlayScale
            let capsuleRect = NSRect(
                x: capsuleCenterX - capsuleWidth / 2,
                y: targetPanelSize.height - capsuleHeight,
                width: capsuleWidth,
                height: capsuleHeight
            )
            return capsuleRect.contains(point)
        }
        return true
    }

    private var historyControlRect: NSRect {
        guard historyButtonShouldBeVisible else { return .zero }
        let direction: CGFloat = historyLayout.controlOnLeft ? -1 : 1
        let controlCenterX = targetPanelSize.width / 2
            + historyLayout.capsuleOffsetX * gOverlayScale
            + direction * (
                historyCapsuleWidth / 2
                    + kWhisperFlowHistoryButtonGap
                    + kWhisperFlowHistoryButtonVisualSize / 2
            ) * gOverlayScale
        let controlTop = max(
            0,
            (historyCapsuleHeight - kWhisperFlowHistoryButtonHitSize) / 2
        ) * gOverlayScale
        let controlCenterY = targetPanelSize.height
            - controlTop
            - kWhisperFlowHistoryButtonHitSize * gOverlayScale / 2
        let radius = kWhisperFlowHistoryButtonHitSize * gOverlayScale / 2
        return NSRect(
            x: controlCenterX - radius,
            y: controlCenterY - radius,
            width: radius * 2,
            height: radius * 2
        )
    }

    private var historyCapsuleHeight: CGFloat {
        let target = presentation.isHistory ? historyOriginPresentation : presentation
        switch target {
        case .answerSummary:
            return kWhisperFlowAnswerSummaryH
        case .ambientMemory:
            return memoryTransitionCountdownActive || continueRequestInFlight
                ? kWhisperFlowNotificationH
                : kWhisperFlowCaptureH
        default:
            return 0
        }
    }

    private func cancelPresentationTimers() {
        cancelAmbientHoverReturn()
        cancelMemoryTransitionCountdown()
    }

    private var targetPanelSize: NSSize {
        let base = basePanelSize(
            for: presentation.isHistory ? historyOriginPresentation : presentation
        )
        if presentation.isHistory {
            return NSSize(
                width: historyLayout.canvasWidth * gOverlayScale,
                height: historyLayout.canvasHeight * gOverlayScale
            )
        }
        if historyButtonShouldBeVisible {
            return NSSize(
                width: base.width
                    + kWhisperFlowHistoryAccessoryAllowance * 2 * gOverlayScale,
                height: base.height
            )
        }
        return base
    }

    private func basePanelSize(for targetPresentation: WhisperFlowPresentation) -> NSSize {
        switch targetPresentation {
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
        case .historyLoading, .historyList, .historyDetail:
            return basePanelSize(for: historyOriginPresentation)
        }
    }

    private var historyButtonShouldBeVisible: Bool {
        if presentation.isHistory || presentation == .answerSummary {
            return true
        }
        return presentation == .ambientMemory
            && !continueRequestInFlight
            && !memoryTransitionCountdownActive
            && snapshot.state != "starting"
            && snapshot.state != "processing"
    }

    private func refreshHistoryLayout() {
        let screen = historyAnchor.flatMap(screenContaining)
            ?? panel?.screen
            ?? screenContaining(NSEvent.mouseLocation)
            ?? NSScreen.main
            ?? NSScreen.screens.first
        let visibleSize = screen?.visibleFrame.size ?? NSSize(width: 1280, height: 800)
        let usableWidth = max(1, visibleSize.width / gOverlayScale)
        let usableHeight = max(1, visibleSize.height / gOverlayScale)
        let availableHeightBelowAnchor = historyAnchor.map { anchor in
            max(1, (anchor.y - (screen?.visibleFrame.minY ?? 0)) / gOverlayScale)
        } ?? usableHeight
        let basePresentation = presentation.isHistory
            ? historyOriginPresentation
            : presentation
        let base = basePanelSize(for: basePresentation)
        let baseWidth = base.width / gOverlayScale
        let baseHeight = base.height / gOverlayScale

        if presentation.isHistory {
            let cardWidth = min(
                kWhisperFlowHistoryCardMaxW,
                max(
                    kWhisperFlowHistoryCardMinW,
                    min(kWhisperFlowHistoryCardPreferredW, usableWidth - 24)
                )
            )
            let cardHeight = max(
                1,
                min(
                    kWhisperFlowHistoryCardPreferredH,
                    usableHeight * 0.60,
                    availableHeightBelowAnchor
                        - historyCapsuleHeight
                        - kWhisperFlowHistoryCardGap
                        - 12
                )
            )
            historyLayout.cardWidth = cardWidth
            historyLayout.cardHeight = cardHeight
            historyLayout.canvasWidth = min(
                usableWidth,
                max(
                    cardWidth,
                    baseWidth + kWhisperFlowHistoryAccessoryAllowance * 2
                )
            )
            historyLayout.canvasHeight = historyCapsuleHeight
                + kWhisperFlowHistoryCardGap
                + cardHeight
        } else {
            historyLayout.cardWidth = kWhisperFlowHistoryCardPreferredW
            historyLayout.cardHeight = kWhisperFlowHistoryCardPreferredH
            historyLayout.canvasWidth = baseWidth
                + (historyButtonShouldBeVisible
                    ? kWhisperFlowHistoryAccessoryAllowance * 2
                    : 0)
            historyLayout.canvasHeight = baseHeight
            historyLayout.capsuleOffsetX = 0
        }
    }

    private func updateHistoryPlacement(for frame: NSRect) {
        let anchor = historyAnchor
            ?? NSPoint(x: frame.midX, y: frame.maxY)
        historyLayout.capsuleOffsetX = presentation.isHistory
            ? (anchor.x - frame.midX) / gOverlayScale
            : 0

        let screen = screenContaining(anchor)
            ?? panel?.screen
            ?? NSScreen.main
            ?? NSScreen.screens.first
        let visibleFrame = screen?.visibleFrame ?? frame
        let capsuleWidth = historyCapsuleWidth
        let leftEdge = anchor.x
            - (capsuleWidth / 2
                + kWhisperFlowHistoryButtonGap
                + kWhisperFlowHistoryButtonVisualSize / 2
                + kWhisperFlowHistoryButtonHitSize / 2) * gOverlayScale
        historyLayout.controlOnLeft = leftEdge >= visibleFrame.minX + 4
    }

    private var historyCapsuleWidth: CGFloat {
        let target = presentation.isHistory ? historyOriginPresentation : presentation
        switch target {
        case .answerSummary:
            return answerLayout.summaryWidth
        case .ambientMemory:
            return memoryTransitionCountdownActive || continueRequestInFlight
                ? kWhisperFlowNotificationW
                : kWhisperFlowCaptureW
        default:
            return 0
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
        refreshHistoryLayout()
        let frame = resolvedPanelFrame(preserveCurrentAnchor: preserveCurrentAnchor)
        updateHistoryPlacement(for: frame)
        setPanelFrame(frame, animated: animated)
        updateContent()
    }

    private func resolvedPanelFrame(preserveCurrentAnchor: Bool) -> NSRect {
        let size = targetPanelSize
        let currentFrame = panel?.frame ?? .zero
        let anchor: NSPoint
        if let historyAnchor {
            anchor = historyAnchor
        } else if preserveCurrentAnchor && currentFrame.width > 0 && currentFrame.height > 0 {
            anchor = NSPoint(x: currentFrame.midX, y: currentFrame.maxY)
        } else {
            anchor = initialTopCenterAnchor(for: size)
        }
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
        panel.isMovableByWindowBackground = !presentation.isHistory
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
        if islandModel.blockedContinueShakeNonce != blockedContinueShakeNonce {
            islandModel.blockedContinueShakeNonce = blockedContinueShakeNonce
        }
        if islandModel.answer != latchedAnswer {
            islandModel.answer = latchedAnswer
        }
        if islandModel.answerLayout != answerLayout {
            islandModel.answerLayout = answerLayout
        }
        if islandModel.visualCueImage !== visualCueImage {
            islandModel.visualCueImage = visualCueImage
        }
        if islandModel.visualCuePresented != visualCuePresented {
            islandModel.visualCuePresented = visualCuePresented
        }
        if islandModel.historyOriginPresentation != historyOriginPresentation {
            islandModel.historyOriginPresentation = historyOriginPresentation
        }
        if islandModel.historyButtonVisible != historyButtonShouldBeVisible {
            islandModel.historyButtonVisible = historyButtonShouldBeVisible
        }
        if islandModel.historyItems != historyItems {
            islandModel.historyItems = historyItems
        }
        if islandModel.historyNextCursor != historyNextCursor {
            islandModel.historyNextCursor = historyNextCursor
        }
        if islandModel.historyError != historyError {
            islandModel.historyError = historyError
        }
        if islandModel.historyLoadingOlder != historyLoadingOlder {
            islandModel.historyLoadingOlder = historyLoadingOlder
        }
        if islandModel.historySelectedOutput != historySelectedOutput {
            islandModel.historySelectedOutput = historySelectedOutput
        }
        if islandModel.historySelectedDecisionId != historySelectedDecisionId {
            islandModel.historySelectedDecisionId = historySelectedDecisionId
        }
        if islandModel.historyDetailLoading != historyDetailLoading {
            islandModel.historyDetailLoading = historyDetailLoading
        }
        if islandModel.historyDetailError != historyDetailError {
            islandModel.historyDetailError = historyDetailError
        }
        let currentDecisionId = latchedDecisionId
            ?? snapshot.islandContinueState?.decisionId
            ?? snapshot.continueDecisionId
        if islandModel.currentDecisionId != currentDecisionId {
            islandModel.currentDecisionId = currentDecisionId
        }
        if islandModel.historyLayout != historyLayout {
            islandModel.historyLayout = historyLayout
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
            },
            onToggleVisualCue: { [weak self] in
                self?.toggleVisualCue()
            },
            onToggleHistory: { [weak self] in
                self?.toggleHistory()
            },
            onHistoryButtonHover: { [weak self] hovering in
                self?.ambientHoverChanged(hovering)
            },
            onLoadOlderHistory: { [weak self] in
                self?.loadOlderHistory()
            },
            onRetryHistory: { [weak self] in
                self?.retryHistory()
            },
            onRetryHistoryDetail: { [weak self] in
                self?.retryHistoryDetail()
            },
            onSelectHistoryOutput: { [weak self] decisionId in
                self?.selectHistoryOutput(decisionId)
            },
            onBackFromHistoryDetail: { [weak self] in
                self?.showHistoryList()
            },
            onDismissOneLevel: { [weak self] in
                self?.dismissOnePresentationLevel()
            }
        )

        let hosting = DraggableHostingView(rootView: AnyView(view))
        hosting.onDragBegan = { [weak self] in
            self?.markPanelDragBegan()
        }
        hosting.onDragEnded = { [weak self] in
            self?.markPanelDragEnded()
        }
        hosting.shouldBeginWindowDrag = { [weak self] point in
            self?.shouldBeginWindowDrag(at: point) ?? false
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
        if visible && (
            presentation == .answerSummary
                || presentation == .answerExpanded
                || presentation.isHistory
        ) {
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
        guard presentation == .answerSummary
                || presentation == .answerExpanded
                || presentation.isHistory,
              let panel else { return }
        if event.window === panel {
            return
        }
        if NSMouseInRect(NSEvent.mouseLocation, panel.frame, false) {
            return
        }

        DispatchQueue.main.async { [weak self] in
            self?.dismissOnePresentationLevel()
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
        taskHypothesisId: String? = nil,
        historyRequestId: UInt64? = nil,
        historyCursor: ContinueHistoryCursorV1? = nil
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
        if let historyRequestId {
            fields.append("\"history_request_id\":\(historyRequestId)")
        }
        if let historyCursor {
            fields.append(
                "\"history_cursor\":{\"created_at_ms\":\(historyCursor.createdAtMs),"
                    + "\"decision_id\":\"\(jsonEscaped(historyCursor.decisionId))\"}"
            )
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
    var onDragEnded: (() -> Void)?
    var shouldBeginWindowDrag: ((NSPoint) -> Bool)?
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
        guard shouldBeginWindowDrag?(event.locationInWindow) ?? true else { return }

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
                    self.onDragEnded?()
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
