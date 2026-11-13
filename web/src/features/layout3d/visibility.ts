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

/** 座標の表示の種類（原点の軸・地面の格子・画面隅の軸） */
export const SCENE_HELPERS = ["origin", "grid", "gizmo"] as const;
export type SceneHelper = (typeof SCENE_HELPERS)[number];
export type SceneHelpers = Record<SceneHelper, boolean>;

/** 既定ではすべて出す */
export const DEFAULT_SCENE_HELPERS: SceneHelpers = { origin: true, grid: true, gizmo: true };

/** 3D の視点を部品に寄せる依頼。同じ部品を続けて押しても寄せ直すよう、押した回数 `n` を持つ */
export interface FocusRequest {
  id: string;
  n: number;
}
