import { sampleTrack } from "./sample";
import type { PuppetAction } from "./types";

export interface SpriteAction extends Omit<PuppetAction, "stages"> {
  frames: Array<{ frame: number; durationMs: number }>;
}

export interface SpriteLibrary {
  version: number;
  frameCount: number;
  frameSize: number;
  baseline: number;
  columns: number;
  blendMs: number;
  actions: Record<string, SpriteAction>;
}

/** Shared clock for scrubbing, finite playback and background cancellation. */
export function sampleSprite(library: SpriteLibrary, actionId: string, timeMs: number) {
  const action = library.actions[actionId];
  if (!action) throw new Error(`Unknown sprite action: ${actionId}`);
  const time = Math.max(0, Math.min(timeMs, action.durationMs));
  let start = 0, index = 0;
  while (index < action.frames.length - 1 && time >= start + action.frames[index].durationMs) {
    start += action.frames[index++].durationMs;
  }
  const current = action.frames[index].frame;
  const previous = action.frames[Math.max(0, index - 1)].frame;
  const mix = index === 0 || time === action.durationMs || !library.blendMs
    ? 1 : Math.min(1, (time - start) / library.blendMs);
  const route = sampleTrack(action.route, time / action.durationMs);
  return { current, previous, mix, displacement: { x: route.x, y: route.y } };
}
