import type { BonePose, Matrix2D, PoseKey, PuppetFrame, PuppetMotionLibrary, PuppetRig } from "./types";

const neutral: BonePose = { x: 0, y: 0, rotation: 0, scaleX: 1, scaleY: 1 };
const channels = ["x", "y", "rotation", "scaleX", "scaleY"] as const;
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

export function sampleTrack(keys: PoseKey[], at: number): BonePose {
  const result = { ...neutral };
  // Each channel has its own keys: absent fields do not accidentally reset it.
  for (const channel of channels) {
    const authored = keys.filter(key => key[channel] !== undefined);
    if (!authored.length) continue;
    const rightIndex = authored.findIndex(key => key.at >= at);
    if (rightIndex === -1) result[channel] = authored[authored.length - 1][channel]!;
    else if (rightIndex === 0) result[channel] = authored[0][channel]!;
    else {
      const left = authored[rightIndex - 1], right = authored[rightIndex];
      const t = clamp((at - left.at) / (right.at - left.at), 0, 1);
      result[channel] = left[channel]! + (right[channel]! - left[channel]!) * t;
    }
  }
  return result;
}

function multiply(a: Matrix2D, b: Matrix2D): Matrix2D {
  return [
    a[0] * b[0] + a[2] * b[1], a[1] * b[0] + a[3] * b[1],
    a[0] * b[2] + a[2] * b[3], a[1] * b[2] + a[3] * b[3],
    a[0] * b[4] + a[2] * b[5] + a[4], a[1] * b[4] + a[3] * b[5] + a[5],
  ];
}

/** Pure sampler: playback owns scheduling; scrubbing is deterministic and timer-free. */
export function samplePuppet(rig: PuppetRig, library: PuppetMotionLibrary, actionId: string, timeMs: number): PuppetFrame {
  const action = library.actions[actionId];
  if (!action) throw new Error(`Unknown puppet action: ${actionId}`);
  const time = clamp(timeMs, 0, action.durationMs);
  const stage = action.stages.find(item => time < item.endMs) ?? action.stages[action.stages.length - 1];
  const clip = library.clips[stage.clip];
  const stageProgress = clamp((time - stage.startMs) / (stage.endMs - stage.startMs), 0, 1);
  const cycleProgress = stageProgress === 1 ? 1 : (stageProgress * stage.cycles) % 1;
  const bones = new Map<string, Matrix2D>();
  for (const bone of rig.bones) {
    const pose = sampleTrack(clip.tracks[bone.id] ?? [], cycleProgress);
    const angle = pose.rotation * Math.PI / 180;
    const local: Matrix2D = [
      Math.cos(angle) * pose.scaleX, Math.sin(angle) * pose.scaleX,
      -Math.sin(angle) * pose.scaleY, Math.cos(angle) * pose.scaleY,
      bone.x + pose.x, bone.y + pose.y,
    ];
    const parent = bone.parent ? bones.get(bone.parent) : undefined;
    if (bone.parent && !parent) throw new Error(`Missing or unordered parent: ${bone.parent}`);
    bones.set(bone.id, parent ? multiply(parent, local) : local);
  }
  const route = sampleTrack(action.route, time / action.durationMs);
  let expression = rig.defaultExpression;
  for (const key of clip.expressions) if (key.at <= cycleProgress) expression = key.value;
  return { bones, expression, displacement: { x: route.x, y: route.y } };
}
