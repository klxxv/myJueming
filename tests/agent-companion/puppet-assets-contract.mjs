import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { build } from "esbuild";

const assetRoot = new URL("../../apps/desktop/src/assets/companion/puppets/v1/", import.meta.url);
const json = async path => JSON.parse(await readFile(new URL(path, assetRoot), "utf8"));
const library = await json("motions.json");
const bundled = await build({ entryPoints: [new URL("../../apps/desktop/src/domain/companion/puppet/sample.ts", import.meta.url).pathname], bundle: true, format: "esm", platform: "node", write: false });
const { samplePuppet, sampleTrack } = await import(`data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString("base64")}`);

// Sparse channel authoring must preserve rotation while another channel changes.
const sparse = sampleTrack([{ at: 0, rotation: 0 }, { at: .5, y: 8 }, { at: 1, rotation: 90 }], .5);
assert.equal(sparse.rotation, 45);
assert.equal(sparse.y, 8);

for (const actor of ["orange-cat", "golden-dog"]) {
  const rig = await json(`${actor}/rig.json`);
  const boneIds = new Set(rig.bones.map(bone => bone.id));
  for (const clip of Object.values(library.clips)) {
    for (const [bone, track] of Object.entries(clip.tracks)) {
      assert.ok(boneIds.has(bone), `${actor}: unknown animated bone ${bone}`);
      assert.ok(track.length > 0);
      for (let i = 0; i < track.length; i++) {
        assert.ok(track[i].at >= 0 && track[i].at <= 1);
        if (i) assert.ok(track[i].at > track[i - 1].at, "Track times must increase");
        assert.ok(Object.values(track[i]).every(Number.isFinite));
      }
    }
    assert.ok(clip.expressions.every(key => rig.expressions.includes(key.value)));
  }
  for (const [id, action] of Object.entries(library.actions)) {
    assert.ok(action.durationMs > 0 && Number.isFinite(action.durationMs));
    assert.equal(action.stages[0].startMs, 0);
    assert.equal(action.stages.at(-1).endMs, action.durationMs);
    for (let i = 0; i < action.stages.length; i++) {
      const stage = action.stages[i];
      assert.ok(library.clips[stage.clip]);
      assert.ok(Number.isInteger(stage.cycles) && stage.cycles > 0);
      assert.ok(stage.endMs > stage.startMs);
      if (i) assert.equal(stage.startMs, action.stages[i - 1].endMs);
    }
    assert.equal(action.route[0].at, 0);
    assert.equal(action.route.at(-1).at, 1);
    for (let i = 1; i < action.route.length; i++) assert.ok(action.route[i].at > action.route[i - 1].at);
    // Inspect dense samples: no invalid transforms or runaway motion in the authored pack.
    for (let t = 0; t <= 100; t++) {
      const frame = samplePuppet(rig, library, id, action.durationMs * t / 100);
      for (const matrix of frame.bones.values()) assert.ok(matrix.every(Number.isFinite));
      assert.ok(Math.abs(frame.displacement.x) <= 400 && Math.abs(frame.displacement.y) <= 180);
    }
    const end = samplePuppet(rig, library, id, action.durationMs);
    assert.deepEqual(end, samplePuppet(rig, library, id, action.durationMs + 50_000), "Finite actions hold their terminal pose");
    for (const stage of action.stages.slice(1)) {
      const before = samplePuppet(rig, library, id, stage.startMs - .001);
      const after = samplePuppet(rig, library, id, stage.startMs);
      for (const [bone, matrix] of before.bones) matrix.forEach((value, i) => assert.ok(Math.abs(value - after.bones.get(bone)[i]) < .01, `${id}: discontinuous joint ${bone}`));
    }
  }
  assert.equal(samplePuppet(rig, library, "sleep", 3800).expression, "sleep");
  assert.deepEqual(samplePuppet(rig, library, "jump-to-bed", 1700).displacement, { x: 220, y: -80 });
  assert.deepEqual(samplePuppet(rig, library, "go-to-garden", 5400).displacement, { x: 340, y: 0 });
}
console.log("puppet assets: shared rig bindings, finite actions, clip continuity and terminal poses passed");
