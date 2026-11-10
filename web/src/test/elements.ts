import { isValidElement, type ReactElement, type ReactNode } from "react";

/** テストで要素の props を読む・ハンドラーを呼ぶための形 */
export type AnyElement = ReactElement<Record<string, unknown>>;

/**
 * 部品が返した要素の木（描画はしない。関数の部品は展開しない）から、props が条件に合う最初の要素を探す。
 * DOM の無いテスト（vitest の node 環境）で、欄やボタンのハンドラーを直接呼んで振る舞いを確かめるのに使う
 */
export function findElement(
  node: ReactNode,
  match: (props: Record<string, unknown>) => boolean,
): AnyElement | null {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findElement(child, match);
      if (found) return found;
    }
    return null;
  }
  if (!isValidElement<Record<string, unknown>>(node)) return null;
  if (match(node.props)) return node;
  return findElement(node.props.children as ReactNode, match);
}

/** `aria-label` で要素を探す（見つからなければテストを失敗させる） */
export function byLabel(node: ReactNode, label: string): AnyElement {
  const found = findElement(node, (props) => props["aria-label"] === label);
  if (!found) throw new Error(`aria-label="${label}" の要素がありません`);
  return found;
}

/** 要素の props のハンドラーを、テスト用の偽のイベントで呼ぶ */
export function fire(element: AnyElement, handler: string, event: object = {}): void {
  const fn = element.props[handler];
  if (typeof fn !== "function") throw new Error(`${handler} がありません`);
  fn(event);
}
