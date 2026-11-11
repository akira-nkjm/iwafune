import { useEffect, useRef } from "react";

/**
 * ⌘Z / Ctrl+Z で `undo`、⇧⌘Z / Ctrl+Shift+Z / Ctrl+Y で `redo` を呼ぶ。文字を入力している欄の中では
 * 何もしない（ブラウザの文字の取り消しを使う）。
 */
export function useUndoKeys(undo: () => void, redo: () => void) {
  const latest = useRef({ undo, redo });
  latest.current = { undo, redo };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key !== "z" && key !== "y") return;
      const el = e.target instanceof HTMLElement ? e.target : null;
      if (el && (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName)))
        return;
      e.preventDefault();
      if (key === "y" || e.shiftKey) latest.current.redo();
      else latest.current.undo();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
