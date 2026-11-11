import type { DiagramModel } from "../model";
import { compact } from "./compact";
import { wireEnds } from "./estimate";
import { CARD_H, CARD_W, type DiagramLayout, type Pt, type Size, TITLE_H } from "./geometry";
import { gridUnlinked, placeUnlinked } from "./grid";
import { placeLabels } from "./labels";
import { connectedNodes, type Tree, treeOf } from "./pack";
import { type Arrangement, arrangements, place } from "./place";
import { cardSizes, planPorts, portPositions } from "./ports";
import { type PortChoice, type Proposal, refinePorts } from "./refine";
import { routeWires } from "./route";
import { layoutCost } from "./score";
import { wiresOf } from "./wires";

/** 線まで引いて比べる並べ方の候補の数（見積もりの小さい順） */
const CANDIDATES = 6;
const base = (): Size => ({ width: CARD_W, height: CARD_H });

/** 同じ端子から出る同じプロトコルの線のうち、ラベルを付ける 1 本（ID 順で最初） */
function labelledEdges(model: DiagramModel): Set<string> {
  const first = new Map<string, string>();
  for (const e of model.edges.toSorted((a, b) => a.id.localeCompare(b.id))) {
    if (!e.label) continue;
    const key = `${e.source.port ?? e.source.node}|${e.label}`;
    if (!first.has(key)) first.set(key, e.id);
  }
  return new Set(first.values());
}

/** ラベルの位置（labels.ts）。避けるのはカードと、枠の見出しの帯 */
function labelPositions(
  model: DiagramModel,
  layout: DiagramLayout,
  labelled: ReadonlySet<string>,
): Map<string, Pt> {
  const box = (id: string, height?: number) => {
    const p = layout.absolute.get(id);
    const b = layout.boxes.get(id);
    return p && b ? [{ ...p, width: b.width, height: height ?? b.height }] : [];
  };
  const obstacles = [
    ...model.cards.flatMap((c) => box(c.id)),
    ...model.containers.flatMap((c) => box(c.id, TITLE_H)),
  ];
  const texts = new Map(
    model.edges.flatMap((e) => (e.label && labelled.has(e.id) ? [[e.id, e.label] as const] : [])),
  );
  return placeLabels(layout.routes, texts, obstacles);
}

/**
 * 1 つの並べ方で、カードと枠の位置・カードの大きさ・端子の位置まで決める（線はまだ引かない）。枠の詰めは
 * compact.ts。図のどの層を表示しても位置が変わらないよう、全部の層の線（`full`）で決める。
 */
function arrange(
  full: DiagramModel,
  tree: Tree,
  arr: Arrangement,
  choice?: PortChoice,
): DiagramLayout {
  const ends = wireEnds(full);
  const first = place(tree, arr, base);
  compact(tree, first, ends);
  const plan = planPorts(full, first, choice?.sides);
  const sizes = cardSizes(full, first, plan);
  const placed = place(tree, arr, (id) => sizes.get(id) ?? base());
  compact(tree, placed, ends);
  const { ports, taps } = portPositions(full, placed, plan, choice?.offsets);
  return {
    boxes: placed.boxes,
    absolute: placed.absolute,
    ports,
    taps,
    routes: new Map(),
    labels: new Map(),
  };
}

/** 配置（`geometry`。書き換えない）の上に、表示する線（`model`）のジャンクションと経路を置く */
function wire(model: DiagramModel, tree: Tree, geometry: DiagramLayout): DiagramLayout {
  const layout = copyOf(geometry);
  const { wires, cards, frames } = wiresOf(model, tree, layout);
  const unrouted = new Set<string>();
  layout.routes = routeWires(wires, cards, frames, unrouted);
  layout.unrouted = unrouted;
  return layout;
}

/** 配置の写し（箱と位置の表は新しく作る。ジャンクションを書き足しても元を変えない） */
const copyOf = (geometry: DiagramLayout): DiagramLayout => ({
  boxes: new Map(geometry.boxes),
  absolute: new Map(geometry.absolute),
  ports: geometry.ports,
  taps: geometry.taps,
  routes: new Map(),
  labels: new Map(),
});

/** 線を引く前の配置。層の表示を切り替えても使い回す（位置を共通に保つため） */
export interface LayoutPlan {
  tree: Tree;
  geometry: DiagramLayout;
}

/**
 * 系統図の配置を決める。列や層は決めない: サブシステムの枠を縦横の格子に、カードを枠の中の格子に置き、線の長さと
 * 交差の見積もりがいちばん小さくなる置き方を探す（place.ts）。見積もりの小さい候補をいくつか選んで線を引き
 * （route.ts）、交差・重なり・折れ・長さ・全体の形の費用（score.ts）がいちばん小さいものを使う。
 * 線の無い枠（構体など）は図の下か右に格子で並べる（grid.ts）。`full` は全部の層の線を入れた表示モデルで、
 * どの層を表示しても同じ位置になる。座標は表示のためだけのもので、設計データには書かない（手で動かした位置と
 * 線の経路は view/electrical.toml に保存され、ここで決めた位置より優先される）。
 */
export function planLayout(full: DiagramModel): LayoutPlan {
  const inGraph = connectedNodes(full);
  const tree = treeOf(full, inGraph);
  const unlinked = gridUnlinked(full, inGraph);
  let best: { geometry: DiagramLayout; cost: number; arr: Arrangement } | null = null;
  const candidates = arrangements(full, tree, base, unlinked.roots)
    .toSorted((a, b) => a.estimate - b.estimate)
    .slice(0, CANDIDATES);
  for (const arr of candidates) {
    const geometry = arrange(full, tree, arr);
    const cost = layoutCost(full, wire(full, tree, geometry), unlinked.roots);
    if (!best || cost < best.cost) best = { geometry, cost, arr };
  }
  if (best) best = { ...best, ...refine(full, tree, best, unlinked.roots) };
  const geometry = best?.geometry ?? {
    boxes: new Map(),
    absolute: new Map(),
    ports: new Map(),
    taps: new Map(),
    routes: new Map(),
    labels: new Map(),
  };
  placeUnlinked(full, unlinked, geometry);
  return { tree, geometry };
}

/** 1 回の選び直しで試す案の数の上限（得の大きい順） */
const PROPOSALS = 6;
/** 選び直しを繰り返す回数の上限（案が通らなくなればそこで終わる） */
const REFINE_ROUNDS = 4;

/**
 * 選んだ候補の端子を、実際に線を引いた費用で選び直す（refine.ts）。側、位置の順に案を出し、得の大きい順に
 * 1 つずつ試して、全体の費用が下がる案だけを採る。採った案があれば、もう一度案を出し直す（ある端子を動かすと、
 * 別の端子が動ける空きができることがあるため。例: SPW1 を下へ移すと、LVDS1 が左へ移れる）。
 */
function refine(
  full: DiagramModel,
  tree: Tree,
  start: { geometry: DiagramLayout; cost: number; arr: Arrangement },
  unlinked: readonly Size[],
): { geometry: DiagramLayout; cost: number } {
  let best = { geometry: start.geometry, cost: start.cost };
  let choice: PortChoice = { sides: new Map(), offsets: new Map() };
  const tryChoice = (next: PortChoice) => {
    const geometry = arrange(full, tree, start.arr, next);
    const cost = layoutCost(full, wire(full, tree, geometry), unlinked);
    if (cost >= best.cost - 1e-6) return false;
    best = { geometry, cost };
    choice = next;
    return true;
  };
  const withAll = (base: PortChoice, ps: Proposal[]): PortChoice => ({
    sides: new Map([
      ...base.sides,
      ...ps.flatMap((p) => (p.side ? [[p.port, p.side] as const] : [])),
    ]),
    offsets: new Map([
      ...base.offsets,
      ...ps.flatMap((p) => (p.offset !== undefined ? [[p.port, p.offset] as const] : [])),
    ]),
  });
  for (let round = 0; round < REFINE_ROUNDS; round++) {
    let improved = false;
    for (const mode of ["sides", "offsets"] as const) {
      const proposals = refinePorts(full, tree, best.geometry, mode)
        .toSorted((a, b) => b.gain - a.gain)
        .slice(0, PROPOSALS);
      for (const p of proposals) improved = tryChoice(withAll(choice, [p])) || improved;
    }
    if (!improved) break;
  }
  return best;
}

/** 決めた配置の上に、表示する層の線（`model`）を引き、ラベルを置く */
export function drawLayout(plan: LayoutPlan, model: DiagramModel): DiagramLayout {
  const layout = wire(model, plan.tree, plan.geometry);
  layout.labels = labelPositions(model, layout, labelledEdges(model));
  return layout;
}

/** 配置を決めて線を引くまでを 1 度に行う（`full` を省くと `model` の線だけで位置を決める） */
export async function layoutModel(
  model: DiagramModel,
  full: DiagramModel = model,
): Promise<DiagramLayout> {
  return drawLayout(planLayout(full), model);
}
