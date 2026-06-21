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
    var lastError: String?

    enum CodingKeys: String, CodingKey {
        case state
        case elapsedMs = "elapsed_ms"
        case frameCount = "frame_count"
        case lastError = "last_error"
    }
}

private struct OverlayMetrics {
    var screenActive = false
    var captureFps = 0.0
    var meetingActive = false
}

private enum Brand {
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
private struct ScreenMatrixView: View {
    let active: Bool
    let captureFps: Double
    @ObservedObject private var anim = AnimationTick.shared

    var body: some View {
        Canvas { context, size in
            let tick = anim.value
            let fill = active ? min(1, captureFps / 2.0) : 0.0
            let speed = active ? 0.003 + fill * 0.007 : 0.001
            let sweepX = fmod(tick * speed * 60, 1.0) * size.width

            let capturedAlpha = active ? 0.06 + fill * 0.06 : 0.02
            context.fill(
                Path(CGRect(x: 0, y: 0, width: sweepX, height: size.height)),
                with: .color(.white.opacity(capturedAlpha))
            )
            context.fill(
                Path(CGRect(x: sweepX, y: 0, width: size.width - sweepX, height: size.height)),
                with: .color(.white.opacity(0.015))
            )
            let barAlpha = active ? 0.5 + fill * 0.2 : 0.08
            context.fill(
                Path(CGRect(x: round(sweepX), y: 0, width: 1, height: size.height)),
                with: .color(.white.opacity(barAlpha))
            )
            for index in 1..<5 {
                let y = round(Double(index) * size.height / 5.0)
                context.fill(
                    Path(CGRect(x: 0, y: y, width: size.width, height: 1)),
                    with: .color(.black.opacity(0.35))
                )
            }
        }
        .drawingGroup()
    }
}

private let kBaseCollapsedW: CGFloat = 150
private let kBaseCollapsedH: CGFloat = 34
private let kBaseMicroHitW: CGFloat = 86
private let kBaseMicroHitH: CGFloat = 24
private let kBaseMicroVisualW: CGFloat = 58
private let kBaseMicroVisualH: CGFloat = 10
private let kBaseExpandedW: CGFloat = 352
private let kBaseExpandedH: CGFloat = 112
private let kAnimDur = 0.2
private let kIdleMicroDelay: TimeInterval = 5.0
private let kPanelFrameAnimDur = 0.18

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

    private func s(_ value: CGFloat) -> CGFloat { value * scale }

    private var captureActive: Bool {
        metrics.meetingActive
    }

    private var isRecording: Bool {
        snapshot.state == "recording_compact" || snapshot.state == "recording_expanded"
    }

    private var isBusy: Bool {
        snapshot.state == "starting" || snapshot.state == "processing"
    }

    private var statusText: String {
        if snapshot.lastError != nil {
            return "ERROR"
        }
        switch snapshot.state {
        case "starting":
            return "STARTING"
        case "recording_compact", "recording_expanded":
            return "REC"
        case "processing":
            return "SAVING"
        case "stopped_toast":
            return "SAVED"
        default:
            return "READY"
        }
    }

    private var elapsedText: String {
        let totalSeconds = max(0, snapshot.elapsedMs / 1000)
        let hours = totalSeconds / 3600
        let minutes = (totalSeconds % 3600) / 60
        let seconds = totalSeconds % 60
        if hours > 0 {
            return String(format: "%lld:%02lld:%02lld", hours, minutes, seconds)
        }
        return String(format: "%02lld:%02lld", minutes, seconds)
    }

    private var frameText: String {
        let count = snapshot.frameCount
        if count == 1 {
            return "1 frame"
        }
        if count >= 1_000_000 {
            return String(format: "%.1fm frames", Double(count) / 1_000_000)
        }
        if count >= 10_000 {
            return String(format: "%.1fk frames", Double(count) / 1_000)
        }
        return "\(count) frames"
    }

    private var detailText: String {
        if snapshot.lastError != nil {
            return "Capture needs attention"
        }
        if isRecording || snapshot.state == "starting" || snapshot.state == "processing" {
            return "\(elapsedText) · \(frameText)"
        }
        return frameText
    }

    private var primaryActionLabel: String {
        if snapshot.state == "starting" {
            return "Starting"
        }
        if snapshot.state == "processing" {
            return "Saving"
        }
        return isRecording ? "Stop" : "Start"
    }

    var body: some View {
        ZStack {
            switch presentation {
            case .micro:
                microView
                    .transition(.opacity.combined(with: .scale(scale: 0.90, anchor: .center)))
            case .compact:
                collapsedView
                    .transition(.opacity.combined(with: .scale(scale: 1.03, anchor: .center)))
            case .expanded:
                expandedView
                    .transition(.opacity.combined(with: .scale(scale: 0.92, anchor: .center)))
            }
        }
        .fixedSize()
        .accessibilityHidden(true)
        .animation(.easeOut(duration: reduceMotion ? 0.01 : kAnimDur), value: presentation)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
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
        Button {
            onAction("open_expanded")
        } label: {
            HStack(spacing: s(8)) {
                LiveDot(active: isRecording, state: snapshot.state, scale: scale)

                VStack(alignment: .leading, spacing: s(1)) {
                    Text(statusText)
                        .font(Brand.swiftUIMonoFont(size: s(7.5), weight: .semibold))
                        .foregroundColor(.white.opacity(0.88))
                    Text(captureActive ? elapsedText : frameText)
                        .font(Brand.swiftUIMonoFont(size: s(9.5), weight: .medium))
                        .foregroundColor(.white.opacity(0.64))
                        .lineLimit(1)
                        .fixedSize()
                }

                Spacer(minLength: s(4))

                ScreenActivityBadge(
                    active: metrics.screenActive,
                    captureFps: metrics.captureFps,
                    scale: scale
                )
            }
            .padding(.horizontal, s(10))
            .frame(width: kBaseCollapsedW * scale, height: kBaseCollapsedH * scale)
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .background(.ultraThinMaterial)
        .background(capsuleFill(active: captureActive))
        .overlay(capsuleStroke(active: captureActive))
        .clipShape(Capsule())
        .shadow(color: .black.opacity(0.18), radius: s(16), x: 0, y: s(8))
        .onHover { hovering in
            if hovering {
                onAction("keep_compact")
            }
        }
    }

    private var expandedView: some View {
        VStack(alignment: .leading, spacing: s(14)) {
            HStack(alignment: .center, spacing: s(9)) {
                LiveDot(active: isRecording, state: snapshot.state, scale: scale)

                VStack(alignment: .leading, spacing: s(3)) {
                    Text(statusText)
                        .font(Brand.swiftUIMonoFont(size: s(9), weight: .semibold))
                        .foregroundColor(.white.opacity(0.94))
                        .lineLimit(1)
                    Text(detailText)
                        .font(Brand.swiftUIMonoFont(size: s(11), weight: .medium))
                        .foregroundColor(.white.opacity(0.68))
                        .lineLimit(1)
                }

                Spacer(minLength: s(8))

                ScreenActivityBadge(
                    active: metrics.screenActive,
                    captureFps: metrics.captureFps,
                    scale: scale
                )
            }

            HStack(spacing: s(9)) {
                GlassActionButton(
                    label: primaryActionLabel,
                    scale: scale,
                    prominent: true,
                    disabled: isBusy
                ) {
                    onAction("toggle_meeting")
                }

                GlassActionButton(
                    label: "Capture now",
                    scale: scale,
                    prominent: false,
                    disabled: !isRecording || isBusy
                ) {
                    onAction("capture_once")
                }
            }
        }
        .padding(.horizontal, s(16))
        .padding(.vertical, s(14))
        .frame(width: kBaseExpandedW * scale, height: kBaseExpandedH * scale)
        .background(.ultraThinMaterial)
        .background(expandedGlassFill(active: captureActive))
        .overlay(expandedGlassStroke(active: captureActive))
        .clipShape(RoundedRectangle(cornerRadius: s(24), style: .continuous))
        .shadow(color: .black.opacity(0.28), radius: s(28), x: 0, y: s(14))
    }

    private func capsuleFill(active: Bool) -> some ShapeStyle {
        LinearGradient(
            colors: active
                ? [Color.white.opacity(0.22), Color(red: 0.55, green: 0.09, blue: 0.07).opacity(0.16), Color.black.opacity(0.45)]
                : [Color.white.opacity(0.20), Color.black.opacity(0.32), Color.black.opacity(0.46)],
            startPoint: .leading,
            endPoint: .trailing
        )
    }

    private func capsuleStroke(active: Bool) -> some View {
        Capsule()
            .stroke(
                active ? Color(red: 1, green: 0.32, blue: 0.28).opacity(0.42) : Color.white.opacity(0.24),
                lineWidth: 0.5
            )
            .overlay(
                Capsule()
                    .stroke(Color.white.opacity(0.18), lineWidth: 1)
                    .blur(radius: 0.5)
            )
    }

    private func expandedGlassFill(active: Bool) -> some ShapeStyle {
        LinearGradient(
            colors: active
                ? [
                    Color.white.opacity(0.30),
                    Color(red: 0.22, green: 0.05, blue: 0.045).opacity(0.30),
                    Color.black.opacity(0.54),
                ]
                : [
                    Color.white.opacity(0.28),
                    Color.black.opacity(0.28),
                    Color.black.opacity(0.50),
                ],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }

    private func expandedGlassStroke(active: Bool) -> some View {
        let shape = RoundedRectangle(cornerRadius: s(24), style: .continuous)
        return shape
            .stroke(active ? Color(red: 1, green: 0.32, blue: 0.28).opacity(0.36) : Color.white.opacity(0.22), lineWidth: 0.7)
            .overlay(
                shape
                    .stroke(Color.white.opacity(0.20), lineWidth: 1)
                    .blur(radius: 0.7)
            )
            .overlay(alignment: .topLeading) {
                RoundedRectangle(cornerRadius: s(20), style: .continuous)
                    .fill(
                        LinearGradient(
                            colors: [Color.white.opacity(0.36), Color.white.opacity(0.04), Color.clear],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
                    .frame(height: s(38))
                    .padding(.horizontal, s(8))
                    .padding(.top, s(5))
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
private struct ScreenActivityBadge: View {
    let active: Bool
    let captureFps: Double
    let scale: CGFloat

    var body: some View {
        ScreenMatrixView(active: active, captureFps: captureFps)
            .frame(width: 34 * scale, height: 10 * scale)
            .background(
                RoundedRectangle(cornerRadius: 3 * scale, style: .continuous)
                    .fill(Color.white.opacity(0.055))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 3 * scale, style: .continuous)
                    .stroke(Color.white.opacity(active ? 0.18 : 0.08), lineWidth: 0.5)
            )
            .clipShape(RoundedRectangle(cornerRadius: 3 * scale, style: .continuous))
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

    var body: some View {
        Button {
            if !disabled {
                action()
            }
        } label: {
            Text(label)
                .font(Brand.swiftUIMonoFont(size: 10 * scale, weight: .semibold))
                .foregroundColor(.white.opacity(disabled ? 0.38 : hovered ? 0.98 : 0.86))
                .lineLimit(1)
                .fixedSize()
                .frame(maxWidth: .infinity)
                .frame(height: 32 * scale)
                .background(buttonFill)
                .overlay(buttonStroke)
                .clipShape(Capsule())
                .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .disabled(disabled)
        .onHover { hovered = $0 }
    }

    private var buttonFill: some View {
        Capsule()
            .fill(
                prominent
                    ? Color.white.opacity(disabled ? 0.08 : hovered ? 0.24 : 0.18)
                    : Color.white.opacity(disabled ? 0.04 : hovered ? 0.14 : 0.09)
            )
    }

    private var buttonStroke: some View {
        Capsule()
            .stroke(Color.white.opacity(disabled ? 0.06 : hovered ? 0.24 : 0.13), lineWidth: 0.6)
    }
}

@available(macOS 13.0, *)
private struct PlayPauseButton: View {
    let isActive: Bool
    let scale: CGFloat
    let action: () -> Void
    @State private var hovered = false

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle()
                    .fill(
                        isActive
                            ? Color(red: 0.92, green: 0.12, blue: 0.09).opacity(hovered ? 1.0 : 0.92)
                            : Color.white.opacity(hovered ? 0.16 : 0.09)
                    )
                Circle()
                    .stroke(Color.white.opacity(isActive ? 0.22 : 0.10), lineWidth: 0.5)

                Image(systemName: isActive ? "pause.fill" : "play.fill")
                    .font(.system(size: 10 * scale, weight: .semibold))
                    .foregroundColor(isActive ? .white : (hovered ? .white.opacity(0.92) : .white.opacity(0.62)))
                    .offset(x: isActive ? 0 : 0.7 * scale)
            }
            .frame(width: 26 * scale, height: 26 * scale)
            .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .onHover { hovered = $0 }
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
        tracking.autoresizingMask = [.width, .height]
        panel.contentView = tracking
        trackingView = tracking
        self.panel = panel
    }

    private func positionPanel(preserveCurrentAnchor: Bool, animated: Bool) {
        guard let panel else { return }
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
                context.timingFunction = CAMediaTimingFunction(name: .easeOut)
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

    deinit {
        if let dragMonitor {
            NSEvent.removeMonitor(dragMonitor)
        }
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
