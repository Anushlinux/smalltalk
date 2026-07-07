import AppKit
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
        case privacyLabel = "privacy_label"
        case isSensitive = "is_sensitive"
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

    static func swiftUIMonoFont(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        let name: String
        switch weight {
        case .medium:
            name = "IBMPlexMono-Medium"
        case .semibold, .bold:
            name = "IBMPlexMono-SemiBold"
        default:
            name = "IBMPlexMono"
        }
        if NSFont(name: name, size: size) != nil {
            return Font.custom(name, fixedSize: size)
        }
        return Font.system(size: size, weight: weight, design: .monospaced)
    }
}

private enum IslandMotion {
    static func quick(_ reduceMotion: Bool) -> Animation {
        .timingCurve(0.25, 1, 0.5, 1, duration: reduceMotion ? 0.01 : 0.14)
    }

    static func settle(_ reduceMotion: Bool) -> Animation {
        .timingCurve(0.20, 0.88, 0.20, 1, duration: reduceMotion ? 0.01 : 0.26)
    }

    static func reveal(_ reduceMotion: Bool) -> Animation {
        .timingCurve(0.18, 0.92, 0.18, 1, duration: reduceMotion ? 0.01 : 0.34)
    }

    static func panelTimingFunction() -> CAMediaTimingFunction {
        CAMediaTimingFunction(controlPoints: 0.18, 0.92, 0.18, 1.0)
    }

    static func microTransition(_ reduceMotion: Bool) -> AnyTransition {
        guard !reduceMotion else { return .opacity }
        return .asymmetric(
            insertion: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: 0.74, blur: 6, y: -3),
                identity: IslandMorphModifier(opacity: 1, scale: 1, blur: 0, y: 0)
            ),
            removal: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: 0.82, blur: 7, y: 2),
                identity: IslandMorphModifier(opacity: 1, scale: 1, blur: 0, y: 0)
            )
        )
    }

    static func compactTransition(_ reduceMotion: Bool) -> AnyTransition {
        guard !reduceMotion else { return .opacity }
        return .asymmetric(
            insertion: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: 0.72, blur: 8, y: 4),
                identity: IslandMorphModifier(opacity: 1, scale: 1, blur: 0, y: 0)
            ),
            removal: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: 0.92, blur: 6, y: -2),
                identity: IslandMorphModifier(opacity: 1, scale: 1, blur: 0, y: 0)
            )
        )
    }

    static func expandedTransition(_ reduceMotion: Bool) -> AnyTransition {
        guard !reduceMotion else { return .opacity }
        return .asymmetric(
            insertion: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: 0.88, blur: 8, y: -5),
                identity: IslandMorphModifier(opacity: 1, scale: 1, blur: 0, y: 0)
            ),
            removal: .modifier(
                active: IslandMorphModifier(opacity: 0, scale: 0.96, blur: 5, y: 3),
                identity: IslandMorphModifier(opacity: 1, scale: 1, blur: 0, y: 0)
            )
        )
    }
}

@available(macOS 13.0, *)
private struct IslandMorphModifier: ViewModifier {
    let opacity: Double
    let scale: CGFloat
    let blur: CGFloat
    let y: CGFloat

    func body(content: Content) -> some View {
        content
            .opacity(opacity)
            .scaleEffect(scale, anchor: .top)
            .blur(radius: blur)
            .offset(y: y)
    }
}

@available(macOS 13.0, *)
private final class AnimationTick: ObservableObject {
    static let shared = AnimationTick()
    @Published var value = 0.0
    private var timer: Timer?

    func start() {
        guard timer == nil else { return }
        timer = Timer.scheduledTimer(withTimeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            guard let self else { return }
            value += 1.0 / 60.0
            objectWillChange.send()
        }
        RunLoop.main.add(timer!, forMode: .common)
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }
}

@available(macOS 13.0, *)
private struct MemorySignalView: View {
    let active: Bool
    let processing: Bool
    let error: Bool
    let privateMode: Bool
    let thinEvidence: Bool
    let ready: Bool
    let frameCount: Int64
    let density: Double
    let pulseNonce: UInt64?
    let scale: CGFloat
    let width: CGFloat
    let height: CGFloat
    let cornerRadius: CGFloat
    let compact: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ObservedObject private var anim = AnimationTick.shared
    @State private var lastPulseNonce: UInt64?
    @State private var pulseStartedAt: Double?

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: cornerRadius * scale, style: .continuous)
        let pulseAge = pulseStartedAt.map { max(0, anim.value - $0) } ?? 999
        let pulseProgress = reduceMotion ? 1 : min(1, pulseAge / 0.46)
        let pulseStrength = max(0, 1 - pulseProgress)
        let motionActive = active
        let heat = motionActive && !reduceMotion ? 0.5 + 0.5 * sin(anim.value * 4.6) : 0.0

        Canvas { context, size in
            drawMemorySignal(context: &context, size: size, pulseStrength: pulseStrength, heat: heat)
        }
        .frame(width: width * scale, height: height * scale)
        .background(
            shape.fill(
                LinearGradient(
                    colors: [
                        Color(red: 0.018, green: 0.020, blue: 0.024).opacity(0.96),
                        signalColor.opacity(motionActive ? 0.16 : 0.04),
                        Color(red: 0.042, green: 0.047, blue: 0.052).opacity(0.95),
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
            )
        )
        .overlay(
            shape.stroke(
                signalColor.opacity(motionActive || processing || error || thinEvidence || privateMode ? 0.30 + 0.12 * heat : 0.12),
                lineWidth: 0.7
            )
        )
        .overlay(alignment: .top) {
            shape
                .stroke(Color.white.opacity(motionActive ? 0.09 : 0.055), lineWidth: 1)
                .blur(radius: 0.25)
                .offset(y: -0.5 * scale)
        }
        .overlay(
            shape.fill(
                signalColor.opacity(pulseStrength * (compact ? 0.16 : 0.12))
            )
        )
        .shadow(
            color: signalColor.opacity(motionActive || pulseStrength > 0 ? 0.10 + 0.06 * heat : 0),
            radius: 9 * scale,
            x: 0,
            y: 0
        )
        .clipShape(shape)
        .drawingGroup()
        .onAppear {
            lastPulseNonce = pulseNonce
        }
        .onChange(of: pulseNonce) { next in
            guard next != nil, next != lastPulseNonce else {
                lastPulseNonce = next
                return
            }
            lastPulseNonce = next
            pulseStartedAt = anim.value
        }
        .animation(IslandMotion.settle(reduceMotion), value: active)
        .animation(IslandMotion.settle(reduceMotion), value: processing)
        .animation(IslandMotion.settle(reduceMotion), value: error)
        .animation(IslandMotion.settle(reduceMotion), value: privateMode)
        .animation(IslandMotion.settle(reduceMotion), value: thinEvidence)
        .animation(IslandMotion.settle(reduceMotion), value: ready)
        .animation(IslandMotion.settle(reduceMotion), value: frameCount)
    }

    private var signalColor: Color {
        if error {
            return Color(red: 1.0, green: 0.36, blue: 0.28)
        }
        if thinEvidence || processing {
            return Color(red: 1.0, green: 0.68, blue: 0.28)
        }
        if privateMode {
            return Color.white.opacity(0.62)
        }
        if active || ready {
            return Color(red: 0.36, green: 0.84, blue: 0.68)
        }
        return Color.white.opacity(0.42)
    }

    private func drawMemorySignal(
        context: inout GraphicsContext,
        size: CGSize,
        pulseStrength: Double,
        heat: Double
    ) {
        let tick = anim.value
        let signalRect = CGRect(origin: .zero, size: size)
        let motionActive = active
        let tone = signalColor
        let alert = error ? Color(red: 1.0, green: 0.62, blue: 0.24) : tone
        let centerY = round(size.height * 0.54)
        let startX = size.width * 0.16
        let endX = size.width * 0.84
        let span = max(1, endX - startX)

        context.fill(Path(signalRect), with: .color(.black.opacity(0.16)))

        var signalLine = Path()
        signalLine.move(to: CGPoint(x: startX, y: centerY))
        signalLine.addLine(to: CGPoint(x: endX, y: centerY))
        context.stroke(
            signalLine,
            with: .color(tone.opacity(motionActive || ready ? 0.42 : 0.22)),
            lineWidth: max(1, 1.1 * scale)
        )

        let maxAnchors = compact ? 4 : 6
        let anchorCountFromEvidence: Int
        if frameCount > 0 {
            anchorCountFromEvidence = Int(min(frameCount, Int64(maxAnchors)))
        } else {
            anchorCountFromEvidence = min(maxAnchors, max(2, Int(round(density))))
        }
        let anchorCount = max(2, anchorCountFromEvidence)

        for index in 0..<anchorCount {
            let fraction = anchorCount == 1 ? 0.5 : Double(index) / Double(anchorCount - 1)
            let x = startX + span * CGFloat(fraction)
            let y = centerY + CGFloat(sin((Double(index) + 1.0) * 1.7) * Double(size.height) * 0.055)
            let newest = index == anchorCount - 1
            let anchorHeat = newest && motionActive && !reduceMotion ? heat : 0
            let radius = max(2.2 * scale, (newest ? 3.5 : 2.5) * scale + CGFloat(anchorHeat) * 0.8 * scale)
            let anchorRect = CGRect(x: x - radius, y: y - radius, width: radius * 2, height: radius * 2)
            let opacity = newest ? 0.78 + 0.14 * anchorHeat : 0.34 + 0.04 * Double(index % 2)

            context.fill(Path(ellipseIn: anchorRect), with: .color(tone.opacity(opacity)))
            context.stroke(
                Path(ellipseIn: anchorRect.insetBy(dx: -2.2 * scale, dy: -2.2 * scale)),
                with: .color(tone.opacity(newest && (ready || motionActive) ? 0.25 + 0.10 * anchorHeat : 0.08)),
                lineWidth: max(0.7, 0.8 * scale)
            )
        }

        if pulseStrength > 0 {
            let bloomRadius = max(size.height * 0.56, 8 * scale) * CGFloat(1.0 + (1.0 - pulseStrength) * 0.35)
            let bloomRect = CGRect(
                x: endX - bloomRadius,
                y: centerY - bloomRadius,
                width: bloomRadius * 2,
                height: bloomRadius * 2
            )
            context.stroke(
                Path(ellipseIn: bloomRect),
                with: .color(tone.opacity(0.30 * pulseStrength)),
                lineWidth: max(1, 1.2 * scale)
            )
            context.fill(
                Path(ellipseIn: bloomRect.insetBy(dx: bloomRadius * 0.34, dy: bloomRadius * 0.34)),
                with: .color(tone.opacity(0.10 * pulseStrength))
            )
        }

        if processing {
            let shimmerPhase = reduceMotion ? 0.72 : fmod(tick * 0.82, 1.0)
            let shimmerRect = CGRect(
                x: shimmerPhase * (size.width + size.width * 0.32) - size.width * 0.28,
                y: 0,
                width: size.width * 0.28,
                height: size.height
            )
            let shimmer = GraphicsContext.Shading.linearGradient(
                Gradient(stops: [
                    .init(color: .white.opacity(0), location: 0),
                    .init(color: .white.opacity(0.18), location: 0.52),
                    .init(color: .white.opacity(0), location: 1),
                ]),
                startPoint: CGPoint(x: shimmerRect.minX, y: shimmerRect.midY),
                endPoint: CGPoint(x: shimmerRect.maxX, y: shimmerRect.midY)
            )
            context.fill(Path(shimmerRect), with: shimmer)
        }

        if error || privateMode {
            var slash = Path()
            slash.move(to: CGPoint(x: size.width * 0.22, y: size.height * 0.72))
            slash.addLine(to: CGPoint(x: size.width * 0.78, y: size.height * 0.28))
            context.stroke(
                slash,
                with: .color(alert.opacity(0.62)),
                lineWidth: max(1, 1.3 * scale)
            )
        }
    }
}

private let kBaseCollapsedW: CGFloat = 222
private let kBaseCollapsedH: CGFloat = 48
private let kBaseMicroHitW: CGFloat = 86
private let kBaseMicroHitH: CGFloat = 24
private let kBaseMicroVisualW: CGFloat = 58
private let kBaseMicroVisualH: CGFloat = 10
private let kBaseExpandedW: CGFloat = 520
private let kBaseExpandedH: CGFloat = 268
private let kAnimDur = 0.2
private let kIdleMicroDelay: TimeInterval = 5.0
private let kPanelFrameAnimDur = 0.32

private enum IslandPresentation: Equatable {
    case micro
    case compact
    case expanded
}

private enum IslandProductState: Equatable {
    case memoryOff
    case memoryStarting
    case memoryOn
    case newEvidence
    case continueUpdating
    case continueReady
    case thinEvidence
    case needsEvidence
    case pausedWithEvidence
    case privateOrExcluded
    case needsAttention
}

@available(macOS 13.0, *)
private struct SessionIslandView: View {
    let snapshot: IslandSnapshot
    let metrics: OverlayMetrics
    let scale: CGFloat
    let onAction: (String) -> Void
    @Binding var presentation: IslandPresentation
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @ObservedObject private var anim = AnimationTick.shared
    @State private var signalHovered = false

    private func s(_ value: CGFloat) -> CGFloat { value * scale }

    private var productState: IslandProductState {
        if hasError {
            return .needsAttention
        }
        if isPrivateOrExcluded {
            return .privateOrExcluded
        }
        switch normalizedContinueFreshness {
        case "new_evidence":
            return .newEvidence
        case "updating":
            return .continueUpdating
        case "current", "ready_to_continue":
            return .continueReady
        case "thin_evidence":
            return .thinEvidence
        case "needs_evidence", "needs_attention":
            return .needsEvidence
        default:
            break
        }
        switch snapshot.state {
        case "starting":
            return .memoryStarting
        case "recording_compact", "recording_expanded":
            return .memoryOn
        case "processing":
            return .memoryOn
        case "trail_reconstructing":
            return .continueUpdating
        case "resume_ready":
            return resumeReadyIsThin ? .thinEvidence : .continueReady
        default:
            return hasMoments ? .pausedWithEvidence : .memoryOff
        }
    }

    private var captureActive: Bool {
        productState == .memoryStarting ||
            productState == .memoryOn ||
            productState == .newEvidence
    }

    private var signalReady: Bool {
        productState == .continueReady
    }

    private var signalThin: Bool {
        productState == .thinEvidence || productState == .needsEvidence
    }

    private var signalPrivate: Bool {
        productState == .privateOrExcluded
    }

    private var isBusy: Bool {
        snapshot.state == "starting" || snapshot.state == "processing" || snapshot.state == "trail_reconstructing"
    }

    private var isProcessing: Bool {
        snapshot.state == "starting" || snapshot.state == "processing" || snapshot.state == "trail_reconstructing"
    }

    private var hasError: Bool {
        snapshot.lastError != nil || snapshot.state == "error"
    }

    private var isPrivateOrExcluded: Bool {
        snapshot.isSensitive || isPrivatePrivacyLabel(snapshot.privacyLabel)
    }

    private var primaryDisplayText: String {
        switch productState {
        case .needsAttention:
            return "Needs attention"
        case .privateOrExcluded:
            return "Not observing this app"
        case .memoryStarting:
            return "Starting memory"
        case .memoryOn:
            return snapshot.state == "processing" ? "Pausing memory" : "Memory on"
        case .newEvidence:
            return "New evidence"
        case .continueUpdating:
            return "Finding continuation"
        case .continueReady:
            return resumeReadyTitle
        case .thinEvidence:
            return "Evidence is thin"
        case .needsEvidence:
            return "Needs evidence"
        case .pausedWithEvidence:
            return "Continue ready"
        case .memoryOff:
            return "Memory off"
        }
    }

    private var secondaryDisplayText: String {
        switch productState {
        case .needsAttention:
            return productizedErrorLine
        case .privateOrExcluded:
            return "Privacy boundary active"
        case .memoryStarting:
            return "Preparing local memory"
        case .memoryOn:
            return snapshot.state == "processing" ? "Keeping the last answer available" : "Ask Smalltalk where to continue"
        case .newEvidence:
            return "Refresh Continue"
        case .continueUpdating:
            return "Looking for your return point"
        case .continueReady, .thinEvidence, .needsEvidence:
            return resumePointLine
        case .pausedWithEvidence:
            return "Ask Continue from stored evidence"
        case .memoryOff:
            return "Turn on local memory"
        }
    }

    private var primaryActionLabel: String {
        switch productState {
        case .memoryOff:
            return "Turn on local memory"
        case .memoryStarting:
            return "Starting"
        case .memoryOn:
            return snapshot.state == "processing" ? "Pausing" : "Find continuation"
        case .newEvidence:
            return "Refresh Continue"
        case .continueUpdating:
            return "Finding"
        case .continueReady:
            return "Continue here"
        case .thinEvidence:
            return "Why this?"
        case .needsEvidence:
            return "Why this?"
        case .pausedWithEvidence:
            return "Continue"
        case .privateOrExcluded:
            return "Privacy"
        case .needsAttention:
            return "Fix in Smalltalk"
        }
    }

    private var compactActionLabel: String {
        switch productState {
        case .continueReady:
            return "Open"
        case .thinEvidence:
            return "Why"
        case .memoryOff:
            return "On"
        case .memoryStarting:
            return "..."
        case .memoryOn, .pausedWithEvidence:
            return "Ask"
        case .newEvidence:
            return "Refresh"
        case .continueUpdating:
            return "..."
        case .needsEvidence:
            return "Why"
        case .privateOrExcluded:
            return "Privacy"
        case .needsAttention:
            return "Fix"
        }
    }

    private var secondaryActionLabel: String {
        switch productState {
        case .memoryOn:
            return snapshot.state == "processing" ? "Open Smalltalk" : "Pause memory"
        case .continueReady:
            return "Why this?"
        case .thinEvidence, .needsEvidence:
            return "Open Smalltalk"
        case .newEvidence:
            return "Open Smalltalk"
        case .pausedWithEvidence, .memoryOff, .privateOrExcluded, .needsAttention:
            return "Open Smalltalk"
        case .memoryStarting, .continueUpdating:
            return "Open Smalltalk"
        }
    }

    private var primaryActionDisabled: Bool {
        productState == .memoryStarting ||
            productState == .continueUpdating ||
            (productState == .memoryOn && snapshot.state == "processing")
    }

    private var secondaryActionDisabled: Bool {
        productState == .memoryStarting || productState == .continueUpdating
    }

    private var primaryButtonAction: String {
        switch productState {
        case .memoryOff:
            return "start_memory"
        case .memoryOn, .pausedWithEvidence, .newEvidence:
            return "continue"
        case .continueReady:
            return "open_resume_point"
        case .thinEvidence, .needsEvidence:
            return "show_trail"
        case .privateOrExcluded, .needsAttention:
            return "open_main_window"
        case .memoryStarting, .continueUpdating:
            return "open_main_window"
        }
    }

    private var secondaryButtonAction: String {
        switch productState {
        case .memoryOn:
            return snapshot.state == "processing" ? "open_main_window" : "pause_memory"
        case .continueReady:
            return "show_trail"
        case .thinEvidence, .needsEvidence:
            return "open_main_window"
        case .newEvidence:
            return "open_main_window"
        default:
            return "open_main_window"
        }
    }

    private var normalizedContinueFreshness: String {
        trimmed(snapshot.continueFreshness).lowercased()
    }

    private var hasMoments: Bool {
        trailMomentCount > 0
    }

    private var trailMomentCount: Int64 {
        max(0, snapshot.trailMomentCount ?? snapshot.frameCount)
    }

    private var trailAppCount: Int64 {
        max(0, snapshot.trailAppCount ?? Int64(snapshot.trailLabels?.count ?? 0))
    }

    private var signalEvidenceCount: Int64 {
        switch productState {
        case .newEvidence, .continueUpdating, .continueReady, .thinEvidence, .needsEvidence:
            return trailMomentCount
        default:
            return 0
        }
    }

    private var momentLabel: String {
        "\(trailMomentCount) \(trailMomentCount == 1 ? "signal" : "signals")"
    }

    private var resumePointLine: String {
        let point = trimmed(snapshot.resumePoint)
        guard !point.isEmpty else {
            return resumeReadyIsThin ? "No reliable return target yet" : "Continue target ready"
        }
        return "Continue at \(point)"
    }

    private var resumeDetailEyebrow: String {
        resumeReadyIsThin ? "Best available answer" : "Continue at"
    }

    private var resumeReadyTitle: String {
        if trimmed(snapshot.resumeSource) == "continue" {
            if resumeReadyIsThin {
                return "Evidence is thin"
            }
            let headline = trimmed(snapshot.resumeHeadline)
            return headline.isEmpty ? "Continue ready" : headline
        }
        if trimmed(snapshot.resumeSource) == "cloud" {
            return "Continue ready"
        }
        let warning = trimmed(snapshot.resumeWarning).lowercased()
        if warning.contains("openai_api_key") || warning.contains("key") {
            return "OpenAI key missing"
        }
        return "OpenAI unavailable"
    }

    private var resumeReadyIsThin: Bool {
        if normalizedContinueFreshness == "thin_evidence" || normalizedContinueFreshness == "needs_evidence" {
            return true
        }
        let warning = trimmed(snapshot.resumeWarning).lowercased()
        return warning.contains("thin") || warning.contains("missing") || warning.contains("no_")
    }

    private var resumeDetailLine: String {
        let detail = trimmed(snapshot.resumeDetail)
        if !detail.isEmpty {
            return detail
        }
        let headline = trimmed(snapshot.resumeHeadline)
        return headline.isEmpty ? "Smalltalk found a local continuation." : headline
    }

    private var resumeProvenanceLine: String {
        if trimmed(snapshot.resumeSource) == "continue" {
            let warning = trimmed(snapshot.resumeWarning)
            return warning.isEmpty ? "Evidence-backed Continue" : productizedWarningLine(warning)
        }
        if trimmed(snapshot.resumeSource) == "cloud" {
            return "AI-assisted Continue"
        }
        let warning = trimmed(snapshot.resumeWarning)
        return warning.isEmpty ? "Local evidence" : productizedWarningLine(warning)
    }

    private var productizedErrorLine: String {
        let error = trimmed(snapshot.lastError)
        if error.isEmpty {
            return "Open Smalltalk to fix local memory"
        }
        let lower = error.lowercased()
        if lower.contains("permission") || lower.contains("denied") || lower.contains("not authorized") {
            return "Memory needs permission"
        }
        if lower.contains("screen") || lower.contains("accessibility") || lower.contains("ax") {
            return "Memory needs attention"
        }
        if lower.contains("no space") || lower.contains("disk") || lower.contains("storage") {
            return "Local memory needs cleanup"
        }
        if lower.contains("privacy") || lower.contains("excluded") || lower.contains("never_send_to_ai") {
            return "Privacy boundary active"
        }
        return "Open Smalltalk to inspect local evidence"
    }

    private func productizedWarningLine(_ value: String) -> String {
        let warning = value.lowercased()
        if warning.contains("thin") || warning.contains("missing") || warning.contains("no_") {
            return "Evidence is thin"
        }
        if warning.contains("key") || warning.contains("openai") {
            return "AI unavailable; using local evidence"
        }
        if warning.contains("privacy") || warning.contains("excluded") || warning.contains("redacted") {
            return "Privacy boundary active"
        }
        return "Open Smalltalk to inspect local evidence"
    }

    private func trimmed(_ value: String?) -> String {
        value?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    private func isPrivatePrivacyLabel(_ value: String?) -> Bool {
        let label = trimmed(value).lowercased()
        guard !label.isEmpty else { return false }
        return !["normal", "ok", "allowed"].contains(label)
    }

    private var displayTrailLabels: [String] {
        let labels = (snapshot.trailLabels ?? [])
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        return Array(labels.suffix(4))
    }

    private var activeBreath: Double {
        guard captureActive && !reduceMotion else { return 0 }
        return 0.5 + 0.5 * sin(anim.value * 1.85)
    }

    private var signalColor: Color {
        switch productState {
        case .needsAttention:
            return Color(red: 1.0, green: 0.36, blue: 0.28)
        case .newEvidence, .thinEvidence, .needsEvidence, .continueUpdating:
            return Color(red: 1.0, green: 0.68, blue: 0.28)
        case .privateOrExcluded:
            return Color.white.opacity(0.64)
        case .memoryOn, .memoryStarting, .continueReady:
            return Color(red: 0.36, green: 0.84, blue: 0.68)
        case .pausedWithEvidence, .memoryOff:
            return Color.white.opacity(0.42)
        }
    }

    var body: some View {
        ZStack {
            switch presentation {
            case .micro:
                microView
                    .transition(IslandMotion.microTransition(reduceMotion))
            case .compact:
                collapsedView
                    .transition(IslandMotion.compactTransition(reduceMotion))
            case .expanded:
                expandedView
                    .transition(IslandMotion.expandedTransition(reduceMotion))
            }
        }
        .fixedSize()
        .background(Color.clear)
        .accessibilityLabel(accessibilityLabel)
        .animation(IslandMotion.reveal(reduceMotion), value: presentation)
        .animation(IslandMotion.settle(reduceMotion), value: primaryDisplayText)
        .animation(IslandMotion.settle(reduceMotion), value: captureActive)
        .animation(IslandMotion.settle(reduceMotion), value: metrics.screenActive)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var accessibilityLabel: String {
        switch productState {
        case .memoryOn:
            return "Smalltalk local memory is on"
        case .newEvidence:
            return "Smalltalk noticed new evidence"
        case .continueReady:
            return "Smalltalk Continue is ready"
        case .thinEvidence, .needsEvidence:
            return "Smalltalk evidence is thin"
        case .privateOrExcluded:
            return "Smalltalk is not observing this app"
        case .needsAttention:
            return "Smalltalk needs attention"
        case .memoryStarting:
            return "Smalltalk local memory is starting"
        case .continueUpdating:
            return "Smalltalk is finding a continuation"
        case .pausedWithEvidence:
            return "Smalltalk has local evidence ready"
        case .memoryOff:
            return "Smalltalk local memory is off"
        }
    }

    private var microView: some View {
        Button {
            presentation = .compact
            onAction("reveal_compact")
        } label: {
            MemorySignalView(
                active: captureActive,
                processing: isProcessing,
                error: hasError,
                privateMode: signalPrivate,
                thinEvidence: signalThin,
                ready: signalReady,
                frameCount: signalEvidenceCount,
                density: 3,
                pulseNonce: snapshot.capturePulseNonce,
                scale: scale,
                width: kBaseMicroVisualW,
                height: kBaseMicroVisualH,
                cornerRadius: 999,
                compact: true
            )
                .shadow(color: .black.opacity(0.26), radius: s(4), x: 0, y: s(2))
                .frame(width: kBaseMicroHitW * scale, height: kBaseMicroHitH * scale)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(accessibilityLabel)
        .onHover { hovering in
            if hovering {
                presentation = .compact
                onAction("reveal_compact")
            }
        }
    }

    private var collapsedView: some View {
        HStack(spacing: s(7)) {
            MemorySignalView(
                active: captureActive,
                processing: isProcessing,
                error: hasError,
                privateMode: signalPrivate,
                thinEvidence: signalThin,
                ready: signalReady,
                frameCount: signalEvidenceCount,
                density: 5,
                pulseNonce: snapshot.capturePulseNonce,
                scale: scale,
                width: 34,
                height: 24,
                cornerRadius: 8,
                compact: true
            )
            .frame(width: s(38), height: s(30))

            VStack(alignment: .leading, spacing: s(1)) {
                Text(primaryDisplayText)
                    .font(Brand.swiftUIFont(size: s(12), weight: .semibold))
                    .foregroundColor(.white.opacity(0.92))
                    .lineLimit(1)
                    .monospacedDigit()
                    .fixedSize(horizontal: true, vertical: false)
                    .id("compact-primary-\(primaryDisplayText)")
                    .transition(.opacity.combined(with: .move(edge: .top)))
                Text(secondaryDisplayText)
                    .font(Brand.swiftUIFont(size: s(10.5), weight: .medium))
                    .foregroundColor(.white.opacity(0.54))
                    .lineLimit(1)
                    .fixedSize(horizontal: true, vertical: false)
                    .id("compact-secondary-\(secondaryDisplayText)")
                    .transition(.opacity)
            }
            .frame(width: s(106), alignment: .leading)

            Spacer(minLength: s(2))

            Button {
                onAction(primaryButtonAction)
            } label: {
                Text(compactActionLabel)
                    .font(Brand.swiftUIFont(size: s(10.5), weight: .semibold))
                    .foregroundColor(.white.opacity(primaryActionDisabled ? 0.36 : signalHovered ? 0.96 : 0.78))
                    .lineLimit(1)
                    .frame(width: s(48), height: s(26))
                    .background(
                        Capsule()
                            .fill(Color.white.opacity(primaryActionDisabled ? 0.035 : signalHovered ? 0.14 : 0.075))
                    )
                    .overlay(
                        Capsule()
                            .stroke(Color.white.opacity(primaryActionDisabled ? 0.05 : signalHovered ? 0.18 : 0.09), lineWidth: 0.6)
                    )
                    .scaleEffect(signalHovered && !primaryActionDisabled && !reduceMotion ? 1.035 : 1)
            }
            .buttonStyle(.plain)
            .disabled(primaryActionDisabled)
            .frame(width: s(52), height: s(30))
            .contentShape(Rectangle())
            .accessibilityLabel(primaryActionLabel)
            .onHover { hovering in
                withAnimation(IslandMotion.quick(reduceMotion)) {
                    signalHovered = hovering
                }
            }
        }
        .padding(.leading, s(7))
        .padding(.trailing, s(7))
        .frame(width: kBaseCollapsedW * scale, height: kBaseCollapsedH * scale)
        .background(capsuleFill(active: captureActive))
        .overlay(capsuleStroke(active: captureActive))
        .overlay(alignment: .top) {
            Capsule()
                .fill(Color.white.opacity(captureActive ? 0.07 + 0.035 * activeBreath : 0.08))
                .frame(height: max(1, 1 * scale))
                .padding(.horizontal, s(18))
                .padding(.top, s(1))
                .allowsHitTesting(false)
        }
        .overlay(alignment: .bottom) {
            Capsule()
                .fill(
                    LinearGradient(
                        colors: [
                            Color.clear,
                            signalColor.opacity(captureActive ? 0.08 + 0.04 * activeBreath : 0),
                            Color.clear,
                        ],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                )
                .frame(height: s(8))
                .blur(radius: s(4))
                .padding(.horizontal, s(20))
                .allowsHitTesting(false)
        }
        .clipShape(Capsule())
        .shadow(color: .black.opacity(0.24), radius: s(13), x: 0, y: s(7))
        .shadow(color: signalColor.opacity(captureActive ? 0.08 + 0.025 * activeBreath : 0), radius: s(10), x: 0, y: 0)
        .onHover { hovering in
            if hovering {
                onAction("keep_compact")
            }
        }
    }

    private var expandedView: some View {
        VStack(alignment: .leading, spacing: s(13)) {
            HStack(alignment: .center, spacing: s(11)) {
                MemorySignalView(
                    active: captureActive,
                    processing: isProcessing,
                    error: hasError,
                    privateMode: signalPrivate,
                    thinEvidence: signalThin,
                    ready: signalReady,
                    frameCount: signalEvidenceCount,
                    density: 6,
                    pulseNonce: snapshot.capturePulseNonce,
                    scale: scale,
                    width: 46,
                    height: 32,
                    cornerRadius: 10,
                    compact: false
                )
                .frame(width: s(48), height: s(34))

                VStack(alignment: .leading, spacing: s(3)) {
                    Text(primaryDisplayText)
                        .font(Brand.swiftUIFont(size: s(13.5), weight: .semibold))
                        .foregroundColor(.white.opacity(0.94))
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .monospacedDigit()
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .id("expanded-primary-\(primaryDisplayText)")
                        .transition(.opacity.combined(with: .move(edge: .top)))
                    Text(secondaryDisplayText)
                        .font(Brand.swiftUIFont(size: s(11.5), weight: .medium))
                        .foregroundColor(.white.opacity(0.58))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .transition(.opacity)
                }
                .frame(maxWidth: .infinity, alignment: .leading)

                Spacer(minLength: s(8))

                MemorySignalView(
                    active: captureActive,
                    processing: isProcessing,
                    error: hasError,
                    privateMode: signalPrivate,
                    thinEvidence: signalThin,
                    ready: signalReady,
                    frameCount: signalEvidenceCount,
                    density: 7,
                    pulseNonce: snapshot.capturePulseNonce,
                    scale: scale,
                    width: 104,
                    height: 30,
                    cornerRadius: 7,
                    compact: false
                )
                .frame(width: s(104), alignment: .trailing)
            }

            if snapshot.state == "resume_ready" && productState != .privateOrExcluded {
                VStack(alignment: .leading, spacing: s(7)) {
                    Text(resumeDetailEyebrow)
                        .font(Brand.swiftUIMonoFont(size: s(9), weight: .semibold))
                        .foregroundColor(.white.opacity(0.50))
                        .lineLimit(1)
                    Text(resumeDetailLine)
                        .font(Brand.swiftUIFont(size: s(12), weight: .medium))
                        .foregroundColor(.white.opacity(0.74))
                        .lineLimit(4)
                        .multilineTextAlignment(.leading)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)

                    Text(resumeProvenanceLine)
                        .font(Brand.swiftUIMonoFont(size: s(10), weight: .semibold))
                        .foregroundColor(.white.opacity(0.54))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                    .padding(.horizontal, s(10))
                    .padding(.vertical, s(10))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(
                        RoundedRectangle(cornerRadius: s(11), style: .continuous)
                            .fill(Color.white.opacity(0.055))
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: s(11), style: .continuous)
                            .stroke(Color.white.opacity(0.075), lineWidth: 0.7)
                    )
                    .transition(.opacity.combined(with: .move(edge: .top)))
            }

            Rectangle()
                .fill(
                    LinearGradient(
                        colors: [Color.white.opacity(0.02), Color.white.opacity(0.095), Color.white.opacity(0.02)],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                )
                .frame(height: max(1, 1 * scale))
                .padding(.horizontal, s(1))

            HStack(spacing: s(9)) {
                GlassActionButton(
                    label: primaryActionLabel,
                    scale: scale,
                    prominent: true,
                    disabled: primaryActionDisabled
                ) {
                    onAction(primaryButtonAction)
                }

                GlassActionButton(
                    label: secondaryActionLabel,
                    scale: scale,
                    prominent: false,
                    disabled: secondaryActionDisabled
                ) {
                    onAction(secondaryButtonAction)
                }
            }
        }
        .padding(.horizontal, s(18))
        .padding(.vertical, s(17))
        .frame(width: kBaseExpandedW * scale, height: kBaseExpandedH * scale)
        .background(expandedGlassFill(active: captureActive))
        .overlay(expandedGlassStroke(active: captureActive))
        .clipShape(RoundedRectangle(cornerRadius: s(18), style: .continuous))
        .shadow(color: .black.opacity(0.34), radius: s(22), x: 0, y: s(12))
        .shadow(color: signalColor.opacity(captureActive ? 0.10 + 0.035 * activeBreath : 0), radius: s(22), x: 0, y: 0)
    }

    private func capsuleFill(active: Bool) -> some ShapeStyle {
        LinearGradient(
            colors: active
                ? [
                    Color(red: 0.06, green: 0.15, blue: 0.13).opacity(0.98),
                    Color(red: 0.055, green: 0.078, blue: 0.080).opacity(0.98),
                ]
                : [
                    Color(red: 0.12, green: 0.125, blue: 0.135).opacity(0.98),
                    Color(red: 0.075, green: 0.08, blue: 0.09).opacity(0.98),
                ],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }

    private func capsuleStroke(active: Bool) -> some View {
        Capsule()
            .stroke(
                active ? signalColor.opacity(0.34) : Color.white.opacity(0.16),
                lineWidth: 0.7
            )
            .overlay(
                Capsule()
                    .stroke(Color.black.opacity(0.44), lineWidth: 1)
                    .offset(y: s(0.5))
                    .blur(radius: 0.4)
            )
    }

    private func expandedGlassFill(active: Bool) -> some ShapeStyle {
        LinearGradient(
            colors: active
                ? [
                    Color(red: 0.055, green: 0.135, blue: 0.12).opacity(0.99),
                    Color(red: 0.060, green: 0.072, blue: 0.080).opacity(0.99),
                ]
                : [
                    Color(red: 0.115, green: 0.12, blue: 0.13).opacity(0.99),
                    Color(red: 0.065, green: 0.07, blue: 0.08).opacity(0.99),
                ],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }

    private func expandedGlassStroke(active: Bool) -> some View {
        let shape = RoundedRectangle(cornerRadius: s(18), style: .continuous)
        return shape
            .stroke(active ? signalColor.opacity(0.32) : Color.white.opacity(0.14), lineWidth: 0.8)
            .overlay(
                shape
                    .stroke(Color.black.opacity(0.48), lineWidth: 1)
                    .offset(y: s(0.5))
                    .blur(radius: 0.5)
            )
            .overlay(alignment: .topLeading) {
                shape
                    .stroke(Color.white.opacity(0.08), lineWidth: 1)
                    .frame(height: kBaseExpandedH * scale)
                    .allowsHitTesting(false)
            }
    }
}

@available(macOS 13.0, *)
private struct LiveDot: View {
    let active: Bool
    let state: String
    let scale: CGFloat
    @ObservedObject private var anim = AnimationTick.shared

    private var color: Color {
        if state == "processing" || state == "stopped_toast" {
            return Color(red: 1.0, green: 0.72, blue: 0.25)
        }
        if active {
            return Color(red: 0.36, green: 0.84, blue: 0.68)
        }
        return .white.opacity(0.38)
    }

    var body: some View {
        let pulse = active ? 0.68 + 0.32 * abs(sin(anim.value * 4.0)) : 1.0
        Circle()
            .fill(color.opacity(pulse))
            .frame(width: 5 * scale, height: 5 * scale)
            .overlay(
                Circle()
                    .stroke(color.opacity(active ? 0.32 : 0.12), lineWidth: 2 * scale)
                    .scaleEffect(active ? 1.35 : 1)
            )
    }
}

@available(macOS 13.0, *)
private struct IslandPressButtonStyle: ButtonStyle {
    let reduceMotion: Bool
    let pressedScale: CGFloat

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed && !reduceMotion ? pressedScale : 1)
            .animation(IslandMotion.quick(reduceMotion), value: configuration.isPressed)
    }
}

@available(macOS 13.0, *)
private struct EvidenceChip: View {
    let label: String
    let scale: CGFloat

    var body: some View {
        Text(label)
            .font(Brand.swiftUIMonoFont(size: 7.5 * scale, weight: .medium))
            .foregroundColor(.white.opacity(0.58))
            .lineLimit(1)
            .fixedSize()
            .padding(.horizontal, 5 * scale)
            .frame(height: 14 * scale)
            .background(
                Capsule()
                    .fill(Color.white.opacity(0.07))
            )
            .overlay(
                Capsule()
                    .stroke(Color.white.opacity(0.08), lineWidth: 0.5)
            )
    }
}

@available(macOS 13.0, *)
private struct TrailRouteView: View {
    let labels: [String]
    let scale: CGFloat

    var body: some View {
        HStack(spacing: 5 * scale) {
            ForEach(Array(labels.enumerated()), id: \.offset) { index, label in
                Text(label)
                    .font(Brand.swiftUIFont(size: 9.5 * scale, weight: index == labels.count - 1 ? .semibold : .medium))
                    .foregroundColor(.white.opacity(index == labels.count - 1 ? 0.82 : 0.58))
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .frame(maxWidth: 68 * scale, alignment: .leading)
                    .padding(.horizontal, 7 * scale)
                    .frame(height: 20 * scale)
                    .background(
                        Capsule()
                            .fill(Color.white.opacity(index == labels.count - 1 ? 0.085 : 0.045))
                    )
                    .overlay(
                        Capsule()
                            .stroke(Color.white.opacity(index == labels.count - 1 ? 0.13 : 0.07), lineWidth: 0.6)
                    )

                if index < labels.count - 1 {
                    Image(systemName: "chevron.right")
                        .font(.system(size: 6.5 * scale, weight: .semibold))
                        .foregroundColor(.white.opacity(0.30))
                        .frame(width: 6 * scale)
                }
            }
        }
        .frame(height: 20 * scale)
        .frame(maxWidth: .infinity, alignment: .leading)
        .clipped()
        .allowsHitTesting(false)
    }
}

@available(macOS 13.0, *)
private struct TextActionButton: View {
    let label: String
    let scale: CGFloat
    let action: () -> Void
    @State private var hovered = false

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(Brand.swiftUIMonoFont(size: 8 * scale, weight: .semibold))
                .foregroundColor(.white.opacity(hovered ? 0.95 : 0.72))
                .lineLimit(1)
                .fixedSize()
                .frame(width: 48 * scale, height: 26 * scale)
                .background(
                    Capsule()
                        .fill(hovered ? Color.white.opacity(0.14) : Color.white.opacity(0.065))
                )
                .overlay(
                    Capsule()
                        .stroke(Color.white.opacity(hovered ? 0.18 : 0.08), lineWidth: 0.5)
                )
                .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .onHover { hovered = $0 }
    }
}

@available(macOS 13.0, *)
private struct GlassActionButton: View {
    let label: String
    let scale: CGFloat
    let prominent: Bool
    let disabled: Bool
    let action: () -> Void
    @State private var hovered = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Button {
            if !disabled {
                action()
            }
        } label: {
            Text(label)
                .font(Brand.swiftUIFont(size: 11 * scale, weight: .semibold))
                .foregroundColor(.white.opacity(disabled ? 0.34 : prominent ? 0.92 : hovered ? 0.86 : 0.66))
                .lineLimit(1)
                .fixedSize()
                .frame(maxWidth: .infinity)
                .frame(height: 30 * scale)
                .background(buttonFill)
                .overlay(buttonStroke)
                .clipShape(RoundedRectangle(cornerRadius: 8 * scale, style: .continuous))
                .contentShape(RoundedRectangle(cornerRadius: 8 * scale, style: .continuous))
                .scaleEffect(hovered && !disabled && !reduceMotion ? 1.015 : 1)
                .shadow(
                    color: prominent && !disabled
                        ? Color.white.opacity(hovered ? 0.08 : 0.025)
                        : Color.clear,
                    radius: 7 * scale,
                    x: 0,
                    y: 0
                )
        }
        .buttonStyle(IslandPressButtonStyle(reduceMotion: reduceMotion, pressedScale: 0.965))
        .disabled(disabled)
        .onHover { hovering in
            withAnimation(IslandMotion.quick(reduceMotion)) {
                hovered = hovering
            }
        }
    }

    private var buttonFill: some View {
        RoundedRectangle(cornerRadius: 8 * scale, style: .continuous)
            .fill(
                prominent
                    ? Color.white.opacity(disabled ? 0.055 : hovered ? 0.16 : 0.115)
                    : Color.white.opacity(disabled ? 0.025 : hovered ? 0.075 : 0.045)
            )
    }

    private var buttonStroke: some View {
        RoundedRectangle(cornerRadius: 8 * scale, style: .continuous)
            .stroke(Color.white.opacity(disabled ? 0.045 : prominent ? hovered ? 0.20 : 0.13 : hovered ? 0.13 : 0.075), lineWidth: 0.7)
    }
}

@available(macOS 13.0, *)
private struct PlayPauseButton: View {
    let isActive: Bool
    let scale: CGFloat
    let disabled: Bool
    let action: () -> Void
    @State private var hovered = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Button {
            if !disabled {
                action()
            }
        } label: {
            ZStack {
                Circle()
                    .fill(
                        isActive
                            ? Color(red: 0.36, green: 0.84, blue: 0.68).opacity(disabled ? 0.38 : hovered ? 0.96 : 0.86)
                            : Color.white.opacity(disabled ? 0.045 : hovered ? 0.12 : 0.075)
                    )
                Circle()
                    .stroke(Color.white.opacity(disabled ? 0.055 : isActive ? 0.22 : hovered ? 0.16 : 0.09), lineWidth: 0.6)

                Image(systemName: isActive ? "pause.fill" : "play.fill")
                    .font(.system(size: 9.5 * scale, weight: .semibold))
                    .foregroundColor(disabled ? .white.opacity(0.32) : isActive ? .white.opacity(0.94) : (hovered ? .white.opacity(0.82) : .white.opacity(0.58)))
                    .offset(x: isActive ? 0 : 0.7 * scale)
                    .rotationEffect(.degrees(isActive || reduceMotion ? 0 : hovered ? -4 : 0))
                    .animation(IslandMotion.quick(reduceMotion), value: isActive)
            }
            .frame(width: 28 * scale, height: 28 * scale)
            .frame(width: 32 * scale, height: 32 * scale)
            .contentShape(Circle())
            .scaleEffect(hovered && !disabled && !reduceMotion ? 1.045 : 1)
            .shadow(
                color: isActive
                    ? Color(red: 0.36, green: 0.84, blue: 0.68).opacity(hovered ? 0.28 : 0.16)
                    : Color.white.opacity(hovered ? 0.08 : 0),
                radius: 8 * scale,
                x: 0,
                y: 0
            )
        }
        .buttonStyle(IslandPressButtonStyle(reduceMotion: reduceMotion, pressedScale: 0.92))
        .disabled(disabled)
        .onHover { hovering in
            withAnimation(IslandMotion.quick(reduceMotion)) {
                hovered = hovering
            }
        }
        .animation(IslandMotion.settle(reduceMotion), value: isActive)
    }
}

private var gOverlayScale: CGFloat = 1.0

@available(macOS 13.0, *)
private final class SessionIslandController: NSObject {
    static let shared = SessionIslandController()

    private var panel: NSPanel?
    private var hostingView: NSHostingView<AnyView>?
    private var trackingView: IslandTrackingView?
    private var presentation: IslandPresentation = .micro
    private var visible = false
    private var snapshot = IslandSnapshot()
    private var metrics = OverlayMetrics()
    private var previousFrameCount: Int64?
    private var outsideGlobalClickMonitor: Any?
    private var outsideLocalClickMonitor: Any?
    private var idleMicroTimer: Timer?
    private var lastCompactRevealAt: Date?
    private var suppressOpenExpandedUntil: Date?

    func initializeIfNeeded() {
        if panel == nil {
            createPanel()
            updateContent()
            positionPanel(preserveCurrentAnchor: false, animated: false)
        }
    }

    func update(json: String) {
        let wasMicroEligible = canUseMicro

        if let data = json.data(using: .utf8),
           let next = try? JSONDecoder().decode(IslandSnapshot.self, from: data) {
            snapshot = next
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

        if snapshot.state == "starting" || snapshot.state == "processing" || snapshot.state == "stopped_toast" {
            setPresentation(.compact, resetIdleTimer: false)
        } else if canUseMicro {
            if !wasMicroEligible && presentation == .compact {
                scheduleMicroReturn()
            }
        } else if presentation == .micro {
            setPresentation(.compact, resetIdleTimer: false)
        } else {
            cancelMicroReturn()
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
        AnimationTick.shared.start()
    }

    func hide() {
        visible = false
        presentation = .micro
        cancelMicroReturn()
        updateOutsideClickMonitors()
        AnimationTick.shared.stop()
        DispatchQueue.main.async { [self] in
            panel?.orderOut(nil)
            updateContent()
        }
    }

    func setExpanded(_ expanded: Bool) {
        setPresentation(expanded ? .expanded : .compact)
    }

    func reposition() {
        initializeIfNeeded()
        positionPanel(preserveCurrentAnchor: false, animated: false)
    }

    func shutdown() {
        AnimationTick.shared.stop()
        cancelMicroReturn()
        removeOutsideClickMonitors()
        panel?.orderOut(nil)
        panel = nil
        hostingView = nil
        trackingView = nil
        visible = false
    }

    private func isRecording(_ state: String) -> Bool {
        state == "recording_compact" || state == "recording_expanded"
    }

    private var canUseMicro: Bool {
        snapshot.lastError == nil &&
            (
                snapshot.state == "ready" ||
                    snapshot.state == "recording_compact" ||
                    snapshot.state == "recording_expanded" ||
                    snapshot.state == "resume_ready"
            )
    }

    private func setPresentation(
        _ nextPresentation: IslandPresentation,
        resetIdleTimer: Bool = true
    ) {
        let normalized = normalizePresentation(nextPresentation)
        let changed = presentation != normalized
        presentation = normalized

        if changed {
            updateOutsideClickMonitors()
            updateContent()
            positionPanel(preserveCurrentAnchor: true, animated: visible)
        }

        if presentation == .compact && canUseMicro {
            if resetIdleTimer || idleMicroTimer == nil {
                scheduleMicroReturn()
            }
        } else {
            cancelMicroReturn()
        }
    }

    private func normalizePresentation(_ nextPresentation: IslandPresentation) -> IslandPresentation {
        if nextPresentation == .micro && !canUseMicro {
            return .compact
        }
        return nextPresentation
    }

    private func revealCompact() {
        lastCompactRevealAt = Date()
        setPresentation(.compact, resetIdleTimer: true)
    }

    private func openExpandedFromCompact() {
        if let suppressOpenExpandedUntil,
           Date() < suppressOpenExpandedUntil {
            revealCompact()
            return
        }
        if let lastCompactRevealAt,
           Date().timeIntervalSince(lastCompactRevealAt) < 0.25 {
            revealCompact()
            return
        }
        setPresentation(.expanded)
    }

    private func markPanelDragBegan() {
        suppressOpenExpandedUntil = Date().addingTimeInterval(0.35)
        revealCompact()
    }

    private func scheduleMicroReturn() {
        guard canUseMicro, presentation == .compact else {
            cancelMicroReturn()
            return
        }

        cancelMicroReturn()
        let timer = Timer(timeInterval: kIdleMicroDelay, repeats: false) { [weak self] _ in
            DispatchQueue.main.async {
                self?.returnToMicroIfIdle()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        idleMicroTimer = timer
    }

    private func cancelMicroReturn() {
        idleMicroTimer?.invalidate()
        idleMicroTimer = nil
    }

    private func returnToMicroIfIdle() {
        guard canUseMicro, presentation == .compact else { return }
        setPresentation(.micro, resetIdleTimer: false)
    }

    private var targetPanelSize: NSSize {
        switch presentation {
        case .micro:
            return NSSize(width: kBaseMicroHitW * gOverlayScale, height: kBaseMicroHitH * gOverlayScale)
        case .compact:
            return NSSize(width: kBaseCollapsedW * gOverlayScale, height: kBaseCollapsedH * gOverlayScale)
        case .expanded:
            return NSSize(width: kBaseExpandedW * gOverlayScale, height: kBaseExpandedH * gOverlayScale)
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
                context.duration = kPanelFrameAnimDur
                context.timingFunction = IslandMotion.panelTimingFunction()
                panel.animator().setFrame(frame, display: visible)
            }
        } else {
            panel.setFrame(frame, display: visible)
        }
    }

    private func updateContent() {
        guard let panel, let contentView = panel.contentView else { return }
        let controller = self
        let view = SessionIslandView(
            snapshot: snapshot,
            metrics: metrics,
            scale: gOverlayScale,
            onAction: { [weak self] action in
                self?.handle(action: action)
            },
            presentation: Binding(
                get: { controller.presentation },
                set: { controller.setPresentation($0) }
            )
        )

        if let hostingView {
            hostingView.rootView = AnyView(view)
        } else {
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
            setPresentation(.expanded, resetIdleTimer: false)
            sendAction("continue")
        case "start_memory":
            revealCompact()
            sendAction("start_capture")
        case "pause_memory":
            revealCompact()
            sendAction("stop_capture")
        case "reconstruct_trail":
            setPresentation(.expanded, resetIdleTimer: false)
            sendAction("continue")
        case "show_trail":
            sendAction("show_trail")
        case "open_resume_point":
            sendAction("open_resume_point")
        case "open_main_window":
            sendAction("open_main_window")
        case "open_expanded":
            openExpandedFromCompact()
        case "toggle_meeting":
            revealCompact()
            sendAction(metrics.meetingActive ? "stop_capture" : "start_capture")
        case "capture_once":
            revealCompact()
            sendAction("capture_once")
        case "reveal_compact", "keep_compact":
            revealCompact()
        case "close":
            setExpanded(false)
            sendAction("collapse")
        default:
            break
        }
    }

    private func updateOutsideClickMonitors() {
        if visible && presentation == .expanded {
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
        guard presentation == .expanded, let panel else { return }
        if event.window === panel {
            return
        }
        if NSMouseInRect(NSEvent.mouseLocation, panel.frame, false) {
            return
        }

        DispatchQueue.main.async { [weak self] in
            self?.setExpanded(false)
            self?.sendAction("collapse")
        }
    }

    private func sendAction(_ action: String) {
        guard let callback = gActionCallback else { return }
        let json = "{\"action\":\"\(action)\"}"
        json.withCString { pointer in
            callback(pointer)
        }
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
