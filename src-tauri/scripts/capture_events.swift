import AppKit
import ApplicationServices
import Foundation

struct FrontmostContext: Encodable {
  let app_pid: Int?
  let app_bundle_id: String?
  let app_name: String?
  let window_title: String?
}

struct EventPayload: Encodable {
  let ts_ms: Int64
  let event_type: String
  let app_pid: Int?
  let app_bundle_id: String?
  let app_name: String?
  let window_title: String?
  let x: Double?
  let y: Double?
  let button: String?
  let scroll_dx: Double?
  let scroll_dy: Double?
  let key_category: String?
  let modifier_flags: String?
  let is_repeat: Bool?
  let payload: [String: String]?
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.withoutEscapingSlashes]

var lastClipboardChange = NSPasteboard.general.changeCount
var activeObserver: AXObserver?
var activeObserverRunLoopSource: CFRunLoopSource?
var unsupportedAxNotifications = Set<String>()

func nowMs() -> Int64 {
  Int64(Date().timeIntervalSince1970 * 1000)
}

func emit(_ payload: EventPayload) {
  guard let data = try? encoder.encode(payload),
        let line = String(data: data, encoding: .utf8) else {
    return
  }
  print(line)
  fflush(stdout)
}

func clean(_ value: String?) -> String? {
  guard let value else { return nil }
  let trimmed = value
    .replacingOccurrences(of: "\n", with: " ")
    .replacingOccurrences(of: "\t", with: " ")
    .trimmingCharacters(in: .whitespacesAndNewlines)
  return trimmed.isEmpty ? nil : trimmed
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

func frontmostContext() -> FrontmostContext {
  let app = NSWorkspace.shared.frontmostApplication
  let pid = app?.processIdentifier
  return FrontmostContext(
    app_pid: pid.map(Int.init),
    app_bundle_id: clean(app?.bundleIdentifier),
    app_name: clean(app?.localizedName),
    window_title: pid.flatMap { focusedWindowTitle(pid: $0) }
  )
}

func baseEvent(
  type: String,
  x: Double? = nil,
  y: Double? = nil,
  button: String? = nil,
  scrollDx: Double? = nil,
  scrollDy: Double? = nil,
  keyCategory: String? = nil,
  modifierFlags: String? = nil,
  isRepeat: Bool? = nil,
  payload: [String: String]? = nil
) -> EventPayload {
  let ctx = frontmostContext()
  return EventPayload(
    ts_ms: nowMs(),
    event_type: type,
    app_pid: ctx.app_pid,
    app_bundle_id: ctx.app_bundle_id,
    app_name: ctx.app_name,
    window_title: ctx.window_title,
    x: x,
    y: y,
    button: button,
    scroll_dx: scrollDx,
    scroll_dy: scrollDy,
    key_category: keyCategory,
    modifier_flags: modifierFlags,
    is_repeat: isRepeat,
    payload: payload
  )
}

func modifierDescription(_ flags: CGEventFlags) -> String? {
  var parts: [String] = []
  if flags.contains(.maskCommand) { parts.append("cmd") }
  if flags.contains(.maskShift) { parts.append("shift") }
  if flags.contains(.maskAlternate) { parts.append("option") }
  if flags.contains(.maskControl) { parts.append("control") }
  if flags.contains(.maskSecondaryFn) { parts.append("fn") }
  return parts.isEmpty ? nil : parts.joined(separator: "+")
}

func keyCategory(keyCode: Int64, flags: CGEventFlags) -> String {
  if flags.intersection([.maskCommand, .maskAlternate, .maskControl]).isEmpty == false {
    return "shortcut"
  }

  switch keyCode {
  case 36, 76:
    return "enter"
  case 48, 49:
    return "modifier"
  case 51, 117:
    return "backspace"
  case 53:
    return "escape"
  case 123, 124, 125, 126:
    return "arrow"
  default:
    return "char"
  }
}

func buttonName(_ type: CGEventType) -> String? {
  switch type {
  case .leftMouseDown:
    return "left"
  case .rightMouseDown:
    return "right"
  case .otherMouseDown:
    return "other"
  default:
    return nil
  }
}

func fnvHash(_ input: String) -> String {
  var hash: UInt64 = 0xcbf29ce484222325
  for byte in input.utf8 {
    hash ^= UInt64(byte)
    hash = hash &* 0x100000001b3
  }
  return String(format: "%016llx", hash)
}

func redactedPreview(_ input: String) -> String {
  let collapsed = input
    .replacingOccurrences(of: #"\s+"#, with: " ", options: .regularExpression)
    .trimmingCharacters(in: .whitespacesAndNewlines)
  let clipped = String(collapsed.prefix(96))
  return clipped
    .replacingOccurrences(
      of: #"[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}"#,
      with: "[email]",
      options: [.regularExpression, .caseInsensitive]
    )
    .replacingOccurrences(
      of: #"\b\d{4,}\b"#,
      with: "[number]",
      options: .regularExpression
    )
}

func clipboardMetadata() -> [String: String] {
  let pasteboard = NSPasteboard.general
  let types = pasteboard.types?.map { $0.rawValue }.joined(separator: ",") ?? ""

  if let text = pasteboard.string(forType: .string) {
    return [
      "content_type": "text",
      "text_hash": fnvHash(text),
      "redacted_preview": redactedPreview(text),
      "byte_size": "\(text.lengthOfBytes(using: .utf8))",
      "types": types
    ]
  }

  if pasteboard.canReadObject(forClasses: [NSURL.self], options: nil) {
    return ["content_type": "file_url", "types": types]
  }

  if types.contains("public.tiff") || types.contains("public.png") {
    return ["content_type": "image", "types": types]
  }

  return ["content_type": types.isEmpty ? "unknown" : "rich_text", "types": types]
}

let axCallback: AXObserverCallback = { _, element, notification, _ in
  let name = notification as String
  emit(baseEvent(
    type: "ax_notification",
    payload: [
      "notification": name,
      "element": "\(element)"
    ]
  ))
}

func installAxObserver(for pid: pid_t) {
  if let source = activeObserverRunLoopSource {
    CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .defaultMode)
  }
  activeObserver = nil
  activeObserverRunLoopSource = nil

  var observer: AXObserver?
  guard AXObserverCreate(pid, axCallback, &observer) == .success, let observer else {
    return
  }

  let appElement = AXUIElementCreateApplication(pid)
  let notifications = [
    kAXFocusedWindowChangedNotification,
    kAXFocusedUIElementChangedNotification,
    kAXValueChangedNotification,
    kAXSelectedTextChangedNotification,
    kAXWindowCreatedNotification,
    kAXWindowMovedNotification,
    kAXWindowResizedNotification,
    kAXTitleChangedNotification
  ]

  for notification in notifications {
    let result = AXObserverAddNotification(
      observer,
      appElement,
      notification as CFString,
      nil
    )
    if result != .success {
      let key = "\(pid):\(notification)"
      if unsupportedAxNotifications.insert(key).inserted {
        emit(baseEvent(
          type: "ax_notification",
          payload: [
            "notification": notification,
            "unsupported": "\(result.rawValue)"
          ]
        ))
      }
    }
  }

  let source = AXObserverGetRunLoopSource(observer)
  CFRunLoopAddSource(CFRunLoopGetMain(), source, .defaultMode)
  activeObserver = observer
  activeObserverRunLoopSource = source
}

NSWorkspace.shared.notificationCenter.addObserver(
  forName: NSWorkspace.didActivateApplicationNotification,
  object: nil,
  queue: .main
) { notification in
  if let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication {
    installAxObserver(for: app.processIdentifier)
  }
  emit(baseEvent(type: "app_switch"))
}

Timer.scheduledTimer(withTimeInterval: 0.75, repeats: true) { _ in
  let pasteboard = NSPasteboard.general
  if pasteboard.changeCount != lastClipboardChange {
    lastClipboardChange = pasteboard.changeCount
    var payload = clipboardMetadata()
    payload["change_count"] = "\(lastClipboardChange)"
    emit(baseEvent(type: "clipboard", payload: payload))
  }
}

let eventMask =
  (1 << CGEventType.leftMouseDown.rawValue) |
  (1 << CGEventType.rightMouseDown.rawValue) |
  (1 << CGEventType.otherMouseDown.rawValue) |
  (1 << CGEventType.keyDown.rawValue) |
  (1 << CGEventType.scrollWheel.rawValue)

let callback: CGEventTapCallBack = { _, type, event, _ in
  switch type {
  case .leftMouseDown, .rightMouseDown, .otherMouseDown:
    let location = event.location
    emit(baseEvent(
      type: "click",
      x: location.x,
      y: location.y,
      button: buttonName(type)
    ))
  case .scrollWheel:
    let dx = Double(event.getIntegerValueField(.scrollWheelEventPointDeltaAxis2))
    let dy = Double(event.getIntegerValueField(.scrollWheelEventPointDeltaAxis1))
    emit(baseEvent(type: "scroll", scrollDx: dx, scrollDy: dy))
  case .keyDown:
    let keyCode = event.getIntegerValueField(.keyboardEventKeycode)
    let flags = event.flags
    emit(baseEvent(
      type: "key_down",
      keyCategory: keyCategory(keyCode: keyCode, flags: flags),
      modifierFlags: modifierDescription(flags),
      isRepeat: event.getIntegerValueField(.keyboardEventAutorepeat) != 0
    ))
  default:
    break
  }

  return Unmanaged.passUnretained(event)
}

if let app = NSWorkspace.shared.frontmostApplication {
  installAxObserver(for: app.processIdentifier)
}

guard let eventTap = CGEvent.tapCreate(
  tap: .cgSessionEventTap,
  place: .headInsertEventTap,
  options: .listenOnly,
  eventsOfInterest: CGEventMask(eventMask),
  callback: callback,
  userInfo: nil
) else {
  emit(baseEvent(type: "error", payload: ["message": "event_tap_unavailable"]))
  RunLoop.main.run()
  exit(1)
}

let runLoopSource = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, eventTap, 0)
CFRunLoopAddSource(CFRunLoopGetMain(), runLoopSource, .commonModes)
CGEvent.tapEnable(tap: eventTap, enable: true)

emit(baseEvent(type: "helper_started"))
RunLoop.main.run()
