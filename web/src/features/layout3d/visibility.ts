/**
 * 配置のページの表示・非表示（ページの中だけの状態。設計データには書かない）。
 * 部品は ID の集合で隠し、座標の表示（原点の軸・地面の格子・画面隅の軸）は 1 つずつ切り替える。
 */

/** 部品の組（フォルダーの子孫など）の表示の状態。一部だけ隠れていれば `mixed` */
export type Visibility = "shown" | "hidden" | "mixed";

/** 隠した ID の集合に対する、部品の組の表示の状態（組が空なら `shown`） */
export function visibilityOf(hidden: ReadonlySet<string>, ids: readonly string[]): Visibility {
  const n = ids.filter((id) => hidden.has(id)).length;
  return n === 0 ? "shown" : n === ids.length ? "hidden" : "mixed";
}

/** 目のアイコンを押したときの次の集合: すべて隠れていれば出し、1 つでも見えていればまとめて隠す */
export function toggleHidden(hidden: ReadonlySet<string>, ids: readonly string[]): Set<string> {
  const next = new Set(hidden);
  const show = visibilityOf(hidden, ids) === "hidden";
  for (const id of ids) {
    if (show) next.delete(id);
    else next.add(id);
  }
  return next;
}

/** 補助の表示の種類（視野の円錐・原点の軸・地面の格子・画面隅の軸） */
export const SCENE_HELPERS = ["fov", "origin", "grid", "gizmo"] as const;
export type SceneHelper = (typeof SCENE_HELPERS)[number];
export type SceneHelpers = Record<SceneHelper, boolean>;

/** 既定では視野と画面隅の軸だけ出す（原点の軸と地面の格子は、必要なときに出す） */
export const DEFAULT_SCENE_HELPERS: SceneHelpers = {
  fov: true,
  origin: false,
  grid: false,
  gizmo: true,
};

/** 3D の視点を部品に寄せる依頼。同じ部品を続けて押しても寄せ直すよう、押した回数 `n` を持つ */
export interface FocusRequest {
  id: string;
  n: number;
}

/** 組（`ids`）だけが見えていて、ほかの描ける部品（`placed` の残り）がすべて隠れているか */
export function isIsolated(
  hidden: ReadonlySet<string>,
  placed: readonly string[],
  ids: readonly string[],
): boolean {
  const keep = new Set(ids);
  const others = placed.filter((id) => !keep.has(id));
  return (
    others.length > 0 && others.every((id) => hidden.has(id)) && ids.every((id) => !hidden.has(id))
  );
}

/** 「この部品だけ表示」の次の隠す集合: 組だけが見えていればすべて表示に戻し、そうでなければ組の外の描ける部品を隠す */
export function isolate(
  hidden: ReadonlySet<string>,
  placed: readonly string[],
  ids: readonly string[],
): Set<string> {
  if (isIsolated(hidden, placed, ids)) return new Set();
  const keep = new Set(ids);
  return new Set(placed.filter((id) => !keep.has(id)));
}
