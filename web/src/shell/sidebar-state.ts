/**
 * サイドバーの開閉と、ナビの区分の開閉の状態（閲覧者ごとに localStorage へ保存）。
 * 保存先が使えない環境（プライベートブラウズ・保存の禁止）でも動くよう、読み書きはすべて try/catch で包む。
 */
export const SIDEBAR_ID = "app-sidebar";
const OPEN_KEY = "iwafune.sidebar.open";
const SECTIONS_KEY = "iwafune.sidebar.closedSections";

export interface KeyValueStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function storage(): KeyValueStorage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

export function readOpen(s: KeyValueStorage | null = storage()): boolean {
  try {
    const v = s?.getItem(OPEN_KEY);
    return v === null || v === undefined ? true : v === "true";
  } catch {
    return true;
  }
}

export function writeOpen(open: boolean, s: KeyValueStorage | null = storage()): void {
  try {
    s?.setItem(OPEN_KEY, String(open));
  } catch {
    // 保存できなくても開閉はできる
  }
}

export function readClosedSections(s: KeyValueStorage | null = storage()): string[] {
  try {
    const v = s?.getItem(SECTIONS_KEY);
    const parsed: unknown = v ? JSON.parse(v) : [];
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

export function writeClosedSections(
  ids: readonly string[],
  s: KeyValueStorage | null = storage(),
): void {
  try {
    s?.setItem(SECTIONS_KEY, JSON.stringify(ids));
  } catch {
    // 保存できなくても開閉はできる
  }
}

/** ⌘B（Mac）/ Ctrl+B（それ以外）。エディタ類と同じ割り当て */
export function isToggleShortcut(e: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey?: boolean;
}): boolean {
  return (e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === "b";
}
