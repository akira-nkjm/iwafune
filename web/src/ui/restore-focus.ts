import { useRef } from "react";

/**
 * 開いた要素が `[role="menu"]` の項目なら、そのメニューを開いたボタン（Radix は `aria-labelledby` にトリガーの ID を持つ）。
 * 項目は選ぶとメニューごと消えるので、閉じるときの戻し先にはトリガーを使う。
 */
function menuTriggerOf(el: HTMLElement): HTMLElement | null {
  const menu = el.closest('[role="menu"]');
  const id = menu?.getAttribute("aria-labelledby");
  return id ? document.getElementById(id) : null;
}

/**
 * ダイアログを開いたときにフォーカスのあった要素（開いたボタン）を覚え、閉じたらそこへフォーカスを戻す。
 * Radix はフォーカスを `DialogTrigger` にしか戻さない。ここのダイアログは多くが `DialogTrigger` を使わず、
 * ボタンの `onClick` で開いている間だけ描くので、戻し先が無くなる（Esc で閉じるとキーボード操作が先頭に戻る）。
 * ドロップダウンメニューの項目から開いたときは項目が消えているので、メニューを開いたボタンへ戻す。
 */
export function useRestoreFocus() {
  const targets = useRef<HTMLElement[]>([]);
  return {
    onOpenAutoFocus: () => {
      const active = document.activeElement;
      if (!(active instanceof HTMLElement) || active === document.body) {
        targets.current = [];
        return;
      }
      const trigger = menuTriggerOf(active);
      targets.current = trigger ? [active, trigger] : [active];
    },
    onCloseAutoFocus: (event: Event) => {
      event.preventDefault();
      targets.current.find((el) => el.isConnected)?.focus();
    },
  };
}
