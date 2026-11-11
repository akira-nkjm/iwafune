import type { DiagramModel } from "../model";
import {
  type Box,
  CARD_H,
  CARD_W,
  GRID,
  type Placement,
  PORT_GAP,
  type Pt,
  type Side,
  type Size,
  snapDown,
  snapUp,
  type Taps,
} from "./geometry";

/** カードの端子につながる線の端 1 つと、その線の相手の位置 */
interface End {
  edge: string;
  end: "source" | "target";
  port: string;
  partner: Pt;
}

/** 線の端ごとの相手の位置（カードの中心。ジャンクションはそこにつながるほかのカードの中心の平均） */
function endsOf(model: DiagramModel, placement: Placement): End[] {
  const center = (id: string): Pt | null => {
    const p = placement.absolute.get(id);
    const b = placement.boxes.get(id);
    return p && b ? { x: p.x + b.width / 2, y: p.y + b.height / 2 } : null;
  };
  const junctionPoints = new Map<string, Pt[]>();
  for (const e of model.edges)
    for (const [j, other] of [
      [e.source.node, e.target.node],
      [e.target.node, e.source.node],
    ] as const) {
      const c = center(other);
      if (!placement.absolute.has(j) && c)
        junctionPoints.set(j, [...(junctionPoints.get(j) ?? []), c]);
    }
  const at = (id: string): Pt | null => {
    const pts = junctionPoints.get(id) ?? [];
    return center(id) ?? (pts.length === 0 ? null : mean(pts));
  };
  return model.edges.flatMap((e) =>
    (
      [
        ["source", e.source.port, e.target.node],
        ["target", e.target.port, e.source.node],
      ] as const
    ).flatMap(([end, port, other]) => {
      const partner = port ? at(other) : null;
      return port && partner ? [{ edge: e.id, end, port, partner }] : [];
    }),
  );
}

const mean = (pts: Pt[]): Pt => ({
  x: pts.reduce((s, p) => s + p.x, 0) / pts.length,
  y: pts.reduce((s, p) => s + p.y, 0) / pts.length,
});

const NORMAL: Record<Side, Pt> = {
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  top: { x: 0, y: -1 },
  bottom: { x: 0, y: 1 },
};

/** 相手 `m` がカード `b`（絶対座標）の外のどちらにいちばん大きく離れているか（横なら左右、縦なら上下） */
function sideToward(b: Box, m: Pt): Side {
  const dx = m.x - (b.x + b.width / 2);
  const dy = m.y - (b.y + b.height / 2);
  return Math.abs(dx) - b.width / 2 >= Math.abs(dy) - b.height / 2
    ? dx >= 0
      ? "right"
      : "left"
    : dy >= 0
      ? "bottom"
      : "top";
}

/** 相手が側 `side` の反対側（カードの中心より裏）にいるか。そこへ線を引くとカードを回り込む */
const behind = (b: Box, side: Side, m: Pt) =>
  (m.x - (b.x + b.width / 2)) * NORMAL[side].x + (m.y - (b.y + b.height / 2)) * NORMAL[side].y < 0;

/** 端子の向きと、分かれ口（タップ）の向き */
export interface PortPlan {
  /** 端子 → 側 */
  sides: Map<string, Side>;
  /** 線の ID → その線の端を、端子とは別の側に出すときの側 */
  taps: Map<string, { source?: Side; target?: Side }>;
}

/**
 * 端子の側と分かれ口。端子は、線の相手（の平均）がカードの外のどちらにいちばん大きく離れているかで側を決める
 * （線の無い端子は左）。何本も線が出る端子（バス）で、相手が端子の側の反対側にいる線は、その相手のいる側に
 * 分かれ口を作ってそこから出す（同じ端子の線なので、端の名前も同じ）。カードを大回りして裏の端子へ戻らないように。
 * `overrides` は線を引いて選び直した側（refine.ts）で、相手の方向より優先する。
 */
export function planPorts(
  model: DiagramModel,
  placement: Placement,
  overrides: ReadonlyMap<string, Side> = new Map(),
): PortPlan {
  const plan: PortPlan = { sides: new Map(), taps: new Map() };
  const ends = endsOf(model, placement);
  for (const card of model.cards) {
    const p = placement.absolute.get(card.id);
    const size = placement.boxes.get(card.id);
    if (!p || !size) continue;
    const b = { ...p, width: size.width, height: size.height };
    for (const port of card.ports) {
      const mine = ends.filter((e) => e.port === port.id);
      if (mine.length === 0) {
        plan.sides.set(port.id, "left");
        continue;
      }
      const side = overrides.get(port.id) ?? sideToward(b, mean(mine.map((e) => e.partner)));
      plan.sides.set(port.id, side);
      if (mine.length < 2) continue;
      for (const e of mine) {
        if (!behind(b, side, e.partner)) continue;
        const tap = sideToward(b, e.partner);
        if (tap !== side) plan.taps.set(e.edge, { ...plan.taps.get(e.edge), [e.end]: tap });
      }
    }
  }
  return plan;
}

/**
 * 同じ側に並ぶ端子の間隔 [px]。上下の側は端子の名前（線の端のラベル。等幅 9.5px で 1 文字およそ 6px）を線の右に
 * 置くので、いちばん長い名前が隣の線に届かない間隔にする。左右の側は名前（高さおよそ 12px）を線の上に置くので、
 * PORT_GAP より 1 格子広く。
 */
function gapOf(card: DiagramModel["cards"][number], side: Side): number {
  if (side === "left" || side === "right") return PORT_GAP + GRID;
  const longest = Math.max(0, ...card.ports.map((p) => p.port.length));
  return Math.max(PORT_GAP, snapUp(longest * 6 + 14));
}

/** 側ごとに並べるもの: 端子か分かれ口と、並べる順を決める線の相手の位置 */
interface Item {
  kind: "port" | "tap";
  id: string;
  end?: "source" | "target";
  order: number;
  partners: Pt[];
}

function itemsOf(
  card: DiagramModel["cards"][number],
  plan: PortPlan,
  ends: End[],
): Map<Side, Item[]> {
  const out = new Map<Side, Item[]>();
  const add = (side: Side, item: Item) => out.set(side, [...(out.get(side) ?? []), item]);
  card.ports.forEach((port, order) => {
    const mine = ends.filter((e) => e.port === port.id);
    const tapOf = (e: End) => plan.taps.get(e.edge)?.[e.end];
    add(plan.sides.get(port.id) ?? "left", {
      kind: "port",
      id: port.id,
      order,
      partners: mine.filter((e) => !tapOf(e)).map((e) => e.partner),
    });
    for (const e of mine) {
      const side = tapOf(e);
      if (side) add(side, { kind: "tap", id: e.edge, end: e.end, order, partners: [e.partner] });
    }
  });
  return out;
}

/** 端子と分かれ口の数に合わせたカードの大きさ（同じ側に並ぶものが gapOf の間隔で収まる大きさ。最小は CARD_W × CARD_H） */
export function cardSizes(
  model: DiagramModel,
  placement: Placement,
  plan: PortPlan,
): Map<string, Size> {
  const ends = endsOf(model, placement);
  const out = new Map<string, Size>();
  for (const card of model.cards) {
    const items = itemsOf(card, plan, ends);
    const count = (side: Side) => items.get(side)?.length ?? 0;
    out.set(card.id, {
      width: Math.max(CARD_W, Math.max(count("top"), count("bottom")) * gapOf(card, "top")),
      height: Math.max(CARD_H, Math.max(count("left"), count("right")) * gapOf(card, "left")),
    });
  }
  return out;
}

/**
 * 端子と分かれ口の位置（カードの左上からの相対、端子の中心）。同じ側に並ぶものは、線の相手の位置の順（左右の側は
 * 上から、上下の側は左から）に gapOf の間隔で、側の中ほどに並べる。線がカードのすぐ外で交わらないようにするため。
 * `offsets` は線を引いて選び直した端子の位置（側の上の位置。refine.ts）で、側からはみ出さず、同じ側のほかの
 * 端子から gapOf 以上離れるときだけ使う（名前が重ならないように）。
 */
export function portPositions(
  model: DiagramModel,
  placement: Placement,
  plan: PortPlan,
  offsets: ReadonlyMap<string, number> = new Map(),
): { ports: Map<string, Pt & { side: Side }>; taps: Taps } {
  const ends = endsOf(model, placement);
  const ports = new Map<string, Pt & { side: Side }>();
  const taps: Taps = new Map();
  for (const card of model.cards) {
    const b = placement.boxes.get(card.id);
    if (!b) continue;
    for (const [side, items] of itemsOf(card, plan, ends)) {
      const vertical = side === "left" || side === "right";
      const key = (it: Item) =>
        it.partners.length > 0 ? (vertical ? mean(it.partners).y : mean(it.partners).x) : 0;
      const sorted = items.toSorted((a, c) => key(a) - key(c) || a.order - c.order);
      const length = vertical ? b.height : b.width;
      const gap = gapOf(card, side);
      const start = snapDown((length - (sorted.length - 1) * gap) / 2);
      const alongs = sorted.map((_, i) => start + i * gap);
      sorted.forEach((it, i) => {
        const wanted = it.kind === "port" ? offsets.get(it.id) : undefined;
        const fits =
          wanted !== undefined &&
          wanted >= GRID &&
          wanted <= length - GRID &&
          alongs.every((a, j) => j === i || Math.abs(a - wanted) >= gap);
        if (fits) alongs[i] = wanted;
      });
      sorted.forEach((it, i) => {
        const along = alongs[i] ?? start;
        const at = {
          x: side === "left" ? 0 : side === "right" ? b.width : along,
          y: side === "top" ? 0 : side === "bottom" ? b.height : along,
          side,
        };
        if (it.kind === "port") ports.set(it.id, at);
        else if (it.end) taps.set(it.id, { ...taps.get(it.id), [it.end]: at });
      });
    }
  }
  return { ports, taps };
}
