import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { buildModel } from "../model";
import {
  addBend,
  attachTo,
  followEnds,
  moveSegment,
  orthogonal,
  reattach,
  removeBend,
  storedRoute,
  tidyRoute,
  waypointKey,
} from "./routes";

const isOrthogonal = (pts: { x: number; y: number }[]) =>
  pts.every((p, i) => !i || p.x === pts[i - 1]?.x || p.y === pts[i - 1]?.y);
describe("手で引いた線の経路", () => {
  it("線のキーは並べ直しやネットの順に依らず、区切り文字を含む ID でも取り違えない", () => {
    const edges = buildModel(fixture, {
      layers: new Set(["power", "signal"]),
      mergeSiblings: true,
    }).edges;
    const edge = edges[0];
    if (!edge) throw new Error("missing edge");
    expect(waypointKey(edge)).toBe(
      waypointKey({ ...edge, id: "different layout index", nets: [...edge.nets].reverse() }),
    );
    expect(waypointKey({ ...edge, nets: ["a:b", "c"] })).not.toBe(
      waypointKey({ ...edge, nets: ["a", "b:c"] }),
    );
  });
  it("区間のドラッグと折れの追加・削除で、端の点と直交が保たれる", () => {
    const pts = [
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 100 },
    ];
    for (const next of [
      moveSegment(pts, 0, { x: 50, y: 20 }),
      moveSegment(pts, 1, { x: 80, y: 50 }),
      addBend(pts, 0),
      orthogonal([
        { x: 0, y: 0 },
        { x: 60, y: 80 },
      ]),
    ]) {
      expect(isOrthogonal(next)).toBe(true);
      expect(next[0]).toEqual(pts[0]);
    }
    expect(removeBend(addBend(pts, 0), 2).length).toBeLessThan(addBend(pts, 0).length);
    const moved = moveSegment(pts, 0, { x: 50, y: 20 });
    expect(moved.at(-1)).toEqual(pts.at(-1));
    expect(storedRoute(moved).points.every((p) => p.every(Number.isInteger))).toBe(true);
  });

  it("キーは `ネット:始点の端子>終点の端子` の読める形", () => {
    const edge = buildModel(fixture, { layers: new Set(["power"]), mergeSiblings: false }).edges[0];
    if (!edge) throw new Error("missing edge");
    expect(waypointKey(edge)).toBe(`${edge.nets.join("+")}:${edge.ends}`);
    expect(edge.ends).toMatch(/^[A-Za-z0-9_]+\.[A-Za-z0-9_]+>[A-Za-z0-9_]+\.[A-Za-z0-9_]+$/);
  });

  it("保存する前に、座標の端数でできた 1 px の段差と一直線上の途中の点を除く", () => {
    const pts = [
      { x: 408, y: 506.67 },
      { x: 468, y: 506.67 },
      { x: 468, y: 506.17 },
      { x: 468, y: 350 },
      { x: 742, y: 350 },
    ];
    expect(tidyRoute(pts)).toEqual([
      { x: 408, y: 507 },
      { x: 468, y: 507 },
      { x: 468, y: 350 },
      { x: 742, y: 350 },
    ]);
  });

  it("端はカードのいちばん近い側の、4 px 刻みの位置に付く（縁からはみ出さない）", () => {
    const card = { x: 100, y: 50, width: 150, height: 52 };
    expect(attachTo(card, { x: 251, y: 71 })).toEqual({
      side: "right",
      offset: 20,
      at: { x: 250, y: 70 },
    });
    expect(attachTo(card, { x: 137, y: 45 })).toEqual({
      side: "top",
      offset: 36,
      at: { x: 136, y: 50 },
    });
    // カードの外の遠い点でも、いちばん近い側（下）の縁の端に収まる
    expect(attachTo(card, { x: 400, y: 200 })).toEqual({
      side: "bottom",
      offset: 150,
      at: { x: 250, y: 102 },
    });
  });

  it("端を付け替えると、側の外へ短い足を出してから残りの経路へ直角につなぐ", () => {
    const route = [
      { x: 250, y: 76 },
      { x: 300, y: 76 },
      { x: 300, y: 200 },
      { x: 400, y: 200 },
    ];
    const next = reattach(route, "source", { x: 136, y: 50 }, "top");
    expect(next[0]).toEqual({ x: 136, y: 50 });
    expect(next[1]).toEqual({ x: 136, y: 34 });
    expect(next.at(-1)).toEqual({ x: 400, y: 200 });
    expect(next.every((p, i) => !i || p.x === next[i - 1]?.x || p.y === next[i - 1]?.y)).toBe(true);
    const back = reattach(route, "target", { x: 420, y: 180 }, "left");
    expect(back[0]).toEqual({ x: 250, y: 76 });
    expect(back.at(-1)).toEqual({ x: 420, y: 180 });
  });

  it("端のカードを動かしたら、手で引いた経路の端はいまの端子から外へ出てつなぎ直す（引き返さない）", () => {
    // プレビューで起きた形: PCDU を上へ動かしたあとの 12 V の線（保存した始点は 419,479）
    const stored = [
      { x: 419, y: 479 },
      { x: 458, y: 479 },
      { x: 458, y: 170 },
      { x: 742, y: 170 },
      { x: 742, y: 289 },
      { x: 815, y: 289 },
    ];
    const end = { at: { x: 815, y: 289 }, side: "left" as const };
    const drawn = followEnds(stored, { at: { x: 407, y: 282 }, side: "right" }, end);
    expect(drawn.slice(0, 4)).toEqual([
      { x: 407, y: 282 },
      { x: 423, y: 282 },
      { x: 458, y: 282 },
      { x: 458, y: 170 },
    ]);
    // 縦の区間が向きを変えて引き返していない（458,479 を通らない）
    expect(drawn.some((p) => p.y === 479)).toBe(false);
    expect(drawn.at(-1)).toEqual({ x: 815, y: 289 });
    // 動かしていなければ保存した経路のまま（端数のずれは見逃す）
    expect(followEnds(stored, { at: { x: 419.6, y: 478.8 }, side: "right" }, end)).toEqual(stored);
  });

  it("付け替えた端の続きがカードの向こう側なら、カードの脇を回る（突き抜けない）", () => {
    // 下の端子から下へ出ていた線の始点を、カードの上の縁へ付け替える
    const card = { x: 100, y: 100, width: 150, height: 52 };
    const route = [
      { x: 160, y: 152 },
      { x: 160, y: 200 },
      { x: 400, y: 200 },
    ];
    const next = reattach(route, "source", { x: 200, y: 100 }, "top", card);
    expect(next[0]).toEqual({ x: 200, y: 100 });
    expect(next[1]).toEqual({ x: 200, y: 84 });
    const inside = (p: { x: number; y: number }) =>
      p.x > card.x && p.x < card.x + card.width && p.y > card.y && p.y < card.y + card.height;
    for (const [i, b] of next.slice(1).entries()) {
      const a = next[i] as { x: number; y: number };
      expect(inside({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }), JSON.stringify([a, b])).toBe(
        false,
      );
    }
    expect(next.at(-1)).toEqual({ x: 400, y: 200 });
  });
});
