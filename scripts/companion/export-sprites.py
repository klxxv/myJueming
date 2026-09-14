#!/usr/bin/env python3
"""Pack generated pose atlases into aligned RGBA frames and finite GIF previews.

Requires Pillow and NumPy. The generated source sheets stay unmodified. Chroma
matting is an export step; all character drawing comes from the source sheets.
"""
import json
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "apps/desktop/src/assets/companion/sprites/v1"


def keep_character(cell):
    """Discard disconnected ink from a neighbour that crossed a grid boundary."""
    pixels = np.array(cell)
    remaining = pixels[:, :, 3] > 0
    largest = []
    height, width = remaining.shape
    for y, x in np.argwhere(remaining):
        if not remaining[y, x]:
            continue
        component, queue = [], deque([(int(y), int(x))])
        remaining[y, x] = False
        while queue:
            cy, cx = queue.popleft()
            component.append((cy, cx))
            for ny, nx in [(cy-1, cx), (cy+1, cx), (cy, cx-1), (cy, cx+1)]:
                if 0 <= ny < height and 0 <= nx < width and remaining[ny, nx]:
                    remaining[ny, nx] = False
                    queue.append((ny, nx))
        if len(component) > len(largest):
            largest = component
    alpha = np.zeros((height, width), dtype=np.uint8)
    for y, x in largest:
        alpha[y, x] = pixels[y, x, 3]
    pixels[:, :, 3] = alpha
    return Image.fromarray(pixels)


def export(actor):
    source = Image.open(actor / "source-sheet.png").convert("RGB")
    rgb = np.asarray(source).astype(np.int16)
    r, g, b = rgb[:, :, 0], rgb[:, :, 1], rgb[:, :, 2]
    if actor.name == "golden-dog":
        background = (g > r + 35) & (g > b + 35)
    else:
        # The cat generation painted a cool grey checkerboard instead of alpha.
        # Fur, ink and cream are warm: this key preserves their interior shading.
        background = (b >= r - 4) & (r > 75) & (r < 230) & (b < r + 45)
    rgba = np.concatenate([rgb, np.where(background, 0, 255)[:, :, None]], axis=2).astype(np.uint8)
    if actor.name == "golden-dog":
        # Remove green spill on the antialiased contour only.
        rgba[:, :, 1] = np.minimum(rgba[:, :, 1], np.maximum(rgba[:, :, 0], rgba[:, :, 2]))
    clean = Image.fromarray(rgba)
    atlas = Image.new("RGBA", (1536, 1536))
    frames = []
    manifest = []
    (actor / "frames").mkdir(exist_ok=True)
    (actor / "previews").mkdir(exist_ok=True)
    for i in range(16):
        col, row = i % 4, i // 4
        cell = clean.crop((round(col * source.width / 4), round(row * source.height / 4),
                           round((col + 1) * source.width / 4), round((row + 1) * source.height / 4)))
        cell = keep_character(cell)
        bounds = cell.getbbox()
        assert bounds, f"Empty frame {i}"
        cutout = cell.crop(bounds)
        # Keep the common pixel scale; only align each frame's centre and feet.
        frame = Image.new("RGBA", (384, 384))
        offset = (round((384 - cutout.width) / 2), 352 - cutout.height)
        frame.alpha_composite(cutout, offset)
        frame.save(actor / "frames" / f"{i:02}.png", optimize=True)
        atlas.alpha_composite(frame, (col * 384, row * 384))
        frames.append(frame)
        manifest.append(dict(id=i, sourceBounds=list(bounds), offset=list(offset)))
    atlas.save(actor / "atlas.webp", lossless=True)
    (actor / "manifest.json").write_text(json.dumps(dict(frameSize=384, baseline=352,
        columns=4, frameCount=16, frames=manifest), indent=2) + "\n")
    library = json.loads((ASSETS / "motions.json").read_text())
    for key in ["rise", "walk", "jump-to-bed", "sleep", "sit-down", "wake-up", "go-to-garden"]:
        action = library["actions"][key]
        images = []
        durations = []
        for item in action["frames"]:
            frame = frames[item["frame"]]
            # GIF has one-bit alpha; PNG/WebP remain the runtime sources.
            paletted = frame.convert("RGB").quantize(colors=255)
            paletted.paste(255, mask=frame.getchannel("A").point(lambda alpha: 255 if alpha < 128 else 0))
            images.append(paletted)
            durations.append(item["durationMs"])
        images[0].save(actor / "previews" / f"{key}.gif", save_all=True,
            append_images=images[1:], duration=durations, disposal=2,
            transparency=255, optimize=False)  # No loop extension: play once.
    print(f"{actor.name}: 16 RGBA frames, atlas and 7 finite GIF previews exported")


if __name__ == "__main__":
    for name in ["orange-cat", "golden-dog"]:
        export(ASSETS / name)
