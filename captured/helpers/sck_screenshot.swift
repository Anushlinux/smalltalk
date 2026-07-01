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
  let captured_window_id: Int?
  let captured_bundle_id: String?
  let width: Int?
  let height: Int?
  let bytes: Int?
  let output_path: String
  let fallback_used: Bool
  let error: String?
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.withoutEscapingSlashes]

func emit(_ response: ScreenshotResponse) {
  guard let data = try? encoder.encode(response) else {
    print("{\"ok\":false,\"provider\":\"screencapturekit\",\"mode\":\"unknown\",\"output_path\":\"\",\"fallback_used\":false,\"error\":\"failed to encode response\"}")
    return
  }
  FileHandle.standardOutput.write(data)
  FileHandle.standardOutput.write(Data("\n".utf8))
}

func fail(mode: String, outputPath: String, _ message: String) -> Never {
  emit(ScreenshotResponse(
    ok: false,
    provider: "screencapturekit",
    mode: mode,
    captured_window_id: nil,
    captured_bundle_id: nil,
    width: nil,
    height: nil,
    bytes: nil,
    output_path: outputPath,
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
  let mode = request.mode ?? "active_window"
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
  var capturedWindowId: Int?
  var capturedBundleId: String?

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
    capturedBundleId = app.bundleIdentifier

  default:
    guard let display = content.displays.first else {
      throw NSError(domain: "SmalltalkSCK", code: 7, userInfo: [
        NSLocalizedDescriptionKey: "no display available for display capture"
      ])
    }
    filter = SCContentFilter(display: display, excludingWindows: excludedWindows)
    configuration = makeConfiguration(width: display.width, height: display.height, request: request)
  }

  let image = try await SCScreenshotManager.captureImage(
    contentFilter: filter,
    configuration: configuration
  )
  let written = try writeJpeg(image, request: request)
  return ScreenshotResponse(
    ok: true,
    provider: "screencapturekit",
    mode: mode,
    captured_window_id: capturedWindowId,
    captured_bundle_id: capturedBundleId,
    width: written.0,
    height: written.1,
    bytes: written.2,
    output_path: request.output_path,
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
