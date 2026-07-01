import AppKit
import Dispatch
import Foundation
import ScreenCaptureKit

struct ScreenshotRequest: Decodable {
  let mode: String?
  let target_window_id: Int?
  let target_bundle_id: String?
  let exclude_bundle_ids: [String]?
  let output_path: String
  let max_width: Int?
  let quality: Double?
  let include_cursor: Bool?
}

struct ScreenshotResponse: Encodable {
  let ok: Bool
  let provider: String
  let mode: String
  let captured_display_id: String?
  let captured_window_id: Int?
  let captured_bundle_id: String?
  let width: Int?
  let height: Int?
  let bytes: Int?
  let output_path: String
  let filter_summary_json: String?
  let configuration_summary_json: String?
  let frame_metadata_json: String?
  let capture_mode: String
  let audio_policy: String
  let fallback_used: Bool
  let error: String?
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.withoutEscapingSlashes]

func emit(_ response: ScreenshotResponse) {
  guard let data = try? encoder.encode(response) else {
    print("{\"ok\":false,\"provider\":\"screen_capture_kit\",\"mode\":\"unknown\",\"output_path\":\"\",\"capture_mode\":\"screenshot\",\"audio_policy\":\"disabled\",\"fallback_used\":false,\"error\":\"failed to encode response\"}")
    return
  }
  FileHandle.standardOutput.write(data)
  FileHandle.standardOutput.write(Data("\n".utf8))
}

func jsonString(_ value: [String: Any?]) -> String? {
  let cleaned = value.compactMapValues { $0 }
  guard JSONSerialization.isValidJSONObject(cleaned),
        let data = try? JSONSerialization.data(withJSONObject: cleaned, options: [.sortedKeys]),
        let json = String(data: data, encoding: .utf8) else {
    return nil
  }
  return json
}

func fail(mode: String, outputPath: String, _ message: String) -> Never {
  emit(ScreenshotResponse(
    ok: false,
    provider: "screen_capture_kit",
    mode: mode,
    captured_display_id: nil,
    captured_window_id: nil,
    captured_bundle_id: nil,
    width: nil,
    height: nil,
    bytes: nil,
    output_path: outputPath,
    filter_summary_json: nil,
    configuration_summary_json: nil,
    frame_metadata_json: nil,
    capture_mode: "screenshot",
    audio_policy: "disabled",
    fallback_used: false,
    error: message
  ))
  exit(1)
}

@available(macOS 14.0, *)
func sourceSize(width: Int, height: Int, maxWidth: Int?) -> (Int, Int) {
  let safeWidth = max(width, 1)
  let safeHeight = max(height, 1)
  guard let maxWidth, maxWidth > 0, safeWidth > maxWidth else {
    return (safeWidth, safeHeight)
  }
  let ratio = Double(maxWidth) / Double(safeWidth)
  return (max(1, Int(Double(safeWidth) * ratio)), max(1, Int(Double(safeHeight) * ratio)))
}

@available(macOS 14.0, *)
func makeConfiguration(width: Int, height: Int, request: ScreenshotRequest) -> SCStreamConfiguration {
  let size = sourceSize(width: width, height: height, maxWidth: request.max_width)
  let configuration = SCStreamConfiguration()
  configuration.width = size.0
  configuration.height = size.1
  configuration.showsCursor = request.include_cursor ?? false
  configuration.capturesAudio = false
  return configuration
}

@available(macOS 14.0, *)
func configurationSummary(_ configuration: SCStreamConfiguration, request: ScreenshotRequest) -> String? {
  jsonString([
    "profile": request.mode == "active_window" ? "active_window_still" : "evidence_still",
    "width": configuration.width,
    "height": configuration.height,
    "shows_cursor": configuration.showsCursor,
    "captures_audio": configuration.capturesAudio,
    "quality": request.quality ?? 0.82,
    "max_width": request.max_width
  ])
}

@available(macOS 14.0, *)
func writeJpeg(_ image: CGImage, request: ScreenshotRequest) throws -> (Int, Int, Int) {
  let outputURL = URL(fileURLWithPath: request.output_path)
  try FileManager.default.createDirectory(
    at: outputURL.deletingLastPathComponent(),
    withIntermediateDirectories: true
  )

  let bitmap = NSBitmapImageRep(cgImage: image)
  let quality = min(max(request.quality ?? 0.82, 0.1), 1.0)
  guard let data = bitmap.representation(
    using: .jpeg,
    properties: [.compressionFactor: quality]
  ) else {
    throw NSError(domain: "SmalltalkSCK", code: 2, userInfo: [
      NSLocalizedDescriptionKey: "failed to encode JPEG"
    ])
  }
  try data.write(to: outputURL, options: .atomic)
  return (image.width, image.height, data.count)
}

@available(macOS 14.0, *)
func runCapture(_ request: ScreenshotRequest) async throws -> ScreenshotResponse {
  let mode = request.mode ?? "display"
  let excludedBundleIds = Set((request.exclude_bundle_ids ?? []).filter { !$0.isEmpty })
  let content = try await SCShareableContent.excludingDesktopWindows(
    false,
    onScreenWindowsOnly: true
  )
  let excludedWindows = content.windows.filter { window in
    guard let bundleId = window.owningApplication?.bundleIdentifier else { return false }
    return excludedBundleIds.contains(bundleId)
  }

  let filter: SCContentFilter
  let configuration: SCStreamConfiguration
  var capturedDisplayId: String?
  var capturedWindowId: Int?
  var capturedBundleId: String?
  var filterScope = "display"

  switch mode {
  case "active_window":
    let window: SCWindow?
    if let targetWindowId = request.target_window_id {
      window = content.windows.first { Int($0.windowID) == targetWindowId }
    } else if let bundleId = request.target_bundle_id {
      window = content.windows.first {
        $0.isOnScreen
          && $0.windowLayer == 0
          && $0.owningApplication?.bundleIdentifier == bundleId
      }
    } else {
      let frontmostBundleId = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
      window = content.windows.first {
        $0.isOnScreen
          && $0.windowLayer == 0
          && $0.owningApplication?.bundleIdentifier == frontmostBundleId
      }
    }

    guard let selectedWindow = window else {
      throw NSError(domain: "SmalltalkSCK", code: 3, userInfo: [
        NSLocalizedDescriptionKey: "active window was not present in ScreenCaptureKit shareable content"
      ])
    }
    if let bundleId = selectedWindow.owningApplication?.bundleIdentifier,
       excludedBundleIds.contains(bundleId) {
      throw NSError(domain: "SmalltalkSCK", code: 4, userInfo: [
        NSLocalizedDescriptionKey: "active window belongs to an excluded application"
      ])
    }

    filter = SCContentFilter(desktopIndependentWindow: selectedWindow)
    configuration = makeConfiguration(
      width: Int(selectedWindow.frame.width.rounded()),
      height: Int(selectedWindow.frame.height.rounded()),
      request: request
    )
    capturedWindowId = Int(selectedWindow.windowID)
    capturedBundleId = selectedWindow.owningApplication?.bundleIdentifier
    filterScope = "window"

  case "app_filtered":
    guard let display = content.displays.first else {
      throw NSError(domain: "SmalltalkSCK", code: 5, userInfo: [
        NSLocalizedDescriptionKey: "no display available for app-filtered capture"
      ])
    }
    guard let targetBundleId = request.target_bundle_id,
          let app = content.applications.first(where: { $0.bundleIdentifier == targetBundleId }) else {
      throw NSError(domain: "SmalltalkSCK", code: 6, userInfo: [
        NSLocalizedDescriptionKey: "target application was not present in ScreenCaptureKit shareable content"
      ])
    }
    filter = SCContentFilter(display: display, including: [app], exceptingWindows: excludedWindows)
    configuration = makeConfiguration(width: display.width, height: display.height, request: request)
    capturedDisplayId = String(display.displayID)
    capturedBundleId = app.bundleIdentifier
    filterScope = "application"

  default:
    guard let display = content.displays.first else {
      throw NSError(domain: "SmalltalkSCK", code: 7, userInfo: [
        NSLocalizedDescriptionKey: "no display available for display capture"
      ])
    }
    filter = SCContentFilter(display: display, excludingWindows: excludedWindows)
    configuration = makeConfiguration(width: display.width, height: display.height, request: request)
    capturedDisplayId = String(display.displayID)
  }

  let image = try await SCScreenshotManager.captureImage(
    contentFilter: filter,
    configuration: configuration
  )
  let written = try writeJpeg(image, request: request)
  return ScreenshotResponse(
    ok: true,
    provider: "screen_capture_kit",
    mode: mode,
    captured_display_id: capturedDisplayId,
    captured_window_id: capturedWindowId,
    captured_bundle_id: capturedBundleId,
    width: written.0,
    height: written.1,
    bytes: written.2,
    output_path: request.output_path,
    filter_summary_json: jsonString([
      "scope": filterScope,
      "excluded_bundle_ids": Array(excludedBundleIds).sorted(),
      "excluded_window_ids": excludedWindows.map { Int($0.windowID) },
      "self_exclusion_requested": excludedBundleIds.contains(Bundle.main.bundleIdentifier ?? "")
    ]),
    configuration_summary_json: configurationSummary(configuration, request: request),
    frame_metadata_json: jsonString([
      "content_rect": [
        "x": 0,
        "y": 0,
        "w": written.0,
        "h": written.1
      ],
      "content_scale": 1,
      "scale_factor": NSScreen.main?.backingScaleFactor ?? 1,
      "status": "complete"
    ]),
    capture_mode: "screenshot",
    audio_policy: "disabled",
    fallback_used: false,
    error: nil
  )
}

Task {
  let rawRequest = CommandLine.arguments.dropFirst().joined(separator: " ")
  guard !rawRequest.isEmpty else {
    fail(mode: "unknown", outputPath: "", "missing JSON request")
  }
  guard let data = rawRequest.data(using: .utf8) else {
    fail(mode: "unknown", outputPath: "", "request was not valid UTF-8")
  }
  let request: ScreenshotRequest
  do {
    request = try JSONDecoder().decode(ScreenshotRequest.self, from: data)
  } catch {
    fail(mode: "unknown", outputPath: "", "failed to decode request: \(error)")
  }

  guard #available(macOS 14.0, *) else {
    fail(
      mode: request.mode ?? "unknown",
      outputPath: request.output_path,
      "ScreenCaptureKit one-shot screenshots require macOS 14 or newer"
    )
  }

  do {
    let response = try await runCapture(request)
    emit(response)
    exit(0)
  } catch {
    fail(
      mode: request.mode ?? "unknown",
      outputPath: request.output_path,
      error.localizedDescription
    )
  }
}

dispatchMain()
