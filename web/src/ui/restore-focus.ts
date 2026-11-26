import { useRef } from "react";

/**
 * ダイアログを開いたときにフォーカスのあった要素（開いたボタン）を覚え、閉じたらそこへフォーカスを戻す。
 * Radix はフォーカスを `DialogTrigger` にしか戻さない。ここのダイアログは多くが `DialogTrigger` を使わず、
 * ボタンの `onClick` で開いている間だけ描くので、戻し先が無くなる（Esc で閉じるとキーボード操作が先頭に戻る）。
 */
export function useRestoreFocus() {
  const opener = useRef<HTMLElement | null>(null);
  return {
    onOpenAutoFocus: () => {
      opener.current =
        document.activeElement instanceof HTMLElement ? document.activeElement : null;
    },
    onCloseAutoFocus: (event: Event) => {
      event.preventDefault();
      if (opener.current?.isConnected) opener.current.focus();
    },
  };
}
