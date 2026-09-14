/** Presentation-only cutout animation data. No project IDs or canonical writes. */
export interface PuppetBone {
  id: string;
  parent: string | null;
  x: number;
  y: number;
}

export interface PuppetPart {
  id: string;
  label: string;
  bone: string;
  bounds: number[];
  file: string;
  expressions?: string[];
}

export interface PuppetRig {
  version: number;
  id: string;
  label: string;
  viewBox: number[];
  origin: number[];
  groundY: number;
  defaultExpression: string;
  expressions: string[];
  /** Parent-before-child order; draw order is independently defined by parts. */
  bones: PuppetBone[];
  parts: PuppetPart[];
}

export interface BonePose {
  x: number;
  y: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
}

export interface PoseKey extends Partial<BonePose> {
  at: number;
}

export interface PuppetClip {
  label: string;
  durationMs: number;
  tracks: Record<string, PoseKey[]>;
  expressions: Array<{ at: number; value: string }>;
}

export interface PuppetAction {
  label: string;
  description: string;
  durationMs: number;
  stages: Array<{ clip: string; startMs: number; endMs: number; cycles: number }>;
  /** Scene displacement in rig units, separate from local joint articulation. */
  route: Array<{ at: number; x: number; y: number }>;
  scene: "floor" | "bed" | "garden";
}

export interface PuppetMotionLibrary {
  version: number;
  clips: Record<string, PuppetClip>;
  actions: Record<string, PuppetAction>;
}

export type Matrix2D = [number, number, number, number, number, number];

export interface PuppetFrame {
  bones: Map<string, Matrix2D>;
  expression: string;
  displacement: { x: number; y: number };
}
