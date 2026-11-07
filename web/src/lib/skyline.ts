/**
 * 長方形を、決まった幅の中に隙間が少なくなるよう上から詰める（スカイライン法）。画面の配置だけで、単位は CSS px。
 * 置いた長方形の上端の輪郭（スカイライン）を持ち、次の長方形をいちばん高い所（y が小さい所。同じなら左）に置く。
 * 並べる順で詰まり方が変わるので、元の順・高い順・広い順・面積の順を試し、全体がいちばん低いものを採る
 * （同じ高さなら元の順に近いもの）。
 */

/** 詰める長方形の大きさ */
export interface RectSize {
  width: number;
  height: number;
}

/** 置いた位置（左上） */
export interface Spot {
  x: number;
  y: number;
}

/** 詰めた結果: 元の順の位置と、全体の幅と高さ */
export interface Packed {
  spots: Spot[];
  width: number;
  height: number;
}

/** 輪郭の 1 区間（x から幅 w の間の高さ y） */
interface Segment {
  x: number;
  w: number;
  y: number;
}

/** `sizes` を幅 `width` の中に、間隔 `gap` をあけて詰める。幅より広い長方形は左端に置く */
export function packRects(sizes: readonly RectSize[], width: number, gap: number): Packed {
  const index = sizes.map((_, i) => i);
  const area = (i: number) => (sizes[i]?.width ?? 0) * (sizes[i]?.height ?? 0);
  const orders = [
    index,
    index.toSorted((a, b) => (sizes[b]?.height ?? 0) - (sizes[a]?.height ?? 0)),
    index.toSorted((a, b) => (sizes[b]?.width ?? 0) - (sizes[a]?.width ?? 0)),
    index.toSorted((a, b) => area(b) - area(a)),
  ];
  let best: Packed | null = null;
  for (const order of orders) {
    const packed = packInOrder(sizes, order, width, gap);
    if (!best || packed.height < best.height) best = packed;
  }
  return best ?? { spots: [], width: 0, height: 0 };
}

/** 決めた順に 1 つずつ、いちばん高い所へ置く */
function packInOrder(
  sizes: readonly RectSize[],
  order: readonly number[],
  width: number,
  gap: number,
): Packed {
  // 各長方形の右と下に間隔を足して詰め、全体の幅にも 1 つ分の間隔を足す
  const total = Math.max(width, ...sizes.map((s) => s.width)) + gap;
  let sky: Segment[] = [{ x: 0, w: total, y: 0 }];
  const spots: Spot[] = sizes.map(() => ({ x: 0, y: 0 }));
  for (const i of order) {
    const size = sizes[i];
    if (!size) continue;
    const w = size.width + gap;
    const spot = lowestSpot(sky, w, total);
    spots[i] = spot;
    sky = raise(sky, spot.x, w, spot.y + size.height + gap);
  }
  const right = Math.max(0, ...sizes.map((s, i) => (spots[i]?.x ?? 0) + s.width));
  const bottom = Math.max(0, ...sizes.map((s, i) => (spots[i]?.y ?? 0) + s.height));
  return { spots, width: right, height: bottom };
}

/** 幅 `w` が入る、いちばん高い位置（同じ高さなら左）。候補は輪郭の各区間の左端 */
function lowestSpot(sky: readonly Segment[], w: number, total: number): Spot {
  let best: Spot | null = null;
  for (const seg of sky) {
    if (seg.x + w > total) break;
    const y = Math.max(...sky.filter((s) => s.x < seg.x + w && s.x + s.w > seg.x).map((s) => s.y));
    if (!best || y < best.y) best = { x: seg.x, y };
  }
  return best ?? { x: 0, y: Math.max(...sky.map((s) => s.y)) };
}

/** 輪郭の x から幅 w の間を高さ `top` にする（隣と同じ高さの区間はつなぐ） */
function raise(sky: readonly Segment[], x: number, w: number, top: number): Segment[] {
  const pieces: Segment[] = [{ x, w, y: top }];
  for (const s of sky) {
    const end = s.x + s.w;
    if (end <= x || s.x >= x + w) {
      pieces.push(s);
      continue;
    }
    if (s.x < x) pieces.push({ x: s.x, w: x - s.x, y: s.y });
    if (end > x + w) pieces.push({ x: x + w, w: end - (x + w), y: s.y });
  }
  pieces.sort((a, b) => a.x - b.x);
  const merged: Segment[] = [];
  for (const s of pieces) {
    const last = merged.at(-1);
    if (last && last.y === s.y && last.x + last.w === s.x) last.w += s.w;
    else merged.push({ ...s });
  }
  return merged;
}
