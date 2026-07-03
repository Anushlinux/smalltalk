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
    var trailAppCount: Int64?
    var trailMomentCount: Int64?
    var trailLabels: [String]?
    var lastFrameId: Int64?
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
    var resumeWarning: String?

    enum CodingKeys: String, CodingKey {
        case state
        case elapsedMs = "elapsed_ms"
        case frameCount = "frame_count"
        case trailAppCount = "trail_app_count"
        case trailMomentCount = "trail_moment_count"
        case trailLabels = "trail_labels"
        case lastFrameId = "last_frame_id"
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
        case resumeWarning = "resume_warning"
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
private struct EvidenceTapeView: View {
    let active: Bool
    let processing: Bool
    let error: Bool
    let frameCount: Int64
    let pulseNonce: UInt64?
    let density: Double
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
        let heat = active ? 0.5 + 0.5 * sin(anim.value * 4.6) : 0.0

        Canvas { context, size in
            drawTape(context: &context, size: size, pulseStrength: pulseStrength, heat: heat)
        }
        .frame(width: width * scale, height: height * scale)
        .background(
            shape.fill(
                LinearGradient(
                    colors: [
                        Color(red: 0.018, green: 0.020, blue: 0.024).opacity(0.96),
                        Color(red: 0.045, green: 0.048, blue: 0.056).opacity(0.94),
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
            )
        )
        .overlay(
            shape.stroke(
                error
                    ? Color(red: 1.0, green: 0.62, blue: 0.24).opacity(0.40)
                    : active
                        ? Color(red: 1.0, green: 0.28, blue: 0.22).opacity(0.18 + 0.08 * heat)
                        : Color.white.opacity(0.10),
                lineWidth: 0.7
            )
        )
        .overlay(alignment: .top) {
            shape
                .stroke(Color.white.opacity(active ? 0.08 : 0.055), lineWidth: 1)
                .blur(radius: 0.25)
                .offset(y: -0.5 * scale)
        }
        .overlay(
            shape.fill(
                Color(red: 1, green: 0.22, blue: 0.16)
                    .opacity(pulseStrength * (compact ? 0.13 : 0.11))
            )
        )
        .shadow(
            color: error
                ? Color(red: 1.0, green: 0.56, blue: 0.18).opacity(0.14)
                : active
                    ? Color(red: 1.0, green: 0.18, blue: 0.14).opacity(0.10 + 0.06 * heat)
                    : Color.clear,
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
    }

    private func drawTape(
        context: inout GraphicsContext,
        size: CGSize,
        pulseStrength: Double,
        heat: Double
    ) {
        let tick = anim.value
        let tapeRect = CGRect(origin: .zero, size: size)

        context.fill(Path(tapeRect), with: .color(.black.opacity(0.18)))

        let phase = scanPhase(tick)
        let scanX = round(phase * size.width)
        let red = Color(red: 1.0, green: 0.22, blue: 0.16)
        let amber = Color(red: 1.0, green: 0.62, blue: 0.24)

        let rows = max(3, Int(round(density)))
        for index in 1..<rows {
            let offset = index % 2 == 0 ? 0.18 : -0.08
            let y = round((Double(index) + offset) * size.height / Double(rows))
            context.fill(
                Path(CGRect(x: 0, y: y, width: size.width, height: 1)),
                with: .color(.white.opacity(active ? 0.040 : 0.028))
            )
        }

        let columns = max(4, Int(round(density + 2)))
        for index in 1..<columns {
            let skew = (index % 3 == 0 ? 0.30 : index % 2 == 0 ? -0.16 : 0.06)
            let x = round((Double(index) + skew) * size.width / Double(columns))
            context.fill(
                Path(CGRect(x: x, y: 0, width: 1, height: size.height)),
                with: .color(.white.opacity(active ? 0.028 : 0.017))
            )
        }

        if active || pulseStrength > 0 {
            let filledWidth = max(0, min(size.width, scanX))
            context.fill(
                Path(CGRect(x: 0, y: 0, width: filledWidth, height: size.height)),
                with: .color(red.opacity(active ? 0.035 + 0.025 * heat : 0.018 * pulseStrength))
            )
        }

        drawFrameTicks(context: &context, size: size, red: red, pulseStrength: pulseStrength)

        if active || pulseStrength > 0 || processing {
            let width = processing ? max(2, 2 * scale) : max(1, 1.2 * scale)
            context.fill(
                Path(CGRect(x: scanX, y: 1 * scale, width: width, height: max(1, size.height - 2 * scale))),
                with: .color((processing ? Color.white : red).opacity(processing ? 0.30 : 0.48 + 0.20 * heat))
            )
        }

        if pulseStrength > 0 {
            let bloomX = max(0, min(size.width - 2 * scale, scanX - 1 * scale))
            let bloomRect = CGRect(
                x: max(0, bloomX - size.width * 0.22),
                y: 0,
                width: min(size.width * 0.44, size.width),
                height: size.height
            )
            let bloom = GraphicsContext.Shading.linearGradient(
                Gradient(stops: [
                    .init(color: .white.opacity(0), location: 0.0),
                    .init(color: .white.opacity(0.24 * pulseStrength), location: 0.42),
                    .init(color: red.opacity(0.24 * pulseStrength), location: 0.58),
                    .init(color: .white.opacity(0), location: 1.0),
                ]),
                startPoint: CGPoint(x: bloomRect.minX, y: bloomRect.midY),
                endPoint: CGPoint(x: bloomRect.maxX, y: bloomRect.midY)
            )
            context.fill(Path(bloomRect), with: bloom)
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

        if error {
            let y = round(size.height * 0.63)
            context.fill(
                Path(CGRect(x: size.width * 0.12, y: y, width: size.width * 0.76, height: max(1, 1 * scale))),
                with: .color(amber.opacity(0.50))
            )
            context.fill(
                Path(CGRect(x: size.width * 0.58, y: max(0, y - 3 * scale), width: max(1, 1 * scale), height: min(size.height, 6 * scale))),
                with: .color(amber.opacity(0.72))
            )
        }
    }

    private func drawFrameTicks(
        context: inout GraphicsContext,
        size: CGSize,
        red: Color,
        pulseStrength: Double
    ) {
        let count = max(0, frameCount)
        guard count > 0 else { return }
        let tickCount = Int(min(count, compact ? 9 : 18))
        let first = count - Int64(tickCount) + 1

        for offset in 0..<tickCount {
            let frameOrdinal = first + Int64(offset)
            let fraction = tickFraction(frameOrdinal)
            let tickHeight = size.height * (0.34 + 0.10 * Double((frameOrdinal % 3 + 3) % 3))
            let x = round(max(1, min(size.width - 2, fraction * size.width)))
            let y = round((size.height - tickHeight) / 2.0)
            let isNewest = offset == tickCount - 1
            let alpha = isNewest ? 0.55 + 0.32 * pulseStrength : 0.20 + 0.07 * Double(offset % 3)
            let color = isNewest && (active || pulseStrength > 0) ? red.opacity(alpha) : Color.white.opacity(alpha)
            context.fill(
                Path(CGRect(x: x, y: y, width: max(1, 1 * scale), height: tickHeight)),
                with: .color(color)
            )
        }
    }

    private func scanPhase(_ tick: Double) -> Double {
        if processing {
            return 0.78
        }
        if reduceMotion {
            return active ? 0.62 : 0.38
        }
        let speed = active ? 0.18 : 0.030
        return fmod(tick * speed + (active ? 0.12 : 0.04), 1.0)
    }

    private func tickFraction(_ value: Int64) -> Double {
        let raw = fmod(Double(value) * 0.61803398875 + Double((value % 7 + 7) % 7) * 0.031, 1.0)
        return 0.08 + raw * 0.84
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

    private var captureActive: Bool {
        metrics.meetingActive
    }

    private var isRecording: Bool {
        snapshot.state == "recording_compact" || snapshot.state == "recording_expanded"
    }

    private var isBusy: Bool {
        snapshot.state == "starting" || snapshot.state == "processing" || snapshot.state == "trail_reconstructing"
    }

    private var isProcessing: Bool {
        snapshot.state == "processing" || snapshot.state == "trail_reconstructing"
    }

    private var hasError: Bool {
        snapshot.lastError != nil || snapshot.state == "error"
    }

    private var primaryDisplayText: String {
        if hasError {
            return "Needs attention"
        }
        switch snapshot.state {
        case "starting":
            return "Starting memory"
        case "recording_compact", "recording_expanded":
            return "Following work"
        case "processing":
            return "Saving memory"
        case "stopped_toast":
            return "Continue"
        case "trail_reconstructing":
            return "Finding continuation"
        case "resume_ready":
            return resumeReadyTitle
        default:
            return hasMoments ? "Continue" : "Ready to follow"
        }
    }

    private var secondaryDisplayText: String {
        if hasError {
            return "Capture needs attention"
        }
        switch snapshot.state {
        case "recording_compact", "recording_expanded":
            return trailSummary
        case "starting":
            return "Start local memory"
        case "processing":
            return trailSummary
        case "trail_reconstructing":
            return "Checking local evidence"
        case "resume_ready":
            return resumePointLine
        default:
            return hasMoments ? "Ready to continue" : "Start local memory"
        }
    }

    private var primaryActionLabel: String {
        if snapshot.state == "starting" {
            return "Starting"
        }
        if snapshot.state == "processing" || snapshot.state == "trail_reconstructing" {
            return snapshot.state == "trail_reconstructing" ? "Finding" : "Saving"
        }
        if snapshot.state == "resume_ready" {
            return "Continue here"
        }
        if isRecording {
            return "Mark memory"
        }
        if hasMoments {
            return "Continue"
        }
        return "Start memory"
    }

    private var secondaryActionLabel: String {
        if snapshot.state == "resume_ready" {
            return "Open app"
        }
        if isRecording {
            return "Pause"
        }
        return "Open app"
    }

    private var primaryActionDisabled: Bool {
        isBusy || (hasError && !isRecording)
    }

    private var secondaryActionDisabled: Bool {
        isBusy || (!hasMoments && !isRecording && snapshot.state != "resume_ready")
    }

    private var primaryButtonAction: String {
        if snapshot.state == "resume_ready" {
            return "open_resume_point"
        }
        if isRecording {
            return "capture_once"
        }
        if hasMoments {
            return "continue"
        }
        return "toggle_meeting"
    }

    private var secondaryButtonAction: String {
        if snapshot.state == "resume_ready" {
            return "show_trail"
        }
        if isRecording {
            return "toggle_meeting"
        }
        return "show_trail"
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

    private var trailSummary: String {
        if trailAppCount > 0 && trailMomentCount > 0 {
            return "\(trailAppCount) \(trailAppCount == 1 ? "app" : "apps") · \(momentLabel)"
        }
        if trailMomentCount > 0 {
            return momentLabel
        }
        return "Building local memory"
    }

    private var momentLabel: String {
        "\(trailMomentCount) \(trailMomentCount == 1 ? "signal" : "signals")"
    }

    private var resumePointLine: String {
        let point = trimmed(snapshot.resumePoint)
        guard !point.isEmpty else { return "Continue target ready" }
        return "Continue at \(point)"
    }

    private var resumeReadyTitle: String {
        if trimmed(snapshot.resumeSource) == "continue" {
            let warning = trimmed(snapshot.resumeWarning).lowercased()
            if warning.contains("thin") || warning.contains("missing") || warning.contains("no_") {
                return "Evidence is thin"
            }
            let headline = trimmed(snapshot.resumeHeadline)
            return headline.isEmpty ? "Ready to continue" : headline
        }
        if trimmed(snapshot.resumeSource) == "cloud" {
            return "OpenAI answered"
        }
        let warning = trimmed(snapshot.resumeWarning).lowercased()
        if warning.contains("openai_api_key") || warning.contains("key") {
            return "OpenAI key missing"
        }
        return "OpenAI unavailable"
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
            return warning.isEmpty ? "Local Continue" : warning
        }
        if trimmed(snapshot.resumeSource) == "cloud" {
            let model = trimmed(snapshot.resumeModel)
            let response = trimmed(snapshot.resumeResponseId)
            if !model.isEmpty && !response.isEmpty {
                return "\(model) · \(response)"
            }
            return model.isEmpty ? "Cloud resume" : model
        }
        let warning = trimmed(snapshot.resumeWarning)
        return warning.isEmpty ? "Local evidence" : warning
    }

    private func trimmed(_ value: String?) -> String {
        value?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    private var displayTrailLabels: [String] {
        let labels = (snapshot.trailLabels ?? [])
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        return Array(labels.suffix(4))
    }

    private var activeBreath: Double {
        guard captureActive && !reduceMotion else { return 0 }
        return 0.5 + 0.5 * sin(anim.value * 3.8)
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
        .accessibilityHidden(true)
        .animation(IslandMotion.reveal(reduceMotion), value: presentation)
        .animation(IslandMotion.settle(reduceMotion), value: primaryDisplayText)
        .animation(IslandMotion.settle(reduceMotion), value: captureActive)
        .animation(IslandMotion.settle(reduceMotion), value: metrics.screenActive)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    private var microView: some View {
        Button {
            presentation = .compact
            onAction("reveal_compact")
        } label: {
            Capsule()
                .fill(Color.white.opacity(0.025))
                .frame(width: kBaseMicroVisualW * scale, height: kBaseMicroVisualH * scale)
                .overlay(
                    Capsule()
                        .stroke(Color.white.opacity(0.66), lineWidth: max(1, 1 * scale))
                )
                .overlay(
                    Capsule()
                        .stroke(Color.black.opacity(0.34), lineWidth: max(1, 1 * scale))
                        .blur(radius: 0.3)
                        .offset(y: 0.5 * scale)
                )
                .shadow(color: .black.opacity(0.26), radius: s(4), x: 0, y: s(2))
                .frame(width: kBaseMicroHitW * scale, height: kBaseMicroHitH * scale)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            if hovering {
                presentation = .compact
                onAction("reveal_compact")
            }
        }
    }

    private var collapsedView: some View {
        HStack(spacing: s(7)) {
            PlayPauseButton(
                isActive: isRecording,
                scale: scale,
                disabled: isBusy
            ) {
                onAction("toggle_meeting")
            }

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
            .frame(width: s(96), alignment: .leading)

            Spacer(minLength: s(2))

            Button {
                onAction("open_expanded")
            } label: {
                EvidenceTapeView(
                    active: isRecording,
                    processing: isProcessing,
                    error: hasError,
                    frameCount: snapshot.frameCount,
                    pulseNonce: snapshot.capturePulseNonce,
                    density: 4,
                    scale: scale,
                    width: 60,
                    height: 18,
                    cornerRadius: 6,
                    compact: true
                )
                .scaleEffect(signalHovered ? 1.035 : 1)
                .shadow(
                    color: isRecording
                        ? Color(red: 1, green: 0.20, blue: 0.16).opacity(signalHovered ? 0.22 : 0.12)
                        : Color.white.opacity(signalHovered ? 0.08 : 0),
                    radius: s(signalHovered ? 10 : 7),
                    x: 0,
                    y: 0
                )
            }
            .buttonStyle(.plain)
            .frame(width: s(64), height: s(30))
            .contentShape(Rectangle())
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
                            Color(red: 1, green: 0.20, blue: 0.16).opacity(captureActive ? 0.08 + 0.04 * activeBreath : 0),
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
        .shadow(color: Color(red: 1, green: 0.20, blue: 0.16).opacity(captureActive ? 0.06 + 0.025 * activeBreath : 0), radius: s(10), x: 0, y: 0)
        .onHover { hovering in
            if hovering {
                onAction("keep_compact")
            }
        }
    }

    private var expandedView: some View {
        VStack(alignment: .leading, spacing: s(13)) {
            HStack(alignment: .center, spacing: s(11)) {
                PlayPauseButton(
                    isActive: isRecording,
                    scale: scale,
                    disabled: isBusy
                ) {
                    onAction("toggle_meeting")
                }

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

                EvidenceTapeView(
                    active: isRecording,
                    processing: isProcessing,
                    error: hasError,
                    frameCount: snapshot.frameCount,
                    pulseNonce: snapshot.capturePulseNonce,
                    density: 5,
                    scale: scale,
                    width: 112,
                    height: 30,
                    cornerRadius: 7,
                    compact: false
                )
                .frame(width: s(112), alignment: .trailing)
            }

            if snapshot.state != "resume_ready", !displayTrailLabels.isEmpty {
                TrailRouteView(labels: displayTrailLabels, scale: scale)
                    .transition(.opacity.combined(with: .move(edge: .top)))
            }

            if snapshot.state == "resume_ready" {
                VStack(alignment: .leading, spacing: s(7)) {
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
        .shadow(color: Color(red: 1, green: 0.20, blue: 0.16).opacity(captureActive ? 0.08 + 0.035 * activeBreath : 0), radius: s(22), x: 0, y: 0)
    }

    private func capsuleFill(active: Bool) -> some ShapeStyle {
        LinearGradient(
            colors: active
                ? [
                    Color(red: 0.18, green: 0.15, blue: 0.15).opacity(0.98),
                    Color(red: 0.09, green: 0.095, blue: 0.105).opacity(0.98),
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
                active ? Color(red: 1, green: 0.32, blue: 0.28).opacity(0.34) : Color.white.opacity(0.16),
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
                    Color(red: 0.15, green: 0.13, blue: 0.13).opacity(0.99),
                    Color(red: 0.075, green: 0.08, blue: 0.09).opacity(0.99),
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
            .stroke(active ? Color(red: 1, green: 0.32, blue: 0.28).opacity(0.32) : Color.white.opacity(0.14), lineWidth: 0.8)
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
            return Color(red: 1.0, green: 0.20, blue: 0.16)
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
                            ? Color(red: 0.92, green: 0.12, blue: 0.09).opacity(disabled ? 0.38 : hovered ? 0.96 : 0.86)
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
                    ? Color(red: 1, green: 0.20, blue: 0.16).opacity(hovered ? 0.28 : 0.16)
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
        snapshot.state == "ready" && snapshot.lastError == nil
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
        case "reconstruct_trail":
            setPresentation(.expanded, resetIdleTimer: false)
            sendAction("continue")
        case "show_trail":
            sendAction("show_trail")
        case "open_resume_point":
            sendAction("open_resume_point")
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
