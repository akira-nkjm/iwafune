import { Box3, Quaternion, Vector3 } from "three";
import type { DesignView, PartRow } from "@/api/types";
import { systemTone } from "@/lib/systems";
import { toneHex } from "@/lib/tone";

export type V3 = [number, number, number];
export type Q4 = [number, number, number, number];

export interface Box3D {
  id: string;
  /** 機体座標の位置 [m]（API の body_position） */
  position: V3;
  /** 機体座標の姿勢 [w, x, y, z] */
  orientation: Q4;
  /** 箱の辺 [m]（size 属性の SI 値。無ければ既定の箱） */
  size: V3;
  color: string;
  sized: boolean;
  /** API が返した系で構造を判別する（名前や ID から推測しない）。 */
  structure: boolean;
}

/** 部品座標の方向ベクトルを持つ属性（RW のスピン軸・MTQ の軸・パネルの法線・センサの視線・推力の向き） */
export const AXIS_ATTRS = ["spin_axis", "axis", "normal", "boresight", "direction"] as const;

export interface Axis3D {
  id: string;
  attr: (typeof AXIS_ATTRS)[number];
  part: string;
  /** 部品座標の向き（長さは表示のために 1 に揃える） */
  dir: V3;
}

/** 方向属性を部品の子として描く矢印にする。座標変換は親 mesh に任せる */
export function axesOf(design: DesignView): Axis3D[] {
  const out: Axis3D[] = [];
  for (const p of design.parts) {
    if (!p.mount) continue;
    for (const attr of AXIS_ATTRS) {
      const v = p.values[attr]?.value;
      if (v?.type !== "vector") continue;
      const d = new Vector3(...v.si);
      if (d.lengthSq() === 0) continue;
      d.normalize();
      out.push({ id: `${p.id}.${attr}`, attr, part: p.id, dir: [d.x, d.y, d.z] });
    }
  }
  return out;
}

/** Bounds の最大辺に対する表示寸法の比。装飾の寸法はここだけで定義する。 */
export const SCENE_RATIOS = {
  defaultBox: 0.12,
  arrow: 0.25,
  arrowHead: 0.06,
  arrowWidth: 0.035,
  comRadius: 0.025,
  axes: 0.5,
  grid: 6,
  gridCell: 0.05,
  gridSection: 0.5,
  gridFade: 10,
  cameraX: 2,
  cameraY: -2.3,
  cameraZ: 1.7,
  near: 0.001,
  far: 100,
} as const;

/** 外形も位置の広がりも無い設計だけに使う表示の基準 [m]。 */
const EMPTY_EXTENT = 1;

/** 回転を含む機体座標での外形。未寸法の部品は位置だけを含める。 */
export function boundsOf(boxes: readonly Box3D[]): Box3 {
  const bounds = new Box3();
  for (const box of boxes) {
    const [w, x, y, z] = box.orientation;
    const q = new Quaternion(x, y, z, w);
    for (const sx of [-1, 1])
      for (const sy of [-1, 1])
        for (const sz of [-1, 1]) {
          bounds.expandByPoint(
            new Vector3((sx * box.size[0]) / 2, (sy * box.size[1]) / 2, (sz * box.size[2]) / 2)
              .applyQuaternion(q)
              .add(new Vector3(...box.position)),
          );
        }
  }
  return bounds;
}

/** すべての表示寸法を外形の最大辺から作る。倍率は Bounds と同じになる。 */
export function sceneDimensions(bounds: Box3): Record<keyof typeof SCENE_RATIOS, number> {
  const size = bounds.getSize(new Vector3());
  const extent = Math.max(size.x, size.y, size.z) || EMPTY_EXTENT;
  return Object.fromEntries(
    Object.entries(SCENE_RATIOS).map(([key, ratio]) => [key, extent * ratio]),
  ) as Record<keyof typeof SCENE_RATIOS, number>;
}

function sizeOf(p: PartRow): V3 | null {
  const v = p.values.size?.value;
  return v?.type === "vector" ? v.si : null;
}

/** 設計から 3D の箱を作る（機体座標が導出できた部品だけ）。色は系の色か、colorOf の上書き */
export function boxesOf(
  design: DesignView,
  colorOf?: (id: string) => string | undefined,
): { boxes: Box3D[]; unplaced: string[]; dimensions: ReturnType<typeof sceneDimensions> } {
  const boxes: Box3D[] = [];
  const unplaced: string[] = [];
  for (const p of design.parts) {
    // 取付のある部品だけを置く（取付があれば機体座標の位置・姿勢は必ず API が返す）
    const m = p.mount;
    if (!m) {
      unplaced.push(p.id);
      continue;
    }
    const size = sizeOf(p);
    boxes.push({
      id: p.id,
      position: m.body_position,
      orientation: m.body_orientation,
      size: size ?? [0, 0, 0],
      color: colorOf?.(p.id) ?? toneHex[systemTone(p.system)],
      sized: size !== null,
      structure: p.system === "structure",
    });
  }
  const bounds = boundsOf(boxes);
  const dimensions = sceneDimensions(bounds);
  for (const box of boxes) {
    if (!box.sized)
      box.size = [dimensions.defaultBox, dimensions.defaultBox, dimensions.defaultBox];
  }
  return { boxes, unplaced, dimensions };
}

/** 配置では構造の内側が見える不透明度を既定にする。 */
export const DEFAULT_STRUCTURE_OPACITY = 0.3;

/** 材質は透過モードを固定し、スライダー操作でシェーダーを作り直さない。透過した箱は奥行きを遮らない。 */
export function boxMaterial(box: Box3D, selected: boolean, structureOpacity?: number) {
  const opacity = selected
    ? 1
    : box.structure && structureOpacity !== undefined
      ? structureOpacity
      : box.sized
        ? 0.85
        : 0.45;
  return { color: box.color, transparent: true, opacity, depthWrite: opacity === 1 };
}
