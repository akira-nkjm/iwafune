import type { DiagramModel } from "../model";
import {
  anchorOf,
  type Box,
  type DiagramLayout,
  GRID,
  PORT_GAP,
  type Pt,
  type Side,
  snapDown,
} from "./geometry";
import type { Tree } from "./pack";
import { type Alternative, compareEnds, type Router } from "./route";
import { absBox, wiresOf } from "./wires";

const SIDES: readonly Side[] = ["left", "right", "top", "bottom"];
/** 端子を変えるのは、線の費用がこれより下がるときだけ（1 格子 1） */
const MARGIN = 6;

/** 選び直した端子: 側と、側の上の位置（左か上の角からの距離 [px]） */
export interface PortChoice {
  sides: Map<string, Side>;
  offsets: Map<string, number>;
}

/** 選び直しの案 1 つ: 端子と、新しい側か位置と、その端子の線の費用がどれだけ下がるか */
export interface Proposal {
  port: string;
  side?: Side;
  offset?: number;
  gain: number;
}

/** カードにつながる線の端 1 つ（線・どちらの端か・端子・カード） */
interface PortEnd {
  wire: string;
  end: "from" | "to";
  port: string;
  card: Box;
  at: Pt & { side: Side };
}

/** 分かれ口でもジャンクションでもない、端子につながる線の端 */
function portEnds(
  model: DiagramModel,
  layout: DiagramLayout,
  routed: ReadonlySet<string>,
): PortEnd[] {
  return model.edges.flatMap((e) =>
    (
      [
        ["from", e.source],
        ["to", e.target],
      ] as const
    ).flatMap(([end, at]) => {
      const port = at.port ? layout.ports.get(at.port) : undefined;
      const tapped = layout.taps.get(e.id)?.[end === "from" ? "source" : "target"];
      const card = absBox(layout, at.node);
      return at.port && port && card && !tapped && routed.has(e.id)
        ? [{ wire: e.id, end, port: at.port, card, at: port }]
        : [];
    }),
  );
}

/** 端子の線の費用の合計を、候補ごとにまとめる（\`keyOf\` が同じ候補は同じ選び方） */
function bestPerPort<K>(
  ends: PortEnd[],
  base: Map<string, number>,
  alternatives: (Alternative & { port: string; key: K })[],
  costs: number[],
): { port: string; key: K; gain: number }[] {
  const totals = new Map<string, Map<K, number>>();
  alternatives.forEach((a, i) => {
    const t = totals.get(a.port) ?? new Map<K, number>();
    t.set(a.key, (t.get(a.key) ?? 0) + (costs[i] ?? 0));
    totals.set(a.port, t);
  });
  const out: { port: string; key: K; gain: number }[] = [];
  for (const [port, byKey] of totals) {
    const mine = ends.filter((e) => e.port === port);
    // 端子の線が全部そろった候補だけを比べる（一部の線だけ付け替える候補は選ばない）
    const counts = new Map<K, number>();
    for (const a of alternatives.filter((x) => x.port === port))
      counts.set(a.key, (counts.get(a.key) ?? 0) + 1);
    const current = mine.reduce((s, e) => s + (base.get(e.wire) ?? 0), 0);
    let best: [K, number] | null = null;
    for (const [key, cost] of byKey)
      if (counts.get(key) === mine.length && (!best || cost < best[1])) best = [key, cost];
    if (best && best[1] < current - MARGIN)
      out.push({ port, key: best[0], gain: current - best[1] });
  }
  return out;
}

/**
 * 端子の選び直し。端子の側は線の相手の中心の方向で、側の上の位置は相手の順で決めるが、実際の線は間のカードや同じ
 * ネットの幹を回るので、別の側から出したり、側の上で少しずらしたりしたほうが、短く・折れが少なく引けることがある。
 * 全部の線を引いたうえで、端子ごとに次を比べ（その端子の線だけを引き直す。ほかの線はそのまま。長さ・折れ・交差の費用。
 * route.ts）、MARGIN より安くなる案を返す。分かれ口の線とジャンクションの端は選び直さない。
 * - 側: ほかの 3 つの側の、ほかの端子と重ならない位置のうち線の相手にいちばん近い所から出す
 * - 位置: 端子を出てすぐ横へずれてまた外へ向かう線（段）があれば、段の分だけ端子をずらしてまっすぐにする
 */
export function refinePorts(
  model: DiagramModel,
  tree: Tree,
  geometry: DiagramLayout,
  mode: "sides" | "offsets",
  routed?: Router,
): Proposal[] {
  const layout: DiagramLayout = {
    ...geometry,
    boxes: new Map(geometry.boxes),
    absolute: new Map(geometry.absolute),
  };
  const { wires, cards, frames } = wiresOf(model, tree, layout);
  const ends = portEnds(model, layout, new Set(wires.map((w) => w.id)));
  if (ends.length === 0) return [];
  if (mode === "sides") {
    const partner = (e: PortEnd) => {
      const w = wires.find((x) => x.id === e.wire);
      return (e.end === "from" ? w?.to.at : w?.from.at) ?? middleOf(e.card, e.at.side);
    };
    const alts = ends.flatMap((e) =>
      SIDES.filter((s) => s !== e.at.side).map((side) => ({
        wire: e.wire,
        end: e.end,
        terminal: { at: anchorOf(freeSpot(model, geometry, e, side, partner(e)), side), side },
        port: e.port,
        key: side,
      })),
    );
    const { base, costs } = compareEnds(wires, cards, frames, () => alts, routed);
    return bestPerPort(ends, base, alts, costs).map((p) => ({
      port: p.port,
      side: p.key,
      gain: p.gain,
    }));
  }
  {
    let alts: (Alternative & { port: string; key: number })[] = [];
    const { base, costs } = compareEnds(
      wires,
      cards,
      frames,
      (routes) => {
        alts = ends.flatMap((e) => {
          const along = straightened(e, routes.get(e.wire) ?? []);
          if (along === null) return [];
          const vertical = e.at.side === "left" || e.at.side === "right";
          const local = vertical ? { x: e.at.x, y: along } : { x: along, y: e.at.y };
          const at = anchorOf({ x: e.card.x + local.x, y: e.card.y + local.y }, e.at.side);
          return [
            {
              wire: e.wire,
              end: e.end,
              terminal: { at, side: e.at.side },
              port: e.port,
              key: along,
            },
          ];
        });
        return alts;
      },
      routed,
    );
    return bestPerPort(ends, base, alts, costs).map((p) => ({
      port: p.port,
      offset: p.key,
      gain: p.gain,
    }));
  }
}

/**
 * 側 `side` の上で、同じカードのほかの端子から PORT_GAP 以上離れた位置のうち、線の相手（`toward`）にいちばん近い点
 * （絶対座標）。空きが無ければ並んだ端子のすぐ外側（カードが大きくなるのを見越す）。側を変えたときにほかの端子と
 * 重なって高く見積もらないように。
 */
function freeSpot(
  model: DiagramModel,
  geometry: DiagramLayout,
  e: PortEnd,
  side: Side,
  toward: Pt,
): Pt {
  const vertical = side === "left" || side === "right";
  const length = vertical ? e.card.height : e.card.width;
  const card = model.cards.find((c) => c.ports.some((p) => p.id === e.port));
  const taken = (card?.ports ?? [])
    .filter((p) => p.id !== e.port)
    .flatMap((p) => {
      const at = geometry.ports.get(p.id);
      return at && at.side === side ? [vertical ? at.y : at.x] : [];
    });
  const target = vertical ? toward.y - e.card.y : toward.x - e.card.x;
  let best: number | null = null;
  for (let a = GRID; a <= length - GRID; a += GRID) {
    if (taken.some((t) => Math.abs(t - a) < PORT_GAP)) continue;
    if (best === null || Math.abs(a - target) < Math.abs(best - target)) best = a;
  }
  // 空きが無ければ、並んだ端子の外側（端子が増えるとカードは大きくなるので、その大きさを見越した位置）
  if (best === null && taken.length > 0) {
    const below = Math.max(...taken) + PORT_GAP;
    const above = Math.min(...taken) - PORT_GAP;
    best = Math.abs(below - target) <= Math.abs(above - target) ? below : above;
  }
  if (best === null) return middleOf(e.card, side);
  return vertical
    ? { x: e.card.x + (side === "right" ? e.card.width : 0), y: e.card.y + best }
    : { x: e.card.x + best, y: e.card.y + (side === "bottom" ? e.card.height : 0) };
}

/** 側の中ほどの点（GRID に乗せる。絶対座標） */
function middleOf(card: Box, side: Side): Pt {
  return side === "left" || side === "right"
    ? { x: card.x + (side === "right" ? card.width : 0), y: card.y + snapDown(card.height / 2) }
    : { x: card.x + snapDown(card.width / 2), y: card.y + (side === "bottom" ? card.height : 0) };
}

/**
 * 端子を出てすぐ横へずれ（段）、また外へ向かう線なら、段をなくす端子の位置（側の上の位置。左か上の角からの距離）。
 * 段が無いか、ずらすと側からはみ出すときは null。
 */
function straightened(e: PortEnd, route: Pt[]): number | null {
  const pts = e.end === "from" ? route : [...route].reverse();
  const [p0, p1, p2, p3] = pts;
  if (!p0 || !p1 || !p2 || !p3) return null;
  const vertical = e.at.side === "left" || e.at.side === "right";
  const n = { left: -1, right: 1, top: -1, bottom: 1 }[e.at.side];
  // 外へ出る → 側に沿ってずれる → また外へ、の 3 区間
  const out1 = vertical
    ? (p1.x - p0.x) * n > 0 && p1.y === p0.y
    : (p1.y - p0.y) * n > 0 && p1.x === p0.x;
  const shift = vertical ? p2.x === p1.x && p2.y !== p1.y : p2.y === p1.y && p2.x !== p1.x;
  const out2 = vertical
    ? (p3.x - p2.x) * n > 0 && p3.y === p2.y
    : (p3.y - p2.y) * n > 0 && p3.x === p2.x;
  if (!out1 || !shift || !out2) return null;
  const along = vertical ? e.at.y + (p2.y - p1.y) : e.at.x + (p2.x - p1.x);
  const length = vertical ? e.card.height : e.card.width;
  return along >= 8 && along <= length - 8 ? along : null;
}
