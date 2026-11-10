import { Vector3 } from "three";
import type { DesignView, PartRow } from "@/api/types";
import { subsystemTone } from "@/lib/subsystems";
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

/** size が無い部品の仮の箱 [m] */
export const DEFAULT_BOX: V3 = [0.05, 0.05, 0.05];

function sizeOf(p: PartRow): V3 | null {
  const v = p.values.size?.value;
  return v?.type === "vector" ? v.si : null;
}

/** 設計から 3D の箱を作る（機体座標が導出できた部品だけ）。色はサブシステム色か、colorOf の上書き */
export function boxesOf(
  design: DesignView,
  colorOf?: (id: string) => string | undefined,
): { boxes: Box3D[]; unplaced: string[] } {
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
      size: size ?? DEFAULT_BOX,
      color: colorOf?.(p.id) ?? toneHex[subsystemTone(p.subsystem, p.role)],
      sized: size !== null,
    });
  }
  return { boxes, unplaced };
}
