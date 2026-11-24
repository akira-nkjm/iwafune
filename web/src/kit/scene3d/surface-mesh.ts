import type { SurfaceMesh } from "@/api/types";
import type { V3 } from "./types";

/**
 * メッシュ上の場の表示用の幾何（`SurfaceMesh` から three.js の BufferGeometry に渡す配列を作る）。
 * 位置の足し算（節点 + 倍率 × 変位）と配列の組み立てだけで、物理は計算しない（変位・場の値・規格化は API）。
 */

/** 四角形（節点 4 つ）を 2 つの三角形に分けた頂点の番号（0-1-2, 0-2-3）。パネルの区別は要らないので全部つなぐ */
export function triangleIndices(mesh: Pick<SurfaceMesh, "groups">): Uint32Array {
  const out: number[] = [];
  for (const g of mesh.groups) for (const [a, b, c, d] of g.quads) out.push(a, b, c, a, c, d);
  return Uint32Array.from(out);
}

/**
 * 辺（線分の両端の節点番号の対）。隣り合う四角形が共有する辺は 1 本にする（二重に描いて濃く見えないように）。
 * 並びは四角形の出てきた順で、決定的。
 */
export function edgeIndices(mesh: Pick<SurfaceMesh, "groups">): Uint32Array {
  const seen = new Set<string>();
  const out: number[] = [];
  for (const g of mesh.groups) {
    for (const q of g.quads) {
      for (let i = 0; i < 4; i += 1) {
        const a = q[i] as number;
        const b = q[(i + 1) % 4] as number;
        const key = a < b ? `${a}:${b}` : `${b}:${a}`;
        if (seen.has(key)) continue;
        seen.add(key);
        out.push(a, b);
      }
    }
  }
  return Uint32Array.from(out);
}

/** 節点の座標を平らな配列（x, y, z, x, y, z, …）にする */
export function flatPositions(nodes: readonly V3[]): Float32Array {
  const out = new Float32Array(nodes.length * 3);
  nodes.forEach((p, i) => {
    out.set(p, i * 3);
  });
  return out;
}

/**
 * 変形した位置 = 節点の位置 + 倍率 × 変位。`out` を渡すとそこへ書く（アニメーションで毎コマ配列を作らない）。
 * 倍率は表示のためだけの量（見やすい大きさに拡大する）。変位の数が節点より少なければ、足りない節点は動かさない。
 */
export function deformedPositions(
  nodes: readonly V3[],
  displacement: readonly V3[],
  scale: number,
  out: Float32Array = new Float32Array(nodes.length * 3),
): Float32Array {
  for (let i = 0; i < nodes.length; i += 1) {
    const p = nodes[i] as V3;
    const d = displacement[i];
    for (let k = 0; k < 3; k += 1)
      out[i * 3 + k] = (p[k] as number) + scale * (d ? (d[k] as number) : 0);
  }
  return out;
}

/** 節点ごとの場の値（0 〜 1）を、色の写し方 `hexOf`（`lib/tone.ts` の配色）で RGB（0 〜 1）の平らな配列にする */
export function nodeColors(
  values: readonly number[],
  rgbOf: (ratio: number) => readonly [number, number, number],
): Float32Array {
  const out = new Float32Array(values.length * 3);
  values.forEach((v, i) => {
    out.set(rgbOf(v), i * 3);
  });
  return out;
}

/** 節点の座標の範囲の対角の長さ（場面の大きさ。カメラの距離・限界を決める） */
export function meshExtent(nodes: readonly V3[]): number {
  if (nodes.length === 0) return 1;
  const lo: V3 = [Infinity, Infinity, Infinity];
  const hi: V3 = [-Infinity, -Infinity, -Infinity];
  for (const p of nodes)
    for (let k = 0; k < 3; k += 1) {
      lo[k] = Math.min(lo[k] as number, p[k] as number);
      hi[k] = Math.max(hi[k] as number, p[k] as number);
    }
  return Math.hypot(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]) || 1;
}
