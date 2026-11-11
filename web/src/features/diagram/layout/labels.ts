import type { Box, Pt } from "./geometry";

/**
 * 線のラベル（プロトコル名）を、線の脇の空いた場所に置く。カード・枠の見出し・先に置いたラベル・どの線とも
 * 重ならない場所を、始点に近い区間から順に探す（線の上に重ねると線が読めず、カードや見出しの上では文字が読めない）。
 */
/** 線からラベルの中心までの距離の候補 [px]（近い順） */
const OFFSETS = [18, -18, 32, -32];
/** 区間のどこに置くかの候補（区間の長さに対する割合。中ほどを優先） */
const ALONG = [0.5, 0.25, 0.75];

export function placeLabels(
  routes: ReadonlyMap<string, Pt[]>,
  labels: ReadonlyMap<string, string>,
  cards: Box[],
): Map<string, Pt> {
  const occupied = [...cards];
  const out = new Map<string, Pt>();
  const segments = [...routes.values()].flatMap((pts) =>
    pts.slice(1).map((p, i) => ({ a: pts[i] as Pt, b: p })),
  );
  for (const [id, text] of labels) {
    const pts = routes.get(id) ?? [];
    const width = Math.max(28, text.length * 8 + 8);
    const height = 16;
    const overlaps = (r: Box, b: { x: number; y: number; width: number; height: number }) =>
      r.x < b.x + b.width && r.x + r.width > b.x && r.y < b.y + b.height && r.y + r.height > b.y;
    const crossesLine = (r: Box) =>
      segments.some(
        ({ a, b }) =>
          Math.min(a.x, b.x) < r.x + r.width &&
          Math.max(a.x, b.x) > r.x &&
          Math.min(a.y, b.y) < r.y + r.height &&
          Math.max(a.y, b.y) > r.y,
      );
    // 線の脇の候補（始点に近い区間から。区間の中ほど・1/4・3/4 の位置で、上下・左右に少しずつ離す）
    const candidates = pts.slice(1).flatMap((b, i) => {
      const a = pts[i] as Pt;
      return ALONG.flatMap((t) =>
        OFFSETS.map((offset) => ({
          x: a.x + (b.x - a.x) * t + (a.x === b.x ? width / 2 + offset : 0),
          y: a.y + (b.y - a.y) * t + (a.y === b.y ? offset : 0),
        })),
      );
    });
    // まずどの線とも重ならない場所、無ければ線とは重なってもよい場所（カード・見出し・ラベルとは重ねない）
    const rect = (c: Pt): Box => ({ x: c.x - width / 2, y: c.y - height / 2, width, height });
    const fits = (c: Pt, avoidLines: boolean) =>
      !occupied.some((b) => overlaps(rect(c), b)) && !(avoidLines && crossesLine(rect(c)));
    const at =
      candidates.find((c) => fits(c, true)) ??
      candidates.find((c) => fits(c, false)) ??
      candidates[0];
    if (!at) continue;
    occupied.push(rect(at));
    out.set(id, at);
  }
  return out;
}
