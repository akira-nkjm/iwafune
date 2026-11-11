import type { ViewEdge, ViewPortSide } from "@/api/types";
import type { DiagramEdge } from "../model";
import type { Box, Pt } from "./geometry";

/**
 * 線を指すキー（view/electrical.toml の `[edge]` の表の名前、`fune view edge-set` の引数）。
 * `ネット:始点の端子>終点の端子`（例: `P5V:PCDU.OUT2>CAM.PWR`。まとめた線はネットを `+` でつなぐ）。
 * ネットと端子だけで決め、自動配置の順番や座標には依らないので、並べ直しても同じ線を指す。
 */
export function waypointKey(edge: DiagramEdge): string {
  return `${edge.nets.toSorted().join("+")}:${edge.ends}`;
}

/** 点の列を、水平と垂直の線分だけでつなぐ（斜めになる 2 点の間には角の点を足す。元の点はすべて残す） */
export function orthogonal(points: readonly Pt[]): Pt[] {
  const out: Pt[] = [];
  for (const p of points) {
    const prev = out.at(-1);
    if (prev && prev.x !== p.x && prev.y !== p.y) out.push({ x: p.x, y: prev.y });
    if (!prev || prev.x !== p.x || prev.y !== p.y) out.push({ ...p });
  }
  return out;
}

/** 区間を、その区間と直角の向きに動かす。端の点（端子）は動かさず、短い足を足してつなぐ */
export function moveSegment(points: readonly Pt[], index: number, at: Pt): Pt[] {
  const out = points.map((p) => ({ ...p }));
  const a = out[index];
  const b = out[index + 1];
  if (!a || !b) return out;
  const horizontal = a.y === b.y;
  const shift = (p: Pt) => (horizontal ? { x: p.x, y: at.y } : { x: at.x, y: p.y });
  out.splice(
    index,
    2,
    ...(index === 0 ? [a, shift(a)] : [shift(a)]),
    ...(index + 1 === points.length - 1 ? [shift(b), b] : [shift(b)]),
  );
  return orthogonal(out);
}

/** 区間の中ほどに、四角い迂回（折れ 4 つ）を足す（ダブルクリック）。斜めの線分は作らない */
export function addBend(points: readonly Pt[], i: number): Pt[] {
  const a = points[i];
  const b = points[i + 1];
  if (!a || !b) return [...points];
  const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
  const horizontal = a.y === b.y;
  const p = horizontal ? { x: mid.x - 10, y: a.y } : { x: a.x, y: mid.y - 10 };
  const q = horizontal ? { x: mid.x + 10, y: b.y } : { x: b.x, y: mid.y + 10 };
  const offset = (v: Pt) => (horizontal ? { x: v.x, y: v.y + 20 } : { x: v.x + 20, y: v.y });
  return [...points.slice(0, i + 1), p, offset(p), offset(q), q, ...points.slice(i + 1)];
}

/** これより小さい段差は座標の端数とみなして、ならす [px] */
const SNAP = 1.5;

/**
 * 保存する前に経路を整える。自動配置の座標の端数でできた小さな段差（SNAP 未満）をならし、整数に丸め、
 * 重なった点と一直線上の途中の点を除く（`[[468, 507], [468, 506], [468, 350]]` → `[[468, 507], [468, 350]]`）。
 */
export function tidyRoute(points: readonly Pt[]): Pt[] {
  const snapped: Pt[] = [];
  for (const p of orthogonal(points)) {
    const prev = snapped.at(-1);
    snapped.push({
      x: Math.round(prev && Math.abs(p.x - prev.x) < SNAP ? prev.x : p.x),
      y: Math.round(prev && Math.abs(p.y - prev.y) < SNAP ? prev.y : p.y),
    });
  }
  const unique = snapped.filter(
    (p, i) => i === 0 || p.x !== snapped[i - 1]?.x || p.y !== snapped[i - 1]?.y,
  );
  return unique.filter((p, i) => {
    const a = unique[i - 1];
    const b = unique[i + 1];
    return !a || !b || !((a.x === p.x && p.x === b.x) || (a.y === p.y && p.y === b.y));
  });
}

/** 手で引いた経路を保存する形（view/electrical.toml の `[edge]` の 1 行）にする。端子の側と位置は `ends` から写す */
export function storedRoute(points: readonly Pt[], ends?: ViewEdge): ViewEdge {
  return {
    points: tidyRoute(points).map((p) => [p.x, p.y]),
    source_side: ends?.source_side,
    target_side: ends?.target_side,
    source_offset: ends?.source_offset,
    target_offset: ends?.target_offset,
  };
}

/** 線の端の位置を丸める間隔 [px]（カードの縁の上で、この刻みで止まる） */
const ATTACH_GRID = 4;
/** 付け替えた端から、カードの外へまっすぐ出す足の長さ [px] */
const STUB = 16;
const OUTWARD: Record<ViewPortSide, Pt> = {
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  top: { x: 0, y: -1 },
  bottom: { x: 0, y: 1 },
};

/**
 * 線の端を付けるカードの縁の点。`p` にいちばん近い側を選び、その側の上の位置（左か上の角からの距離）を
 * ATTACH_GRID の刻みに丸めて、縁の中に収める。
 */
export function attachTo(card: Box, p: Pt): { side: ViewPortSide; offset: number; at: Pt } {
  const distance: [ViewPortSide, number][] = [
    ["left", Math.abs(p.x - card.x)],
    ["right", Math.abs(p.x - card.x - card.width)],
    ["top", Math.abs(p.y - card.y)],
    ["bottom", Math.abs(p.y - card.y - card.height)],
  ];
  const side = distance.reduce((a, b) => (b[1] < a[1] ? b : a))[0];
  const vertical = side === "left" || side === "right";
  const length = vertical ? card.height : card.width;
  const along = vertical ? p.y - card.y : p.x - card.x;
  const offset = Math.min(length, Math.max(0, Math.round(along / ATTACH_GRID) * ATTACH_GRID));
  const at = {
    left: { x: card.x, y: card.y + offset },
    right: { x: card.x + card.width, y: card.y + offset },
    top: { x: card.x + offset, y: card.y },
    bottom: { x: card.x + offset, y: card.y + card.height },
  }[side];
  return { side, offset, at };
}

/**
 * 線の端（始点か終点）を `at` に付け替えた経路。端から側の外へ短い足を出し、元の経路の 3 点目（端の次の
 * 折れの先）へ直角につなぐ。反対側の端と途中の経路はそのまま。続きがカードの向こう側（足の内側）にあるときは、
 * カード（`card`。分かるときだけ）の脇を回ってからつなぐ（カードを突き抜けないように）。
 */
export function reattach(
  points: readonly Pt[],
  end: "source" | "target",
  at: Pt,
  side: ViewPortSide,
  card?: Box,
): Pt[] {
  const path = end === "source" ? [...points] : [...points].reverse();
  const out = OUTWARD[side];
  const stub = { x: at.x + out.x * STUB, y: at.y + out.y * STUB };
  const rest = path.length > 2 ? path.slice(2) : path.slice(-1);
  const first = rest[0];
  const behind = first && (first.x - stub.x) * out.x + (first.y - stub.y) * out.y < 0;
  const detour: Pt[] = [];
  if (card && first && behind) {
    if (out.y !== 0) {
      const x = first.x < card.x + card.width / 2 ? card.x - STUB : card.x + card.width + STUB;
      detour.push({ x, y: stub.y }, { x, y: first.y });
    } else {
      const y = first.y < card.y + card.height / 2 ? card.y - STUB : card.y + card.height + STUB;
      detour.push({ x: stub.x, y }, { x: first.x, y });
    }
  }
  const next = orthogonal([at, stub, ...detour, ...rest]);
  return end === "source" ? next : next.reverse();
}

/** 折れ点と、それにつながる短い足を消す（ダブルクリック）。端の点は動かさない */
export function removeBend(points: readonly Pt[], index: number): Pt[] {
  const remove = new Set([index]);
  if (index + 1 < points.length - 1) remove.add(index + 1);
  else if (index - 1 > 0) remove.add(index - 1);
  return orthogonal(points.filter((_, i) => !remove.has(i)));
}

/** 端子がこれより動いていたら、手で引いた経路の端をつなぎ直す [px]（端数のずれは見逃す） */
const FOLLOW_TOLERANCE = 2;

/**
 * 手で引いた経路の両端を、いまの端子の位置に合わせる。保存したときから端子が動いていなければ（カードを動かして
 * いなければ）経路のまま。動いていれば、その端から側の外へ短い足を出し、元の経路の続きへ直角につなぎ直す
 * （reattach）。保存した経路は変えず、描くたびに合わせるので、カードを何度動かしても線がついて行く。
 */
export function followEnds(
  points: readonly Pt[],
  source: { at: Pt; side: ViewPortSide },
  target: { at: Pt; side: ViewPortSide },
): Pt[] {
  const moved = (p: Pt | undefined, q: Pt) =>
    !p || Math.abs(p.x - q.x) > FOLLOW_TOLERANCE || Math.abs(p.y - q.y) > FOLLOW_TOLERANCE;
  let out = [...points];
  if (moved(out[0], source.at)) out = reattach(out, "source", source.at, source.side);
  if (moved(out.at(-1), target.at)) out = reattach(out, "target", target.at, target.side);
  return out;
}
