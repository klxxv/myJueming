import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { build } from "esbuild";

const root = new URL("../../apps/desktop/src/assets/companion/sprites/v1/", import.meta.url);
const library = JSON.parse(await readFile(new URL("motions.json", root), "utf8"));
const bundled = await build({ entryPoints: [new URL("../../apps/desktop/src/domain/companion/puppet/sprite.ts", import.meta.url).pathname], bundle: true, format: "esm", platform: "node", write: false });
const { sampleSprite } = await import(`data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`);

for (const [id, action] of Object.entries(library.actions)) {
  assert.equal(action.frames.reduce((sum, frame) => sum + frame.durationMs, 0), action.durationMs);
  let boundary = 0;
  for (let i = 0; i < action.frames.length; i++) {
    const frame = action.frames[i];
    assert.ok(Number.isInteger(frame.frame) && frame.frame >= 0 && frame.frame < library.frameCount);
    assert.ok(frame.durationMs > library.blendMs, "Blend must finish before the next frame");
    if (i) {
      const before = sampleSprite(library, id, boundary - .001);
      const after = sampleSprite(library, id, boundary);
      assert.equal(after.previous, before.current, "Frame transitions start at the preceding pose");
      assert.equal(after.mix, 0);
    }
    boundary += frame.durationMs;
  }
  const start = sampleSprite(library, id, 0);
  assert.deepEqual(sampleSprite(library, id, -100), start);
  const end = sampleSprite(library, id, action.durationMs);
  assert.deepEqual(sampleSprite(library, id, action.durationMs + 60000), end, "Finished actions hold still");
  assert.equal(end.current, action.frames.at(-1).frame);
  assert.equal(end.mix, 1);
}
assert.equal(sampleSprite(library, "sleep", 2600).current, 14);
assert.deepEqual(sampleSprite(library, "jump-to-bed", 2200).displacement, { x: 220, y: -80 });
assert.deepEqual(sampleSprite(library, "go-to-garden", 5400).displacement, { x: 340, y: 0 });
for (const actor of ["orange-cat", "golden-dog"]) {
  const manifest = JSON.parse(await readFile(new URL(`${actor}/manifest.json`, root), "utf8"));
  assert.equal(manifest.frames.length, library.frameCount);
  assert.equal(manifest.baseline, library.baseline);
  for (let i = 0; i < library.frameCount; i++) {
    const png = await readFile(new URL(`${actor}/frames/${String(i).padStart(2, "0")}.png`, root));
    assert.equal(png.readUInt32BE(16), library.frameSize);
    assert.equal(png.readUInt32BE(20), library.frameSize);
    assert.equal(png[25], 6, "Every PNG frame must have RGBA transparency");
  }
  for (const id of Object.keys(library.actions).filter(id => id !== "stand")) {
    const gif = await readFile(new URL(`${actor}/previews/${id}.gif`, root));
    assert.equal(gif.subarray(0, 6).toString(), "GIF89a");
    assert.ok(!gif.includes(Buffer.from("NETSCAPE2.0")), "Preview GIFs must not loop");
  }
}
console.log("sprite assets: 32 RGBA frames, action boundaries, terminal poses and 14 finite GIFs passed");
