import { beforeAll, describe, expect, it } from "vitest";
import { diagramDemo as model } from "@/test/diagram-model";
import { flowNodes } from "../canvas/flow";
import { MIN_GAP } from "./compact";
import { anchorOf, type Box, type DiagramLayout, extentOf, type Pt } from "./geometry";
import { layoutModel } from "./layout";
import { connectedNodes } from "./pack";
import { compactShape } from "./place";
import { routeWires } from "./route";
import { lineStats } from "./score";

/**
 * 自動配置を丸ごと回すテストの時間の上限。手元では 1 回 1 秒弱だが、CI の runner はほかのテストと並んで
 * 数倍遅くなり、既定の 5 秒を超えることがある（層の切り替えのテストは 3 回回す）。
 */
const LAYOUT_TIMEOUT_MS = 30_000;

const box = (l: DiagramLayout, id: string): Box => {
  const p = l.absolute.get(id);
  const b = l.boxes.get(id);
  if (!p || !b) throw new Error(`missing ${id}`);
  return { ...p, width: b.width, height: b.height };
};

describe("自動配置で並べるもの", () => {
  it("線の無い枠（構体）は線のある図に入れない", () => {
    const linked = connectedNodes(model);
    expect(linked.has("group:structure")).toBe(false);
    expect(linked.has("card:PANEL_PX")).toBe(false);
    expect(linked.has("group:power")).toBe(true);
  });
});

describe("デモの系統図の配置", () => {
  let l: DiagramLayout;
  let cards: typeof model.cards;
  beforeAll(async () => {
    l = await layoutModel(model);
    cards = model.cards.filter((c) => !c.assembly && l.absolute.has(c.id));
  }, LAYOUT_TIMEOUT_MS);

  it("カードは重ならず、枠の中に収まる", () => {
    for (const [i, a] of cards.entries())
      for (const b of cards.slice(i + 1)) {
        const [p, q] = [box(l, a.id), box(l, b.id)];
        expect(
          p.x + p.width <= q.x ||
            q.x + q.width <= p.x ||
            p.y + p.height <= q.y ||
            q.y + q.height <= p.y,
          `${a.id} overlaps ${b.id}`,
        ).toBe(true);
      }
    for (const c of cards.filter((c) => c.parent)) {
      const [inner, outer] = [box(l, c.id), box(l, c.parent ?? "")];
      expect(inner.x >= outer.x && inner.x + inner.width <= outer.x + outer.width, c.id).toBe(true);
      expect(inner.y >= outer.y && inner.y + inner.height <= outer.y + outer.height, c.id).toBe(
        true,
      );
    }
  });

  it("いちばん外の枠どうしは、縦か横に MIN_GAP 以上離れる（詰めても近づきすぎない）", () => {
    const roots = model.containers.filter((c) => !c.parent && connectedNodes(model).has(c.id));
    for (const [i, a] of roots.entries())
      for (const b of roots.slice(i + 1)) {
        const [p, q] = [box(l, a.id), box(l, b.id)];
        const apart =
          p.x + p.width + MIN_GAP <= q.x ||
          q.x + q.width + MIN_GAP <= p.x ||
          p.y + p.height + MIN_GAP <= q.y ||
          q.y + q.height + MIN_GAP <= p.y;
        expect(apart, `${a.id} is too close to ${b.id}`).toBe(true);
      }
  });

  it("枠の中は縦 1 列にも横 1 行にも並べない", () => {
    expect(compactShape(5, 1)).toBe(false);
    expect(compactShape(5, 5)).toBe(false);
    expect(compactShape(5, 2)).toBe(true);
    expect(compactShape(5, 3)).toBe(true);
    expect(compactShape(2, 1)).toBe(true);
    for (const c of model.containers.filter((c) => l.boxes.has(c.id))) {
      const kids = model.cards.filter((k) => k.parent === c.id);
      if (kids.length < 3) continue;
      const xs = new Set(kids.map((k) => box(l, k.id).x));
      const ys = new Set(kids.map((k) => box(l, k.id).y));
      expect(xs.size > 1 && ys.size > 1, c.id).toBe(true);
    }
  });

  it("列に並べない: 枠は縦にも横にも並び、全体は横長", () => {
    const roots = model.containers.filter((c) => !c.parent && connectedNodes(model).has(c.id));
    const xs = new Set(roots.map((c) => box(l, c.id).x));
    const ys = new Set(roots.map((c) => box(l, c.id).y));
    expect(xs.size).toBeGreaterThan(1);
    expect(ys.size).toBeGreaterThan(1);
    const whole = extentOf(l.absolute.keys(), l);
    expect(whole.width / whole.height).toBeGreaterThan(1.2);
    // 線の無い構体の枠は、線のある図の下か右（重ならない）
    const linked = extentOf(connectedNodes(model), l);
    const structure = box(l, "group:structure");
    expect(structure.y >= linked.y + linked.height || structure.x >= linked.x + linked.width).toBe(
      true,
    );
  });

  it("線は直交で、両端は端子の点。ほかのネットの線と重ならず、カードを横切らない", () => {
    expect(l.routes.size).toBe(model.edges.length);
    for (const e of model.edges) {
      const pts = l.routes.get(e.id) ?? [];
      expect(
        pts.every((p, i) => !i || p.x === pts[i - 1]?.x || p.y === pts[i - 1]?.y),
        e.id,
      ).toBe(true);
      for (const [which, end, at] of [
        ["source", e.source, pts[0]],
        ["target", e.target, pts.at(-1)],
      ] as const) {
        // 分かれ口のある端は分かれ口の点、ほかは端子の点
        const port = l.taps.get(e.id)?.[which] ?? l.ports.get(end.port ?? "");
        const card = l.absolute.get(end.node);
        if (!port || !card) throw new Error(`missing ${end.port}`);
        expect(at, e.id).toEqual(anchorOf({ x: card.x + port.x, y: card.y + port.y }, port.side));
      }
      // 折れ点の間の線分は、両端のカード以外のカードの内側を通らない
      const others = cards.filter((c) => c.id !== e.source.node && c.id !== e.target.node);
      for (const [i, b] of pts.slice(1).entries()) {
        const a = pts[i] as Pt;
        for (const c of others) {
          const r = box(l, c.id);
          const inside =
            Math.max(a.x, b.x) > r.x &&
            Math.min(a.x, b.x) < r.x + r.width &&
            Math.max(a.y, b.y) > r.y &&
            Math.min(a.y, b.y) < r.y + r.height;
          expect(inside, `${e.id} crosses ${c.id}`).toBe(false);
        }
      }
    }
    expect(lineStats(model, l).overlaps).toBe(0);
  });

  it("線は自分の端のカードの真裏を回り込まない（バスの裏にいる相手へは分かれ口から出す）", () => {
    for (const e of model.edges)
      for (const [which, end] of [
        ["source", e.source],
        ["target", e.target],
      ] as const) {
        const side = (l.taps.get(e.id)?.[which] ?? l.ports.get(end.port ?? ""))?.side;
        if (!side) continue;
        const c = box(l, end.node);
        // 出る側の反対の、カードの幅（高さ）の帯。ここを通る線は、カードの裏を回って戻っている
        const far = 4000;
        const behind = {
          left: { x: c.x + c.width + 16, y: c.y, width: far, height: c.height },
          right: { x: c.x - 16 - far, y: c.y, width: far, height: c.height },
          top: { x: c.x, y: c.y + c.height + 16, width: c.width, height: far },
          bottom: { x: c.x, y: c.y - 16 - far, width: c.width, height: far },
        }[side];
        const pts = l.routes.get(e.id) ?? [];
        for (const [i, b] of pts.slice(1).entries()) {
          const a = pts[i] as Pt;
          const hits =
            Math.max(a.x, b.x) > behind.x &&
            Math.min(a.x, b.x) < behind.x + behind.width &&
            Math.max(a.y, b.y) > behind.y &&
            Math.min(a.y, b.y) < behind.y + behind.height;
          expect(hits, `${e.id} ${which} goes behind ${end.node}`).toBe(false);
        }
      }
  });

  it(
    "ラベルは同じ端子・同じプロトコルの線に 1 つだけ。毎回同じ配置で、保存した位置が優先",
    async () => {
      const labelled = [...l.labels.keys()].map((id) => model.edges.find((e) => e.id === id));
      const keys = labelled.map((e) => `${e?.source.port}|${e?.label}`);
      expect(new Set(keys).size).toBe(keys.length);
      expect(keys.some((k) => k.endsWith("|CAN"))).toBe(true);
      const again = await layoutModel(model);
      expect([...again.boxes]).toEqual([...l.boxes]);
      expect([...again.routes]).toEqual([...l.routes]);
      const saved = new Map([["power", { id: "power", x: -100, y: 10, w: null, h: null }]]);
      expect(
        flowNodes(model, l, saved, { onResized: () => {} }).find((n) => n.id === "group:power")
          ?.position,
      ).toEqual({ x: -100, y: 10 });
    },
    LAYOUT_TIMEOUT_MS,
  );
});

describe("層を切り替えても位置は同じ", () => {
  it(
    "給電だけ・信号だけ・両方で、枠とカードと端子の位置が同じ（線だけ引き直す）",
    async () => {
      const only = (layer: string) => ({
        ...model,
        edges: model.edges.filter((e) => e.layer === layer),
      });
      const [power, signal, both] = await Promise.all([
        layoutModel(only("power"), model),
        layoutModel(only("signal"), model),
        layoutModel(model, model),
      ]);
      const nodes = [...model.containers, ...model.cards].map((n) => n.id);
      for (const l of [power, signal]) {
        for (const id of nodes) expect(l.absolute.get(id), id).toEqual(both.absolute.get(id));
        expect([...l.ports]).toEqual([...both.ports]);
      }
      expect(power.routes.size).toBe(only("power").edges.length);
      expect(signal.routes.size).toBe(only("signal").edges.length);
    },
    LAYOUT_TIMEOUT_MS,
  );
});

describe("線の経路探索（route.ts）", () => {
  const wire = (id: string, net: string, from: Pt, to: Pt) => ({
    id,
    net,
    from: { at: from, side: "right" as const },
    to: { at: to, side: "left" as const },
    frames: new Set<string>(),
  });

  it("間のカードは避け、まっすぐ行けるなら折れない", () => {
    const routes = routeWires(
      [
        wire("a", "N1", { x: 3, y: 48 }, { x: 397, y: 48 }),
        wire("b", "N2", { x: 3, y: 160 }, { x: 397, y: 160 }),
      ],
      [{ x: 150, y: 24, width: 100, height: 48 }],
      [],
    );
    const a = routes.get("a") ?? [];
    expect(a.length).toBeGreaterThan(2);
    expect(a.every((p) => p.x <= 150 - 8 || p.x >= 250 + 8 || p.y <= 24 - 8 || p.y >= 72 + 8)).toBe(
      true,
    );
    expect(routes.get("b")).toEqual([
      { x: 3, y: 160 },
      { x: 397, y: 160 },
    ]);
  });

  it("ほかのネットの線とは同じ所を通らず、同じネットの線は幹を共有してよい", () => {
    const other = routeWires(
      [
        wire("a", "N1", { x: 3, y: 48 }, { x: 397, y: 48 }),
        wire("b", "N2", { x: 3, y: 48 }, { x: 397, y: 48 }),
      ],
      [],
      [],
    );
    const [a, b] = [other.get("a") ?? [], other.get("b") ?? []];
    const sharedInner = a
      .slice(1, -1)
      .some((p) => b.slice(1, -1).some((q) => q.x === p.x && q.y === p.y));
    expect(JSON.stringify(a) === JSON.stringify(b) || sharedInner).toBe(false);
    const same = routeWires(
      [
        wire("a", "N1", { x: 3, y: 48 }, { x: 397, y: 48 }),
        wire("b", "N1", { x: 3, y: 48 }, { x: 397, y: 48 }),
      ],
      [],
      [],
    );
    expect(same.get("a")).toEqual(same.get("b"));
  });
});

describe("線の混み具合（score.ts）", () => {
  const layout = (routes: [string, Pt[]][]) => ({
    boxes: new Map(),
    absolute: new Map(),
    ports: new Map(),
    taps: new Map(),
    labels: new Map(),
    routes: new Map(routes),
  });
  const edges = (nets: [string, string]) =>
    ({
      ...model,
      edges: [
        {
          ...model.edges[0],
          id: "a",
          nets: [nets[0]],
          source: { node: "x", port: "x1" },
          target: { node: "y", port: "y1" },
        },
        {
          ...model.edges[0],
          id: "b",
          nets: [nets[1]],
          source: { node: "z", port: "z1" },
          target: { node: "w", port: "w1" },
        },
      ],
    }) as typeof model;

  it("交差・重なり・折れ・長さを数え、同じネットの線どうしは数えない", () => {
    const cross = layout([
      [
        "a",
        [
          { x: 0, y: 50 },
          { x: 100, y: 50 },
        ],
      ],
      [
        "b",
        [
          { x: 50, y: 0 },
          { x: 50, y: 100 },
        ],
      ],
    ]);
    expect(lineStats(edges(["N1", "N2"]), cross)).toEqual({
      crossings: 1,
      overlaps: 0,
      bends: 0,
      length: 200,
    });
    expect(lineStats(edges(["N1", "N1"]), cross).crossings).toBe(0);
    const overlap = layout([
      [
        "a",
        [
          { x: 0, y: 50 },
          { x: 100, y: 50 },
        ],
      ],
      [
        "b",
        [
          { x: 50, y: 50 },
          { x: 150, y: 50 },
        ],
      ],
    ]);
    expect(lineStats(edges(["N1", "N2"]), overlap).overlaps).toBe(1);
    expect(lineStats(edges(["N1", "N1"]), overlap).overlaps).toBe(0);
  });
});
