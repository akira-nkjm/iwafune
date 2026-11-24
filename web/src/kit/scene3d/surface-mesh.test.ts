import { describe, expect, it } from "vitest";
import {
  deformedPositions,
  edgeIndices,
  flatPositions,
  meshExtent,
  nodeColors,
  triangleIndices,
} from "./surface-mesh";
import type { V3 } from "./types";

/** 2 × 2 の四角形（3 × 3 の節点）の 1 枚のパネル。節点は (i, j) → 3 i + j、z = 0 の面 */
const NODES: V3[] = Array.from(
  { length: 9 },
  (_, n): V3 => [Math.floor(n / 3) * 0.5, (n % 3) * 0.5, 0],
);
const QUADS: [number, number, number, number][] = [
  [0, 3, 4, 1],
  [1, 4, 5, 2],
  [3, 6, 7, 4],
  [4, 7, 8, 5],
];
const MESH = { groups: [{ id: "P", quads: QUADS }] };

describe("surface mesh geometry", () => {
  it("splits each quad into two triangles", () => {
    const t = triangleIndices(MESH);
    expect(t.length).toBe(4 * 6);
    expect([...t.slice(0, 6)]).toEqual([0, 3, 4, 0, 4, 1]);
  });

  it("lists each shared edge once", () => {
    // 2 × 2 の格子の辺は 12 本（4 四角形 × 4 辺 = 16 のうち、内側の 4 辺が 2 回出る）
    const e = edgeIndices(MESH);
    expect(e.length).toBe(12 * 2);
    const keys = new Set<string>();
    for (let i = 0; i < e.length; i += 2) {
      const a = e[i] as number;
      const b = e[i + 1] as number;
      keys.add(a < b ? `${a}:${b}` : `${b}:${a}`);
    }
    expect(keys.size).toBe(12);
  });

  it("moves each node by scale times its displacement", () => {
    const disp: V3[] = NODES.map((_, n): V3 => [0, 0, n === 4 ? 1 : 0]);
    const out = deformedPositions(NODES, disp, 0.1);
    // 中心の節点（4）だけが z に 0.1 動く
    expect(out[4 * 3 + 2]).toBeCloseTo(0.1, 6);
    expect(out[4 * 3]).toBeCloseTo(0.5, 6);
    expect(out[0 * 3 + 2]).toBe(0);
    // 倍率 0 は元の位置
    expect([...deformedPositions(NODES, disp, 0)]).toEqual([...flatPositions(NODES)]);
  });

  it("writes into the given array and leaves nodes without a displacement in place", () => {
    const out = new Float32Array(NODES.length * 3);
    const r = deformedPositions(NODES, [[1, 0, 0]], 2, out);
    expect(r).toBe(out);
    expect(out[0]).toBeCloseTo(2, 6);
    expect(out[3 + 1]).toBeCloseTo(0.5, 6);
  });

  it("maps each node value to a colour", () => {
    const c = nodeColors([0, 1], (r) => [r, 0.5, 1 - r]);
    expect([...c]).toEqual([0, 0.5, 1, 1, 0.5, 0]);
  });

  it("measures the diagonal of the bounds", () => {
    expect(meshExtent(NODES)).toBeCloseTo(Math.hypot(1, 1, 0), 9);
    expect(meshExtent([])).toBe(1);
  });
});
