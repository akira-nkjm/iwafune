/**
 * 画面の配色（明るい / 暗い）の状態。i18n の言語と同じく、閲覧者ごとにブラウザの localStorage に保存する
 * （設計データにも API にも載せない）。
 * - 選びは `ThemePref`（OS に従う `system`・`light`・`dark`）。`system` は `prefers-color-scheme` に従い、OS の切り替えにも追従する
 * - 決まった配色 `Scheme` は <html data-theme="light|dark"> に付ける。CSS（index.css の暗い配色と `dark:` の variant）はこの属性に従う
 * - hex が要る描画（three.js・Recharts など）は `useScheme()` で配色を読み、lib/tone.ts の `…HexFor(scheme)` で色を引く
 * - 最初の描画で明るい画面がちらつかないよう、root.tsx が `themeBootScript()` を <head> に入れる（React より先に属性を付ける。色の値は持たない）
 * - 状態の本体は `createThemeStore`（環境を引数に取るのでテストできる）。モジュールの `themeStore` がブラウザの実体
 */
import { useSyncExternalStore } from "react";
import { type Scheme, toneHexFor, toneInkHexFor, uiHexFor } from "./tone";

export type ThemePref = "system" | "light" | "dark";
export const themePrefs: readonly ThemePref[] = ["system", "light", "dark"];
export const DEFAULT_THEME_PREF: ThemePref = "system";
export const THEME_STORAGE_KEY = "iwafune.theme";
/** OS が暗い配色を望むかを問うメディアクエリ */
export const DARK_QUERY = "(prefers-color-scheme: dark)";

/** 保存した値を選びに直す（知らない値と未保存は OS に従う） */
export function parseThemePref(v: unknown): ThemePref {
  return v === "light" || v === "dark" || v === "system" ? v : DEFAULT_THEME_PREF;
}

/** 選びと OS の設定から、使う配色を決める */
export function resolveScheme(pref: ThemePref, systemDark: boolean): Scheme {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}

/**
 * <head> に入れる小さな script の本文。React が動く前に <html data-theme> を付けて、読み込み中に明るい画面が一瞬出るのを防ぐ。
 * 判定は `parseThemePref`・`resolveScheme` と同じ（保存が無い・壊れている・読めないときは OS に従う）。
 * 値を埋め込んで組み立てず、固定の文字列で持つ（script の本文に値を差し込むと、差し込む値の無害化が要る）。
 * 中の保存のキーとメディアクエリが `THEME_STORAGE_KEY`・`DARK_QUERY` と同じことは theme.test.ts が確かめる
 */
const BOOT_SCRIPT =
  '(function(){try{var p=null;try{p=localStorage.getItem("iwafune.theme")}catch(e){}var d=p==="dark"||(p!=="light"&&window.matchMedia("(prefers-color-scheme: dark)").matches);document.documentElement.setAttribute("data-theme",d?"dark":"light")}catch(e){}})()';

export function themeBootScript(): string {
  return BOOT_SCRIPT;
}

/** 配色の状態が頼る環境（ブラウザでは localStorage・matchMedia・<html>。無ければ null） */
export interface ThemeEnv {
  storage: Pick<Storage, "getItem" | "setItem"> | null;
  media: Pick<MediaQueryList, "matches" | "addEventListener" | "removeEventListener"> | null;
  root: Pick<Element, "setAttribute"> | null;
}

export interface ThemeStore {
  getPref(): ThemePref;
  getScheme(): Scheme;
  setPref(pref: ThemePref): void;
  subscribe(cb: () => void): () => void;
}

/** 配色の状態を作る。作った時点で保存した選びを読み、決まった配色を root に付ける */
export function createThemeStore(env: ThemeEnv): ThemeStore {
  const read = (): ThemePref => {
    try {
      return parseThemePref(env.storage?.getItem(THEME_STORAGE_KEY));
    } catch {
      return DEFAULT_THEME_PREF;
    }
  };
  let pref = read();
  let scheme = resolveScheme(pref, env.media?.matches ?? false);
  const listeners = new Set<() => void>();
  env.root?.setAttribute("data-theme", scheme);

  const notify = () => {
    for (const l of listeners) l();
  };
  /** 配色を決め直し、変わったら属性を替える（変わったかを返す） */
  const refresh = (): boolean => {
    const next = resolveScheme(pref, env.media?.matches ?? false);
    if (next === scheme) return false;
    scheme = next;
    env.root?.setAttribute("data-theme", scheme);
    return true;
  };
  // OS の設定が変わったら（`system` のときだけ結果が変わる）追従する
  env.media?.addEventListener("change", () => {
    if (refresh()) notify();
  });

  return {
    getPref: () => pref,
    getScheme: () => scheme,
    setPref(next) {
      if (next === pref) return;
      pref = next;
      try {
        env.storage?.setItem(THEME_STORAGE_KEY, next);
      } catch {
        // 保存できなくても切り替えはできる
      }
      refresh();
      // 選びだけ変わって配色が同じ（OS に従う → OS と同じ配色）ときも、選びを読む画面を更新する
      notify();
    },
    subscribe(cb) {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
  };
}

function browserEnv(): ThemeEnv {
  if (typeof window === "undefined") return { storage: null, media: null, root: null };
  let storage: ThemeEnv["storage"] = null;
  try {
    storage = window.localStorage;
  } catch {
    // 無効にしたブラウザでは読めない
  }
  const media = typeof window.matchMedia === "function" ? window.matchMedia(DARK_QUERY) : null;
  return { storage, media, root: document.documentElement };
}

/** ブラウザの実体 */
export const themeStore: ThemeStore = createThemeStore(browserEnv());

export function setThemePref(pref: ThemePref): void {
  themeStore.setPref(pref);
}

/** 閲覧者の選び。焼き込んだ HTML（とハイドレーション）は OS に従う */
export function useThemePref(): ThemePref {
  return useSyncExternalStore(themeStore.subscribe, themeStore.getPref, () => DEFAULT_THEME_PREF);
}

/** 決まった配色。焼き込んだ HTML（とハイドレーション）は明るい配色 */
export function useScheme(): Scheme {
  return useSyncExternalStore(themeStore.subscribe, themeStore.getScheme, () => "light");
}

/** hex が要る描画が配色に応じた色を引く（配色が変わると再描画される） */
export function useHex() {
  const scheme = useScheme();
  return { scheme, tone: toneHexFor(scheme), toneInk: toneInkHexFor(scheme), ui: uiHexFor(scheme) };
}
