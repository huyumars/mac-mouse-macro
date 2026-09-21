import AppKit

// Vector artwork in a 1024-point canvas; render each icon size independently.
let output = CommandLine.arguments.dropFirst().first ?? "target/AppIcon.iconset"
try FileManager.default.createDirectory(atPath: output, withIntermediateDirectories: true)
for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = size * scale
        let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
                                      bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                                      isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
        let ctx = NSGraphicsContext.current!.cgContext
        ctx.scaleBy(x: CGFloat(pixels) / 1024, y: CGFloat(pixels) / 1024)
        let tile = NSBezierPath(roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 200, yRadius: 200)
        NSGradient(starting: NSColor(calibratedRed: 0.17, green: 0.22, blue: 0.27, alpha: 1),
                   ending: NSColor(calibratedRed: 0.045, green: 0.07, blue: 0.10, alpha: 1))!.draw(in: tile, angle: -90)
        NSColor(white: 1, alpha: 0.12).setStroke()
        tile.lineWidth = 3
        tile.stroke()

        // A mouse silhouette with a lightning-shaped macro mark.
        let mouse = NSBezierPath(roundedRect: NSRect(x: 310, y: 194, width: 404, height: 640), xRadius: 202, yRadius: 202)
        NSColor(calibratedRed: 0.33, green: 0.88, blue: 0.92, alpha: 1).setFill()
        mouse.fill()
        let ink = NSColor(calibratedRed: 0.055, green: 0.115, blue: 0.15, alpha: 1)
        ink.setFill()
        NSBezierPath(roundedRect: NSRect(x: 493, y: 676, width: 38, height: 94), xRadius: 19, yRadius: 19).fill()
        let bolt = NSBezierPath()
        bolt.move(to: NSPoint(x: 529, y: 624))
        bolt.line(to: NSPoint(x: 414, y: 437))
        bolt.line(to: NSPoint(x: 501, y: 437))
        bolt.line(to: NSPoint(x: 467, y: 303))
        bolt.line(to: NSPoint(x: 610, y: 500))
        bolt.line(to: NSPoint(x: 521, y: 500))
        bolt.close()
        bolt.fill()
        NSGraphicsContext.restoreGraphicsState()
        let suffix = scale == 2 ? "@2x" : ""
        let url = URL(fileURLWithPath: output).appendingPathComponent("icon_\(size)x\(size)\(suffix).png")
        try bitmap.representation(using: .png, properties: [:])!.write(to: url)
    }
}
