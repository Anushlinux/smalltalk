import AppKit
import ApplicationServices
import Foundation

struct RectPayload: Encodable {
  let x: Double
  let y: Double
  let w: Double
  let h: Double
}

struct AxNodePayload: Encodable {
  let local_id: String
  let parent_id: String?
  let role: String?
  let subrole: String?
  let role_description: String?
  let title: String?
  let value: String?
  let description: String?
  let help: String?
  let identifier: String?
  let document: String?
  let url: String?
  let selected_text: String?
  let selected_text_range: [String: Int]?
  let visible_character_range: [String: Int]?
  let number_of_characters: Int?
  let focused: Bool?
  let enabled: Bool?
  let selected: Bool?
  let bounds: RectPayload?
  let actions: [String]
  let children_count: Int
  let depth: Int
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.withoutEscapingSlashes]

func clean(_ value: String?) -> String? {
  guard let value else { return nil }
  let trimmed = value
    .replacingOccurrences(of: "\n", with: " ")
    .replacingOccurrences(of: "\t", with: " ")
    .trimmingCharacters(in: .whitespacesAndNewlines)
  return trimmed.isEmpty ? nil : trimmed
}

func printField(_ name: String, _ value: String?) {
  print("\(name)\t\(clean(value) ?? "")")
}

func attr(_ element: AXUIElement, _ name: String) -> AnyObject? {
  var value: AnyObject?
  guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else {
    return nil
  }
  return value
}

func stringAttr(_ element: AXUIElement, _ name: String) -> String? {
  if let value = attr(element, name) as? String {
    return clean(value)
  }
  if let value = attr(element, name) {
    return clean("\(value)")
  }
  return nil
}

func boolAttr(_ element: AXUIElement, _ name: String) -> Bool? {
  attr(element, name) as? Bool
}

func intAttr(_ element: AXUIElement, _ name: String) -> Int? {
  if let value = attr(element, name) as? Int { return value }
  if let number = attr(element, name) as? NSNumber { return number.intValue }
  return nil
}

func rangeAttr(_ element: AXUIElement, _ name: String) -> [String: Int]? {
  guard let value = attr(element, name) else { return nil }
  var range = CFRange(location: 0, length: 0)
  guard AXValueGetType(value as! AXValue) == .cfRange,
        AXValueGetValue(value as! AXValue, .cfRange, &range) else {
    return nil
  }
  return ["location": range.location, "length": range.length]
}

func pointAttr(_ element: AXUIElement, _ name: String) -> CGPoint? {
  guard let value = attr(element, name) else { return nil }
  var point = CGPoint.zero
  guard AXValueGetType(value as! AXValue) == .cgPoint,
        AXValueGetValue(value as! AXValue, .cgPoint, &point) else {
    return nil
  }
  return point
}

func sizeAttr(_ element: AXUIElement, _ name: String) -> CGSize? {
  guard let value = attr(element, name) else { return nil }
  var size = CGSize.zero
  guard AXValueGetType(value as! AXValue) == .cgSize,
        AXValueGetValue(value as! AXValue, .cgSize, &size) else {
    return nil
  }
  return size
}

func boundsFor(_ element: AXUIElement) -> RectPayload? {
  guard let point = pointAttr(element, kAXPositionAttribute),
        let size = sizeAttr(element, kAXSizeAttribute) else {
    return nil
  }
  return RectPayload(
    x: point.x,
    y: point.y,
    w: size.width,
    h: size.height
  )
}

func actionNames(_ element: AXUIElement) -> [String] {
  var names: CFArray?
  guard AXUIElementCopyActionNames(element, &names) == .success,
        let values = names as? [String] else {
    return []
  }
  return values
}

func children(_ element: AXUIElement) -> [AXUIElement] {
  guard let values = attr(element, kAXChildrenAttribute) as? [AXUIElement] else {
    return []
  }
  return values
}

func textForNode(_ node: AxNodePayload) -> String? {
  let pieces = [
    node.title,
    node.value,
    node.description,
    node.selected_text,
    node.document,
    node.url
  ].compactMap { $0 }
  var unique: [String] = []
  for piece in pieces where !unique.contains(piece) {
    unique.append(piece)
  }
  return clean(unique.joined(separator: " "))
}

func normalizedWebUrl(_ value: String?) -> String? {
  guard let value = clean(value),
        let components = URLComponents(string: value),
        let scheme = components.scheme?.lowercased(),
        scheme == "http" || scheme == "https" else {
    return nil
  }
  return value
}

func browserUrl(document: String?, nodes: [AxNodePayload]) -> String? {
  if let documentUrl = normalizedWebUrl(document) {
    return documentUrl
  }

  if let focusedUrl = nodes
    .filter({ $0.focused == true })
    .lazy
    .compactMap({ normalizedWebUrl($0.url) ?? normalizedWebUrl($0.value) })
    .first {
    return focusedUrl
  }

  return nodes.lazy.compactMap { node -> String? in
    let identity = [node.identifier, node.role_description, node.title]
      .compactMap { $0 }
      .joined(separator: " ")
      .lowercased()
    guard identity.contains("address") || identity.contains("location") || identity.contains("url") else {
      return nil
    }
    return normalizedWebUrl(node.url) ?? normalizedWebUrl(node.value)
  }.first
}

func collectNode(
  _ element: AXUIElement,
  id: String,
  parentId: String?,
  depth: Int,
  output: inout [AxNodePayload]
) {
  if depth > 8 || output.count > 450 {
    return
  }

  let childElements = children(element)
  let node = AxNodePayload(
    local_id: id,
    parent_id: parentId,
    role: stringAttr(element, kAXRoleAttribute),
    subrole: stringAttr(element, kAXSubroleAttribute),
    role_description: stringAttr(element, kAXRoleDescriptionAttribute),
    title: stringAttr(element, kAXTitleAttribute),
    value: stringAttr(element, kAXValueAttribute),
    description: stringAttr(element, kAXDescriptionAttribute),
    help: stringAttr(element, kAXHelpAttribute),
    identifier: stringAttr(element, kAXIdentifierAttribute),
    document: stringAttr(element, kAXDocumentAttribute),
    url: stringAttr(element, kAXURLAttribute),
    selected_text: stringAttr(element, kAXSelectedTextAttribute),
    selected_text_range: rangeAttr(element, kAXSelectedTextRangeAttribute),
    visible_character_range: rangeAttr(element, kAXVisibleCharacterRangeAttribute),
    number_of_characters: intAttr(element, kAXNumberOfCharactersAttribute),
    focused: boolAttr(element, kAXFocusedAttribute),
    enabled: boolAttr(element, kAXEnabledAttribute),
    selected: boolAttr(element, kAXSelectedAttribute),
    bounds: boundsFor(element),
    actions: actionNames(element),
    children_count: childElements.count,
    depth: depth
  )

  output.append(node)

  for (index, child) in childElements.enumerated() {
    collectNode(
      child,
      id: "\(id).\(index)",
      parentId: id,
      depth: depth + 1,
      output: &output
    )
  }
}

guard let frontApp = NSWorkspace.shared.frontmostApplication else {
  print("ERROR\tNo frontmost application")
  exit(1)
}

let pid = frontApp.processIdentifier
let appElement = AXUIElementCreateApplication(pid)
var focusedWindowObject: AnyObject?
let focusedWindowResult = AXUIElementCopyAttributeValue(
  appElement,
  kAXFocusedWindowAttribute as CFString,
  &focusedWindowObject
)

let focusedWindow = focusedWindowObject as! AXUIElement?
let windowTitle = focusedWindow.flatMap { stringAttr($0, kAXTitleAttribute) }
let document = focusedWindow.flatMap { stringAttr($0, kAXDocumentAttribute) }
let windowNumber = focusedWindow.flatMap { intAttr($0, "AXWindowNumber") }

var nodes: [AxNodePayload] = []
if focusedWindowResult == .success, let focusedWindow {
  collectNode(focusedWindow, id: "root", parentId: nil, depth: 0, output: &nodes)
}

printField("APP", frontApp.localizedName)
printField("APP_PID", "\(pid)")
printField("APP_BUNDLE_ID", frontApp.bundleIdentifier)
printField("WINDOW", windowTitle)
printField("WINDOW_ID", windowNumber.map(String.init))
printField("BROWSER_URL", browserUrl(document: document, nodes: nodes))
printField("DOCUMENT", document)

guard focusedWindowResult == .success, focusedWindow != nil else {
  print("ERROR\tFocused window unavailable")
  exit(0)
}

for node in nodes {
  if let data = try? encoder.encode(node),
     let json = String(data: data, encoding: .utf8) {
    print("NODE_JSON\t\(json)")
  }

  if let text = textForNode(node), !text.isEmpty {
    print("NODE\t\(node.depth)\t\(node.role ?? "")\t\(text)")
  }
}
