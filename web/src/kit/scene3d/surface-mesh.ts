import type { SurfaceMesh } from "@/api/types";
import type { V3 } from "./types";

/**
 * メッシュ上の場の表示用の幾何（`SurfaceMesh` から three.js の BufferGeometry に渡す配列を作る）。
 * 位置の足し算（節点 + 倍率 × 変位）と配列の組み立てだけで、物理は計算しない（変位・場の値・規格化は API）。
 * 場の値は節点ごと（`"node"`）か、四角形（升）ごと（`"cell"`）。四角形の通し番号は、パネルの順に四角形を並べた順。
 */

/** 場の値の置き場所: 節点ごと（滑らかに塗る）か、四角形ごと（升を 1 色で塗る） */
export type FieldAt = "node" | "cell";

/** 四角形（節点 4 つ）を 2 つの三角形に分けた頂点の番号（0-1-2, 0-2-3）。パネルの区別は要らないので全部つなぐ */
export function triangleIndices(mesh: Pick<SurfaceMesh, "groups">): Uint32Array {
  const out: number[] = [];
  for (const g of mesh.groups) for (const [a, b, c, d] of g.quads) out.push(a, b, c, a, c, d);
  return Uint32Array.from(out);
}

/**
 * 辺（線分の両端の節点番号の対）。隣り合う四角形が共有する辺は 1 本にする（二重に描いて濃く見えないように）。
 * 面でない線（`lines`。フレームの部材）が面の辺と同じなら 1 本にして、面の辺のあとに足す。
 * 並びは四角形・線の出てきた順で、決定的。
 */
export function edgeIndices(mesh: Pick<SurfaceMesh, "groups" | "lines">): Uint32Array {
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
  for (const [a, b] of mesh.lines ?? []) {
    const key = a < b ? `${a}:${b}` : `${b}:${a}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(a, b);
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

/** 四角形の数 */
export function quadCount(mesh: Pick<SurfaceMesh, "groups">): number {
  return mesh.groups.reduce((n, g) => n + g.quads.length, 0);
}

/**
 * 四角形ごとに塗るための頂点の元の節点番号（四角形の通し番号 × 4 + 角）。四角形は頂点を共有せずに 1 枚ずつ 4 頂点にして、
 * 色を四角形の中で一定にする。
 */
export function quadVertexNodes(mesh: Pick<SurfaceMesh, "groups">): Uint32Array {
  const out: number[] = [];
  for (const g of mesh.groups) for (const q of g.quads) out.push(...q);
  return Uint32Array.from(out);
}

/** 四角形ごとの頂点（`quadVertexNodes` の並び）の三角形の番号（0-1-2, 0-2-3 を四角形ごとに 4 つずつずらす） */
export function quadTriangleIndices(quads: number): Uint32Array {
  const out = new Uint32Array(quads * 6);
  for (let k = 0; k < quads; k += 1)
    out.set(
      [0, 1, 2, 0, 2, 3].map((i) => 4 * k + i),
      6 * k,
    );
  return out;
}

/** 節点の位置 `positions`（x, y, z, …）を、頂点ごとの元の節点番号 `nodeOf` で引き写す（`out` に書く） */
export function gatherPositions(
  positions: Float32Array,
  nodeOf: Uint32Array,
  out: Float32Array,
): Float32Array {
  for (let v = 0; v < nodeOf.length; v += 1) {
    const n = (nodeOf[v] as number) * 3;
    out[v * 3] = positions[n] as number;
    out[v * 3 + 1] = positions[n + 1] as number;
    out[v * 3 + 2] = positions[n + 2] as number;
  }
  return out;
}

/** 四角形ごとの場の値（0 〜 1）を、色の写し方 `rgbOf` で、四角形の 4 頂点すべての RGB にする（`quadVertexNodes` の並び） */
export function quadColors(
  values: readonly number[],
  rgbOf: (ratio: number) => readonly [number, number, number],
): Float32Array {
  const out = new Float32Array(values.length * 12);
  values.forEach((v, k) => {
    const rgb = rgbOf(v);
    for (let c = 0; c < 4; c += 1) out.set(rgb, (4 * k + c) * 3);
  });
  return out;
}

/** 三角形の番号 → 四角形の通し番号（`triangleIndices` も `quadTriangleIndices` も、四角形ごとに 2 三角形を順に並べる） */
export function quadOfFace(faceIndex: number): number {
  return Math.floor(faceIndex / 2);
}

/** 通し番号 `quad` の四角形の 4 辺（線分の両端の節点番号の対）。範囲外なら空 */
export function quadEdges(mesh: Pick<SurfaceMesh, "groups">, quad: number): Uint32Array {
  let k = quad;
  for (const g of mesh.groups) {
    const q = g.quads[k];
    if (q) return Uint32Array.from([q[0], q[1], q[1], q[2], q[2], q[3], q[3], q[0]]);
    k -= g.quads.length;
  }
  return new Uint32Array(0);
}
