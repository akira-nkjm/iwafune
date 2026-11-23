import { describe, expect, it } from "vitest";
import type { OrbitPlaneTrack } from "@/api/types";
import { insetLayout, orbitView, satelliteAt } from "./insetGeometry";

const R = 6_000_000;
/** 半径 2R の円軌道の 4 点（表示枠: +x が太陽側）。最後の 2 点は日陰 */
const plane: OrbitPlaneTrack = {
  x: [2 * R, 0, -2 * R, 0],
  y: [0, 2 * R, 0, -2 * R],
  eclipse: [false, false, true, true],
  sun_dir_body: [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
    [0, 0, -1],
  ],
  nadir_body: [
    [0, 0, 1],
    [0, 0, 1],
    [0, 0, 1],
    [0, 0, 1],
  ],
  body_radius: R,
  sun_elevation: 0.5,
  shadow_half_width: R,
};

describe("insetLayout", () => {
  const layout = insetLayout(plane, 200)!;
  it("一様な倍率で収める: 軌道の端が太陽の印の余白の内側、中心天体と影の帯の幅は同じ倍率", () => {
    // 中心 100、余白 20 → 軌道の半径は 80 px。倍率は 80 / 2R
    expect(layout.scale).toBeCloseTo(80 / (2 * R), 12);
    expect(layout.bodyRadius).toBeCloseTo(40, 6);
    expect(layout.shadow).toEqual({ x: 0, y: 60, width: 100, height: 80 });
    expect(layout.sun).toEqual({ x: 190, y: 100 });
  });
  it("y は画面に合わせて反転し、点の順に折れ線にする", () => {
    expect(layout.path).toBe("M180 100L100 20L20 100L100 180");
  });
  it("標本・半径が無ければ null", () => {
    expect(insetLayout({ ...plane, x: [], y: [] })).toBeNull();
    expect(insetLayout({ ...plane, body_radius: 0 })).toBeNull();
  });
  it("軌道が中心天体より内側でも、天体が収まる（天体の半径を下限にする）", () => {
    const small = insetLayout({ ...plane, x: [R / 2], y: [0] }, 200)!;
    expect(small.bodyRadius).toBeCloseTo(80, 6);
  });
});

describe("satelliteAt", () => {
  const layout = insetLayout(plane, 200)!;
  it("添字の標本の位置と日陰を返す（添字は範囲に収める）", () => {
    expect(satelliteAt(layout, plane, 0)).toEqual({ x: 180, y: 100, eclipse: false });
    expect(satelliteAt(layout, plane, 2)).toEqual({ x: 20, y: 100, eclipse: true });
    expect(satelliteAt(layout, plane, 99).y).toBe(180);
    expect(satelliteAt(layout, plane, -3).x).toBe(180);
  });
});

describe("orbitView", () => {
  it("非定常のときだけ、同じ添字の太陽方向（機体座標系のまま）と日陰を返す", () => {
    expect(orbitView({ kind: "worst" }, plane, 0)).toEqual({ plane: null, sun: null });
    expect(orbitView({ kind: "case", id: "hot" }, plane, 0)).toEqual({ plane: null, sun: null });
    expect(orbitView({ kind: "transient", index: 0 }, undefined, 0)).toEqual({
      plane: null,
      sun: null,
    });
    const at = (i: number) => orbitView({ kind: "transient", index: i }, plane, i);
    expect(at(1)).toEqual({ plane, sun: { dir: [0, 1, 0], eclipse: false } });
    expect(at(3).sun).toEqual({ dir: [0, 0, -1], eclipse: true });
    expect(at(10).sun?.dir).toEqual([0, 0, -1]);
  });
  it("軌道面の値が空（古い結果）なら何も出さない", () => {
    const empty = { ...plane, x: [], y: [], eclipse: [], sun_dir_body: [] };
    expect(orbitView({ kind: "transient", index: 0 }, empty, 0)).toEqual({
      plane: null,
      sun: null,
    });
  });
});
