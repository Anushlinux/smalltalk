import AppKit
import Foundation
import Vision

struct OcrElement: Encodable {
  let text: String
  let confidence: Float
  let left: Double
  let top: Double
  let width: Double
  let height: Double
}

struct OcrPayload: Encodable {
  let text: String
  let text_elements: [OcrElement]
  let confidence: Double
  let error: String?
}

func emit(_ payload: OcrPayload) {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.withoutEscapingSlashes]
  if let data = try? encoder.encode(payload),
     let string = String(data: data, encoding: .utf8) {
    print(string)
  } else {
    print("{\"text\":\"\",\"text_elements\":[],\"confidence\":0,\"error\":\"encode_failed\"}")
  }
}

guard CommandLine.arguments.count >= 2 else {
  emit(OcrPayload(text: "", text_elements: [], confidence: 0, error: "missing_image_path"))
  exit(2)
}

let imageURL = URL(fileURLWithPath: CommandLine.arguments[1])
guard let image = NSImage(contentsOf: imageURL) else {
  emit(OcrPayload(text: "", text_elements: [], confidence: 0, error: "image_load_failed"))
  exit(3)
}

var rect = NSRect(origin: .zero, size: image.size)
guard let cgImage = image.cgImage(forProposedRect: &rect, context: nil, hints: nil) else {
  emit(OcrPayload(text: "", text_elements: [], confidence: 0, error: "cgimage_create_failed"))
  exit(4)
}

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = false
request.recognitionLanguages = ["en-US"]

let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

do {
  try handler.perform([request])
} catch {
  emit(OcrPayload(text: "", text_elements: [], confidence: 0, error: "vision_failed: \(error.localizedDescription)"))
  exit(5)
}

let observations = request.results ?? []
var lines: [String] = []
var elements: [OcrElement] = []
var confidenceTotal = 0.0
var confidenceCount = 0.0

for observation in observations {
  guard let candidate = observation.topCandidates(1).first else { continue }
  let line = candidate.string.trimmingCharacters(in: .whitespacesAndNewlines)
  if line.isEmpty { continue }

  lines.append(line)
  confidenceTotal += Double(candidate.confidence)
  confidenceCount += 1

  let box = observation.boundingBox
  elements.append(
    OcrElement(
      text: line,
      confidence: candidate.confidence,
      left: Double(box.origin.x),
      top: Double(1.0 - box.origin.y - box.size.height),
      width: Double(box.size.width),
      height: Double(box.size.height)
    )
  )
}

emit(
  OcrPayload(
    text: lines.joined(separator: "\n"),
    text_elements: elements,
    confidence: confidenceCount > 0 ? confidenceTotal / confidenceCount : 0,
    error: nil
  )
)
