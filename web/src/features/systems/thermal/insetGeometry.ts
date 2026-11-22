import type { OrbitPlaneTrack } from "@/api/types";
import type { HeatSource } from "./heat";

/** 軌道の図の一辺（SVG の座標。表示の大きさは CSS で決まる） */
export const INSET_SIZE = 150;

/** 図の縁の余白（太陽の印を置く） */
const SUN_MARGIN = 20;

/** 図の座標を小数 1 桁に丸める（path の文字列を短くするだけ。量の表示ではない） */
const px = (v: number) => Math.round(v * 10) / 10;

/** 軌道の図の、時刻によらない部分（軌道の線・中心天体・影の帯・太陽の印）。SVG の座標 */
export interface InsetLayout {
  size: number;
  /** 中心天体の中心 */
  cx: number;
  cy: number;
  /** メートルから図の長さへの一様な倍率 */
  scale: number;
  bodyRadius: number;
  /** 影の帯（中心から −x 側の端まで） */
  shadow: { x: number; y: number; width: number; height: number };
  /** 軌道の線（全標本を通る折れ線の path） */
  path: string;
  /** 太陽の印の中心（+x 側の縁） */
  sun: { x: number; y: number };
}

/**
 * 表示枠の座標（API の `orbit_plane`。メートル）を図の座標にする。**回転も射影もしない**: 表示枠はサーバーが決めていて、
 * ここは一様な倍率で収め、y の向きを画面に合わせて反転するだけ。標本が無い・半径が無ければ null。
 */
export function insetLayout(plane: OrbitPlaneTrack, size = INSET_SIZE): InsetLayout | null {
  const n = Math.min(plane.x.length, plane.y.length);
  const body = plane.body_radius;
  if (n === 0 || !(body > 0)) return null;
  let reach = body;
  for (let i = 0; i < n; i++) reach = Math.max(reach, Math.hypot(plane.x[i] ?? 0, plane.y[i] ?? 0));
  const c = size / 2;
  const scale = (c - SUN_MARGIN) / reach;
  const path = Array.from(
    { length: n },
    (_, i) =>
      `${i === 0 ? "M" : "L"}${px(c + (plane.x[i] ?? 0) * scale)} ${px(c - (plane.y[i] ?? 0) * scale)}`,
  ).join("");
  const half = plane.shadow_half_width * scale;
  return {
    size,
    cx: c,
    cy: c,
    scale,
    bodyRadius: body * scale,
    shadow: { x: 0, y: c - half, width: c, height: 2 * half },
    path,
    sun: { x: size - SUN_MARGIN / 2, y: c },
  };
}

/** 現在の標本の衛星の位置（図の座標）と日陰か。`index` は標本の範囲に収める */
export function satelliteAt(
  layout: InsetLayout,
  plane: OrbitPlaneTrack,
  index: number,
): { x: number; y: number; eclipse: boolean } {
  const last = Math.min(plane.x.length, plane.y.length) - 1;
  const i = Math.min(Math.max(index, 0), last);
  return {
    x: px(layout.cx + (plane.x[i] ?? 0) * layout.scale),
    y: px(layout.cy - (plane.y[i] ?? 0) * layout.scale),
    eclipse: plane.eclipse[i] ?? false,
  };
}

/** 熱画像に重ねる軌道の図と太陽の矢印（`sun` は 3D の `SunArrow` と同じ形）。非定常のときだけ作る（それ以外は何も無し） */
export function orbitView(
  source: HeatSource,
  plane: OrbitPlaneTrack | undefined,
  index: number,
): {
  plane: OrbitPlaneTrack | null;
  sun: { dir: OrbitPlaneTrack["sun_dir_body"][number]; eclipse: boolean } | null;
} {
  if (source.kind !== "transient" || !plane || plane.x.length === 0)
    return { plane: null, sun: null };
  const i = Math.min(Math.max(index, 0), plane.x.length - 1);
  const dir = plane.sun_dir_body[i];
  return { plane, sun: dir ? { dir, eclipse: plane.eclipse[i] ?? false } : null };
}
