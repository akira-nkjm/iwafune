/** 分割した欄の幅の範囲と保存先（CSS px）。物理量ではなく画面の配置だけ。 */
export interface PaneWidth {
  readonly key: string;
  readonly min: number;
  readonly max: number;
  readonly initial: number;
}

/** ツリーの一覧の列幅（ツリーの表示。詳細は残りの幅） */
export const TREE_WIDTH: PaneWidth = {
  key: "iwafune.tree.width",
  min: 320,
  max: 380,
  initial: 350,
};
/** 詳細の欄の幅（一覧の表示。カードの並びが残りの幅を使う） */
export const DETAIL_WIDTH: PaneWidth = {
  key: "iwafune.detail.width",
  min: 440,
  max: 760,
  initial: 560,
};
type WidthStorage = Pick<Storage, "getItem" | "setItem">;
type GetStorage = () => WidthStorage;
const browserStorage: GetStorage = () => window.localStorage;

/** 保存値とドラッグ位置を同じ範囲に収める。 */
export function clampTreeWidth(width: number, spec: PaneWidth = TREE_WIDTH): number {
  return Number.isFinite(width) ? Math.max(spec.min, Math.min(spec.max, width)) : spec.initial;
}

/** SSR・ストレージ拒否・不正な保存値でも既定幅で表示できる。 */
export function readTreeWidth(
  getStorage: GetStorage = browserStorage,
  spec: PaneWidth = TREE_WIDTH,
): number {
  try {
    const value = getStorage().getItem(spec.key);
    return value?.trim() ? clampTreeWidth(Number(value), spec) : spec.initial;
  } catch {
    return spec.initial;
  }
}

/** 保存できなくても幅の変更は使える。 */
export function writeTreeWidth(
  width: number,
  getStorage: GetStorage = browserStorage,
  spec: PaneWidth = TREE_WIDTH,
): void {
  try {
    getStorage().setItem(spec.key, String(clampTreeWidth(width, spec)));
  } catch {
    // ストレージを無効にした環境では、この画面を開いている間だけ保持する。
  }
}
