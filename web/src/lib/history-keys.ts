/** 取り消し・やり直しのキー。入力欄の中では入力欄自身の取り消しを使う */
export type HistoryKey = "undo" | "redo";

/** 文字を打つ欄ではない `<input>` の種類（ボタン・チェック・スライダーなど。ここでは全体の取り消しを使う） */
const NON_TEXT_INPUTS = new Set([
  "button",
  "checkbox",
  "color",
  "file",
  "image",
  "radio",
  "range",
  "reset",
  "submit",
]);

/** 文字を打つ欄（入力欄・テキストエリア・contenteditable）か。そこでは欄自身の取り消しが先 */
export function isTextEditing(target: EventTarget | null): boolean {
  // DOM の型は使わず、必要な属性だけを見る（node のテストでも同じ判定になる）
  const el = target as { tagName?: unknown; isContentEditable?: unknown; type?: unknown } | null;
  if (!el || typeof el.tagName !== "string") return false;
  if (el.isContentEditable === true) return true;
  if (el.tagName === "TEXTAREA") return true;
  return el.tagName === "INPUT" && !NON_TEXT_INPUTS.has(String(el.type ?? "text"));
}

type KeyInfo = Pick<
  KeyboardEvent,
  "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey" | "target" | "defaultPrevented"
> & { isComposing?: boolean };

/**
 * キーが取り消し・やり直しか。⌘Z / Ctrl+Z は取り消し、⇧⌘Z / Ctrl+Shift+Z / Ctrl+Y はやり直し。
 * 文字を打つ欄の中・変換中・ほかが処理済みのキーは null（欄の取り消しやほかの処理を邪魔しない）。
 */
export function historyKey(e: KeyInfo): HistoryKey | null {
  if (e.defaultPrevented || e.isComposing) return null;
  if (!(e.metaKey || e.ctrlKey) || e.altKey) return null;
  if (isTextEditing(e.target)) return null;
  const key = e.key.toLowerCase();
  if (key === "z") return e.shiftKey ? "redo" : "undo";
  if (key === "y" && !e.shiftKey) return "redo";
  return null;
}
