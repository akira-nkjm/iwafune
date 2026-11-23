import { describe, expect, it } from "vitest";
import {
  createThemeStore,
  DARK_QUERY,
  parseThemePref,
  resolveScheme,
  THEME_STORAGE_KEY,
  type ThemeEnv,
  themeBootScript,
} from "@/lib/theme";

/** テスト用の環境（localStorage・matchMedia・<html> の代わり） */
function fakeEnv(opts: { stored?: string; systemDark?: boolean; storageThrows?: boolean } = {}) {
  const data = new Map<string, string>();
  if (opts.stored !== undefined) data.set(THEME_STORAGE_KEY, opts.stored);
  const attrs = new Map<string, string>();
  const handlers = new Set<() => void>();
  const media = {
    matches: opts.systemDark ?? false,
    addEventListener: (_: string, h: () => void) => void handlers.add(h),
    removeEventListener: (_: string, h: () => void) => void handlers.delete(h),
  };
  const env: ThemeEnv = {
    storage: {
      getItem: (k) => {
        if (opts.storageThrows) throw new Error("denied");
        return data.get(k) ?? null;
      },
      setItem: (k, v) => {
        if (opts.storageThrows) throw new Error("denied");
        data.set(k, v);
      },
    },
    media: media as unknown as ThemeEnv["media"],
    root: { setAttribute: (k, v) => void attrs.set(k, v) },
  };
  const osChanges = (dark: boolean) => {
    media.matches = dark;
    for (const h of handlers) h();
  };
  return { env, data, attrs, osChanges };
}

describe("配色の選び", () => {
  it("保存の値を読む。知らない値と未保存は OS に従う", () => {
    expect(parseThemePref("dark")).toBe("dark");
    expect(parseThemePref("light")).toBe("light");
    expect(parseThemePref("system")).toBe("system");
    expect(parseThemePref(null)).toBe("system");
    expect(parseThemePref("blue")).toBe("system");
  });

  it("system は OS の設定、それ以外は選んだ配色", () => {
    expect(resolveScheme("system", true)).toBe("dark");
    expect(resolveScheme("system", false)).toBe("light");
    expect(resolveScheme("light", true)).toBe("light");
    expect(resolveScheme("dark", false)).toBe("dark");
  });
});

describe("配色の状態", () => {
  it("作った時点で保存した選びを読み、<html data-theme> に付ける", () => {
    const f = fakeEnv({ stored: "dark" });
    const store = createThemeStore(f.env);
    expect(store.getPref()).toBe("dark");
    expect(store.getScheme()).toBe("dark");
    expect(f.attrs.get("data-theme")).toBe("dark");
  });

  it("保存が無ければ OS に従い、OS が暗ければ暗い配色", () => {
    const f = fakeEnv({ systemDark: true });
    const store = createThemeStore(f.env);
    expect(store.getPref()).toBe("system");
    expect(f.attrs.get("data-theme")).toBe("dark");
  });

  it("選びを変えると保存し、属性を替え、購読者に知らせる", () => {
    const f = fakeEnv();
    const store = createThemeStore(f.env);
    let n = 0;
    store.subscribe(() => n++);
    store.setPref("dark");
    expect(f.data.get(THEME_STORAGE_KEY)).toBe("dark");
    expect(f.attrs.get("data-theme")).toBe("dark");
    expect(store.getScheme()).toBe("dark");
    expect(n).toBe(1);
    store.setPref("dark");
    expect(n).toBe(1);
    store.setPref("light");
    expect(f.attrs.get("data-theme")).toBe("light");
    expect(n).toBe(2);
  });

  it("system のとき OS の切り替えに追従し、明示した選びは OS に従わない", () => {
    const f = fakeEnv();
    const store = createThemeStore(f.env);
    let n = 0;
    store.subscribe(() => n++);
    f.osChanges(true);
    expect(store.getScheme()).toBe("dark");
    expect(f.attrs.get("data-theme")).toBe("dark");
    expect(n).toBe(1);
    store.setPref("light");
    f.osChanges(false);
    f.osChanges(true);
    expect(store.getScheme()).toBe("light");
    expect(f.attrs.get("data-theme")).toBe("light");
  });

  it("購読をやめると知らせない", () => {
    const f = fakeEnv();
    const store = createThemeStore(f.env);
    let n = 0;
    const off = store.subscribe(() => n++);
    off();
    store.setPref("dark");
    expect(n).toBe(0);
  });

  it("保存できない・読めない環境でも切り替えられる", () => {
    const f = fakeEnv({ storageThrows: true, systemDark: true });
    const store = createThemeStore(f.env);
    expect(store.getPref()).toBe("system");
    store.setPref("light");
    expect(store.getScheme()).toBe("light");
    expect(f.attrs.get("data-theme")).toBe("light");
  });

  it("環境が無い（サーバー側）でも作れて、明るい配色", () => {
    const store = createThemeStore({ storage: null, media: null, root: null });
    expect(store.getScheme()).toBe("light");
    store.setPref("dark");
    expect(store.getScheme()).toBe("dark");
  });
});

describe("最初の描画の前に属性を付ける script", () => {
  /** script を、与えた localStorage・matchMedia・document の上で走らせ、付いた属性を返す */
  function run(stored: string | null, systemDark: boolean, storageThrows = false) {
    const attrs = new Map<string, string>();
    const g = {
      localStorage: {
        getItem: () => {
          if (storageThrows) throw new Error("denied");
          return stored;
        },
      },
      matchMedia: () => ({ matches: systemDark }),
      document: { documentElement: { setAttribute: (k: string, v: string) => attrs.set(k, v) } },
    };
    new Function("localStorage", "window", "document", themeBootScript())(
      g.localStorage,
      g,
      g.document,
    );
    return attrs.get("data-theme");
  }

  it("ストアと同じ結果を返す（選び × OS の全組み合わせ）", () => {
    for (const stored of [null, "system", "light", "dark", "junk"]) {
      for (const systemDark of [false, true]) {
        const f = fakeEnv({ systemDark, ...(stored === null ? {} : { stored }) });
        createThemeStore(f.env);
        expect(run(stored, systemDark), `${stored} dark=${systemDark}`).toBe(
          f.attrs.get("data-theme"),
        );
      }
    }
  });

  it("保存が読めなくても OS に従う", () => {
    expect(run(null, true, true)).toBe("dark");
    expect(run(null, false, true)).toBe("light");
  });

  it("保存のキーとメディアクエリは、状態の本体と同じものを使う（script は固定の文字列で持つ）", () => {
    expect(themeBootScript()).toContain(
      `localStorage.getItem(${JSON.stringify(THEME_STORAGE_KEY)})`,
    );
    expect(themeBootScript()).toContain(`matchMedia(${JSON.stringify(DARK_QUERY)})`);
  });

  it("色の値を持たない", () => {
    expect(themeBootScript()).not.toMatch(/#[0-9a-f]{3,6}|rgb|white|black/i);
  });
});
