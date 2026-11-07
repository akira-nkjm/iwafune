/**
 * 小さな型付きの i18n: 日本語（既定）と英語の 2 つの辞書と、モジュールの状態。
 * - 選んだ言語は閲覧者ごとに localStorage に保存する。保存が無ければ navigator.language（en なら英語）から決める
 * - 部品は `useT()` で引く。React の外（トースト・表の見出しの組み立て）は `getT()`
 * - 言語を変えると root が画面を作り直す（root.tsx の Outlet の key）ので、getT() で引いた文字も新しくなる
 * - 数値と単位の表示文字列は API の display / units/format が返すもの（ここでは訳さない）
 */
import { useSyncExternalStore } from "react";
import { en } from "./en";
import { type Dict, ja } from "./ja";

export type { Dict } from "./ja";
export type Lang = "ja" | "en";

export const langs: readonly Lang[] = ["ja", "en"];
export const dictionaries: Record<Lang, Dict> = { ja, en };
/** 言語の名前はその言語で書く */
export const langNames: Record<Lang, string> = { ja: "日本語", en: "English" };
export const DEFAULT_LANG: Lang = "ja";
const KEY = "iwafune.lang";

const isLang = (v: unknown): v is Lang => v === "ja" || v === "en";

/** 保存した言語 → ブラウザの言語（英語なら en）→ 日本語 */
export function initialLang(stored: string | null, browser: string | null | undefined): Lang {
  if (isLang(stored)) return stored;
  if (browser?.toLowerCase().startsWith("en")) return "en";
  return DEFAULT_LANG;
}

function readStored(): string | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function browserLang(): string | null {
  try {
    return typeof navigator === "undefined" ? null : navigator.language;
  } catch {
    return null;
  }
}

let current: Lang =
  typeof window === "undefined" ? DEFAULT_LANG : initialLang(readStored(), browserLang());
const listeners = new Set<() => void>();

export function getLang(): Lang {
  return current;
}

export function getT(): Dict {
  return dictionaries[current];
}

export function setLang(lang: Lang): void {
  if (lang === current) return;
  current = lang;
  try {
    window.localStorage.setItem(KEY, lang);
  } catch {
    // 保存できなくても切り替えはできる
  }
  if (typeof document !== "undefined") document.documentElement.lang = lang;
  for (const l of listeners) l();
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

/** いまの言語。焼き込んだ index.html（とハイドレーション）は既定の日本語 */
export function useLang(): Lang {
  return useSyncExternalStore(subscribe, getLang, () => DEFAULT_LANG);
}

export function useT(): Dict {
  return dictionaries[useLang()];
}

/** モデルの名前（組み込みの語彙は日本語と英語の両方を持つ）を言語で選ぶ。無ければもう一方 */
export function localized(
  label: string | null | undefined,
  labelEn: string | null | undefined,
): string {
  if (current === "en") return labelEn || label || "";
  return label || labelEn || "";
}
