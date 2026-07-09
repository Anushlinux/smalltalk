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
var activeObservedPid: pid_t?
var activeObservedBundleId: String?
var unsupportedAxNotifications = Set<String>()
var cachedFrontmostContext: FrontmostContext?
var cachedFrontmostContextAtMs: Int64 = 0
var lastAxEmitByKey: [String: Int64] = [:]
var pendingScrollDx = 0.0
var pendingScrollDy = 0.0
var pendingScrollContext: FrontmostContext?
var scrollFlushTimer: Timer?
var lastScrollEmitAtMs: Int64 = 0

let smalltalkBundleId = "com.smalltalk.app"
let frontmostContextCacheMs: Int64 = 250
let scrollCoalesceMs: Int64 = 650
let axDefaultThrottleMs: Int64 = 900
let axValueThrottleMs: Int64 = 1800

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

func isSmalltalkBundle(_ bundleId: String?) -> Bool {
  bundleId == smalltalkBundleId
}

func isSmalltalkAppName(_ appName: String?) -> Bool {
  appName?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == "smalltalk"
}

func shouldSuppressContext(_ context: FrontmostContext) -> Bool {
  isSmalltalkBundle(context.app_bundle_id) || isSmalltalkAppName(context.app_name)
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

func frontmostContext(forceRefresh: Bool = false) -> FrontmostContext {
  let now = nowMs()
  if !forceRefresh,
     let cachedFrontmostContext,
     now - cachedFrontmostContextAtMs < frontmostContextCacheMs {
    return cachedFrontmostContext
  }

  let app = NSWorkspace.shared.frontmostApplication
  let pid = app?.processIdentifier
  let bundleId = clean(app?.bundleIdentifier)
  let appName = clean(app?.localizedName)
  let context = FrontmostContext(
    app_pid: pid.map(Int.init),
    app_bundle_id: bundleId,
    app_name: appName,
    window_title: isSmalltalkBundle(bundleId) ? nil : pid.flatMap { focusedWindowTitle(pid: $0) }
  )
  cachedFrontmostContext = context
  cachedFrontmostContextAtMs = now
  return context
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
  payload: [String: String]? = nil,
  context: FrontmostContext? = nil
) -> EventPayload {
  let ctx = context ?? frontmostContext()
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

func emitObserved(_ payload: EventPayload) {
  if isSmalltalkBundle(payload.app_bundle_id) || isSmalltalkAppName(payload.app_name) {
    return
  }
  emit(payload)
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
  guard shouldEmitAxNotification(name) else {
    return
  }
  emitObserved(baseEvent(
    type: "ax_notification",
    payload: [
      "notification": name
    ]
  ))
}

func axThrottleMs(for notification: String) -> Int64 {
  switch notification {
  case kAXValueChangedNotification,
       kAXSelectedTextChangedNotification,
       kAXWindowMovedNotification,
       kAXWindowResizedNotification:
    return axValueThrottleMs
  default:
    return axDefaultThrottleMs
  }
}

func shouldEmitAxNotification(_ notification: String) -> Bool {
  guard !isSmalltalkBundle(activeObservedBundleId) else {
    return false
  }
  let now = nowMs()
  let pidKey = activeObservedPid.map(String.init) ?? "unknown"
  let key = "\(pidKey):\(notification)"
  let throttleMs = axThrottleMs(for: notification)
  if let last = lastAxEmitByKey[key], now - last < throttleMs {
    return false
  }
  lastAxEmitByKey[key] = now
  if lastAxEmitByKey.count > 256 {
    lastAxEmitByKey = lastAxEmitByKey.filter { now - $0.value < 60_000 }
  }
  return true
}

func clearAxObserver() {
  if let source = activeObserverRunLoopSource {
    CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .defaultMode)
  }
  activeObserver = nil
  activeObserverRunLoopSource = nil
  activeObservedPid = nil
  activeObservedBundleId = nil
}

func installAxObserver(for app: NSRunningApplication) {
  clearAxObserver()

  if isSmalltalkBundle(clean(app.bundleIdentifier)) {
    return
  }

  let pid = app.processIdentifier

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
        emitObserved(baseEvent(
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
  activeObservedPid = pid
  activeObservedBundleId = clean(app.bundleIdentifier)
}

func flushPendingScroll() {
  scrollFlushTimer?.invalidate()
  scrollFlushTimer = nil

  guard let context = pendingScrollContext else {
    pendingScrollDx = 0
    pendingScrollDy = 0
    return
  }

  let dx = pendingScrollDx
  let dy = pendingScrollDy
  pendingScrollDx = 0
  pendingScrollDy = 0
  pendingScrollContext = nil

  if abs(dx) < 0.01 && abs(dy) < 0.01 {
    return
  }

  lastScrollEmitAtMs = nowMs()
  emitObserved(baseEvent(type: "scroll", scrollDx: dx, scrollDy: dy, context: context))
}

func scheduleScrollFlush(after delayMs: Int64) {
  if scrollFlushTimer != nil {
    return
  }
  let interval = max(0.05, TimeInterval(delayMs) / 1000.0)
  scrollFlushTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: false) { _ in
    flushPendingScroll()
  }
}

func handleScroll(dx: Double, dy: Double) {
  let context = frontmostContext()
  if shouldSuppressContext(context) {
    return
  }

  pendingScrollDx += dx
  pendingScrollDy += dy
  pendingScrollContext = context

  let now = nowMs()
  if lastScrollEmitAtMs == 0 || now - lastScrollEmitAtMs >= scrollCoalesceMs {
    flushPendingScroll()
    return
  }
  scheduleScrollFlush(after: scrollCoalesceMs - (now - lastScrollEmitAtMs))
}

NSWorkspace.shared.notificationCenter.addObserver(
  forName: NSWorkspace.didActivateApplicationNotification,
  object: nil,
  queue: .main
) { notification in
  if let app = notification.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication {
    cachedFrontmostContext = nil
    installAxObserver(for: app)
    if isSmalltalkBundle(clean(app.bundleIdentifier)) {
      return
    }
  }
  emitObserved(baseEvent(type: "app_switch", context: frontmostContext(forceRefresh: true)))
}

Timer.scheduledTimer(withTimeInterval: 0.75, repeats: true) { _ in
  let pasteboard = NSPasteboard.general
  if pasteboard.changeCount != lastClipboardChange {
    lastClipboardChange = pasteboard.changeCount
    var payload = clipboardMetadata()
    payload["change_count"] = "\(lastClipboardChange)"
    emitObserved(baseEvent(type: "clipboard", payload: payload))
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
    emitObserved(baseEvent(
      type: "click",
      x: location.x,
      y: location.y,
      button: buttonName(type)
    ))
  case .scrollWheel:
    let dx = Double(event.getIntegerValueField(.scrollWheelEventPointDeltaAxis2))
    let dy = Double(event.getIntegerValueField(.scrollWheelEventPointDeltaAxis1))
    handleScroll(dx: dx, dy: dy)
  case .keyDown:
    let keyCode = event.getIntegerValueField(.keyboardEventKeycode)
    let flags = event.flags
    emitObserved(baseEvent(
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
  installAxObserver(for: app)
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
