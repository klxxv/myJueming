import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

guard CommandLine.arguments.count == 3 else {
    fputs("usage: generate-tutorial-butterfly-gif <sprite.png> <output.gif>\n", stderr)
    exit(2)
}

let sourceURL = URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL
let outputURL = URL(fileURLWithPath: CommandLine.arguments[2]) as CFURL

guard
    let imageSource = CGImageSourceCreateWithURL(sourceURL, nil),
    let sprite = CGImageSourceCreateImageAtIndex(imageSource, 0, nil),
    let destination = CGImageDestinationCreateWithURL(
        outputURL,
        UTType.gif.identifier as CFString,
        8,
        nil
    )
else {
    fputs("could not open the sprite sheet or GIF destination\n", stderr)
    exit(1)
}

let destinationProperties: CFDictionary = [
    kCGImagePropertyGIFDictionary: [
        // A short finite loop avoids a permanent decorative animation in the workspace.
        kCGImagePropertyGIFLoopCount: 3,
    ],
] as CFDictionary
CGImageDestinationSetProperties(destination, destinationProperties)

let frameProperties: CFDictionary = [
    kCGImagePropertyGIFDictionary: [
        kCGImagePropertyGIFDelayTime: 0.10,
        kCGImagePropertyGIFUnclampedDelayTime: 0.10,
    ],
] as CFDictionary

let outputSize = 96
let colorSpace = CGColorSpaceCreateDeviceRGB()

for frameIndex in 0..<8 {
    let column = frameIndex % 4
    let row = frameIndex / 4
    let x0 = column * sprite.width / 4
    let x1 = (column + 1) * sprite.width / 4
    let y0 = row * sprite.height / 2
    let y1 = (row + 1) * sprite.height / 2
    let cropRect = CGRect(x: x0, y: y0, width: x1 - x0, height: y1 - y0)

    guard
        let crop = sprite.cropping(to: cropRect),
        let context = CGContext(
            data: nil,
            width: outputSize,
            height: outputSize,
            bitsPerComponent: 8,
            bytesPerRow: outputSize * 4,
            space: colorSpace,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        )
    else {
        fputs("could not render frame \(frameIndex)\n", stderr)
        exit(1)
    }

    context.interpolationQuality = .high
    context.clear(CGRect(x: 0, y: 0, width: outputSize, height: outputSize))
    context.draw(crop, in: CGRect(x: 0, y: 0, width: outputSize, height: outputSize))

    guard let frame = context.makeImage() else {
        fputs("could not finalize frame \(frameIndex)\n", stderr)
        exit(1)
    }
    CGImageDestinationAddImage(destination, frame, frameProperties)
}

guard CGImageDestinationFinalize(destination) else {
    fputs("could not finalize GIF\n", stderr)
    exit(1)
}
