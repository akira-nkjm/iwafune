import { describe, expect, it } from "vitest";
import { heatRatio } from "@/lib/heat-scale";
import { heatHex } from "@/lib/tone";
import { fieldRgb } from "./field-color";
import {
  deformedPositions,
  edgeIndices,
  flatPositions,
  gatherPositions,
  meshExtent,
  nodeColors,
  quadColors,
  quadCount,
  quadEdges,
  quadOfFace,
  quadTriangleIndices,
  quadVertexNodes,
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

  it("adds the lines that are not panel edges, once", () => {
    // 面の辺と同じ線 (0, 3) は足さず、面の外の線 (8, 0) と、同じ線の逆向き (0, 8) の重複は 1 本だけ足す
    const e = edgeIndices({
      ...MESH,
      lines: [
        [0, 3],
        [8, 0],
        [0, 8],
      ],
    });
    expect(e.length).toBe(13 * 2);
    expect([...e.slice(-2)]).toEqual([8, 0]);
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

  describe("per-cell field (one value per quad)", () => {
    it("counts quads and lists each quad's four nodes without sharing vertices", () => {
      expect(quadCount(MESH)).toBe(4);
      const v = quadVertexNodes(MESH);
      expect(v.length).toBe(16);
      expect([...v.slice(4, 8)]).toEqual([1, 4, 5, 2]);
    });

    it("splits every quad's own four vertices into two triangles", () => {
      expect([...quadTriangleIndices(2)]).toEqual([0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    });

    it("copies the node positions to the quad vertices", () => {
      const out = gatherPositions(
        flatPositions(NODES),
        quadVertexNodes(MESH),
        new Float32Array(48),
      );
      // 四角形 3（節点 4, 7, 8, 5）の最初の頂点は節点 4 = (0.5, 0.5, 0)
      expect([...out.slice(36, 39)]).toEqual([0.5, 0.5, 0]);
      // 節点 4 は 4 つの四角形すべてに出るので、4 回現れる
      let n = 0;
      for (let i = 0; i < 16; i += 1) if (out[i * 3] === 0.5 && out[i * 3 + 1] === 0.5) n += 1;
      expect(n).toBe(4);
    });

    /** 2 × 2 の升に温度（K）を割り当て（最低 280・最高 340）、熱画像と同じ範囲の写し方と配色を通した色になる */
    it("colours each cell through the shared heat scale (all four vertices of a cell alike)", () => {
      const temps = [280, 300, 320, 340];
      const scale = { min: 280, max: 340 };
      const colours = quadColors(
        temps.map((t) => heatRatio(t, scale)),
        fieldRgb,
      );
      expect(colours.length).toBe(4 * 4 * 3);
      temps.forEach((t, k) => {
        // 配色の表の段（1/64）に載る比 0, 1/3, 2/3, 1 ではなく、最も近い段の色になる
        const expected = Float32Array.from(fieldRgb(heatRatio(t, scale)));
        for (let c = 0; c < 4; c += 1)
          expect([...colours.slice((4 * k + c) * 3, (4 * k + c) * 3 + 3)]).toEqual([...expected]);
      });
      // 最低と最高は配色の両端（`heatHex(0)`・`heatHex(1)`）
      expect(heatHex(heatRatio(280, scale))).toBe(heatHex(0));
      expect(heatHex(heatRatio(340, scale))).toBe(heatHex(1));
      expect([...colours.slice(0, 3)]).not.toEqual([...colours.slice(36, 39)]);
    });

    it("maps a triangle face to its quad and lists the quad's four edges", () => {
      expect([0, 1, 2, 3, 7].map(quadOfFace)).toEqual([0, 0, 1, 1, 3]);
      expect([...quadEdges(MESH, 3)]).toEqual([4, 7, 7, 8, 8, 5, 5, 4]);
      expect(quadEdges(MESH, 4).length).toBe(0);
      expect(quadEdges(MESH, -1).length).toBe(0);
    });
  });
});
