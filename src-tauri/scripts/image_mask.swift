import AppKit
import Foundation

struct MaskRect: Decodable {
    let x: Double
    let y: Double
    let w: Double
    let h: Double
}

let args = CommandLine.arguments
guard args.count >= 4 else {
    fputs("usage: image_mask <input> <output> <rects-json>\n", stderr)
    exit(2)
}

let input = URL(fileURLWithPath: args[1])
let output = URL(fileURLWithPath: args[2])
let rectData = args[3].data(using: .utf8) ?? Data()
let rects = (try? JSONDecoder().decode([MaskRect].self, from: rectData)) ?? []

guard let image = NSImage(contentsOf: input) else {
    fputs("could not load input image\n", stderr)
    exit(3)
}

let size = image.size
guard let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: max(1, Int(size.width.rounded())),
    pixelsHigh: max(1, Int(size.height.rounded())),
    bitsPerSample: 8,
    samplesPerPixel: 4,
    hasAlpha: true,
    isPlanar: false,
    colorSpaceName: .deviceRGB,
    bytesPerRow: 0,
    bitsPerPixel: 0
) else {
    fputs("could not create bitmap\n", stderr)
    exit(4)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
image.draw(in: NSRect(origin: .zero, size: size))
NSColor.black.setFill()
for rect in rects {
    let x = max(0.0, min(rect.x, size.width))
    let y = max(0.0, min(rect.y, size.height))
    let w = max(0.0, min(rect.w, size.width - x))
    let h = max(0.0, min(rect.h, size.height - y))
    NSBezierPath(rect: NSRect(x: x, y: size.height - y - h, width: w, height: h)).fill()
}
NSGraphicsContext.restoreGraphicsState()

guard let png = bitmap.representation(using: .png, properties: [:]) else {
    fputs("could not encode png\n", stderr)
    exit(5)
}

try FileManager.default.createDirectory(
    at: output.deletingLastPathComponent(),
    withIntermediateDirectories: true
)
try png.write(to: output)
