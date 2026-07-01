import AppKit
import CoreGraphics
import Foundation

struct RectPayload: Encodable {
  let x: Double
  let y: Double
  let w: Double
  let h: Double
}

struct WindowPayload: Encodable {
  let cg_window_id: Int?
  let owner_pid: Int?
  let owner_name: String?
  let bundle_id: String?
  let window_title: String?
  let layer: Int?
  let alpha: Double?
  let is_onscreen: Bool?
  let is_active: Bool
  let bounds: RectPayload?
  let workspace: Int?
  let raw: [String: String]
}

struct SnapshotPayload: Encodable {
  let ts_ms: Int64
  let active_window_id: Int?
  let active_app_pid: Int?
  let active_app_bundle_id: String?
  let screen_count: Int
  let windows: [WindowPayload]
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.withoutEscapingSlashes]

func nowMs() -> Int64 {
  Int64(Date().timeIntervalSince1970 * 1000)
}

func clean(_ value: String?) -> String? {
  guard let value else { return nil }
  let trimmed = value
    .replacingOccurrences(of: "\n", with: " ")
    .replacingOccurrences(of: "\t", with: " ")
    .trimmingCharacters(in: .whitespacesAndNewlines)
  return trimmed.isEmpty ? nil : trimmed
}

func bundleId(for pid: Int?) -> String? {
  guard let pid else { return nil }
  return NSRunningApplication(processIdentifier: pid_t(pid))?.bundleIdentifier
}

func focusedWindowTitle(pid: pid_t) -> String? {
  let app = AXUIElementCreateApplication(pid)
  var focusedWindow: AnyObject?
  guard AXUIElementCopyAttributeValue(
    app,
    kAXFocusedWindowAttribute as CFString,
    &focusedWindow
  ) == .success else {
    return nil
  }
  var title: AnyObject?
  guard AXUIElementCopyAttributeValue(
    focusedWindow as! AXUIElement,
    kAXTitleAttribute as CFString,
    &title
  ) == .success else {
    return nil
  }
  return clean(title as? String)
}

func stringValue(_ value: Any?) -> String? {
  if let value = value as? String { return clean(value) }
  if let value { return clean("\(value)") }
  return nil
}

let activeApp = NSWorkspace.shared.frontmostApplication
let activePid = activeApp.map { Int($0.processIdentifier) }
let activeBundleId = clean(activeApp?.bundleIdentifier)
let activeTitle = activeApp.flatMap { focusedWindowTitle(pid: $0.processIdentifier) }

let rawWindows = CGWindowListCopyWindowInfo(
  [.optionOnScreenOnly, .excludeDesktopElements],
  kCGNullWindowID
) as? [[String: Any]] ?? []

var windows: [WindowPayload] = []
var activeWindowId: Int?

for entry in rawWindows {
  let windowId = entry[kCGWindowNumber as String] as? Int
  let ownerPid = entry[kCGWindowOwnerPID as String] as? Int
  let ownerName = clean(entry[kCGWindowOwnerName as String] as? String)
  let title = clean(entry[kCGWindowName as String] as? String)
  let layer = entry[kCGWindowLayer as String] as? Int
  let alpha = entry[kCGWindowAlpha as String] as? Double
  let isOnscreen = entry[kCGWindowIsOnscreen as String] as? Bool
  let workspace = entry["kCGWindowWorkspace"] as? Int

  var bounds: RectPayload?
  if let dict = entry[kCGWindowBounds as String] as? [String: Any],
     let x = dict["X"] as? Double,
     let y = dict["Y"] as? Double,
     let w = dict["Width"] as? Double,
     let h = dict["Height"] as? Double {
    bounds = RectPayload(x: x, y: y, w: w, h: h)
  }

  let pidMatches = ownerPid != nil && activePid != nil && ownerPid == activePid
  let titleMatches = activeTitle == nil || title == nil || title == activeTitle
  let active = activeWindowId == nil && pidMatches && layer == 0 && titleMatches
  if active {
    activeWindowId = windowId
  }

  var raw: [String: String] = [:]
  for (key, value) in entry {
    raw[key] = stringValue(value)
  }

  windows.append(WindowPayload(
    cg_window_id: windowId,
    owner_pid: ownerPid,
    owner_name: ownerName,
    bundle_id: clean(bundleId(for: ownerPid)),
    window_title: title,
    layer: layer,
    alpha: alpha,
    is_onscreen: isOnscreen,
    is_active: active,
    bounds: bounds,
    workspace: workspace,
    raw: raw
  ))
}

let payload = SnapshotPayload(
  ts_ms: nowMs(),
  active_window_id: activeWindowId,
  active_app_pid: activePid,
  active_app_bundle_id: activeBundleId,
  screen_count: NSScreen.screens.count,
  windows: windows
)

if let data = try? encoder.encode(payload),
   let json = String(data: data, encoding: .utf8) {
  print(json)
} else {
  print("{\"ts_ms\":0,\"screen_count\":0,\"windows\":[]}")
  exit(1)
}
