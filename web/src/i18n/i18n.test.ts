import { describe, expect, it } from "vitest";
import { DEFAULT_LANG, dictionaries, initialLang, langs } from "./index";

type Shape = Record<string, unknown>;

/** "a.b.c" → "string" | "function/2" の一覧 */
function shape(o: Shape, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [k, v] of Object.entries(o)) {
    const path = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "function") out.set(path, `function/${v.length}`);
    else if (v && typeof v === "object") for (const e of shape(v as Shape, path)) out.set(...e);
    else out.set(path, typeof v);
  }
  return out;
}

function flatStrings(o: Shape, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [k, v] of Object.entries(o)) {
    const path = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out.set(path, v);
    else if (typeof v === "function") {
      const r = (v as (...a: unknown[]) => unknown)(...Array.from({ length: v.length }, () => 1));
      out.set(path, String(r));
    } else if (v && typeof v === "object")
      for (const e of flatStrings(v as Shape, path)) out.set(...e);
  }
  return out;
}

describe("辞書", () => {
  const ja = shape(dictionaries.ja);

  it("既定は日本語。保存した言語 → ブラウザの言語（英語なら en）の順に決める", () => {
    expect(DEFAULT_LANG).toBe("ja");
    expect(initialLang(null, "ja-JP")).toBe("ja");
    expect(initialLang(null, "en-US")).toBe("en");
    expect(initialLang(null, "fr-FR")).toBe("ja");
    expect(initialLang("ja", "en-US")).toBe("ja");
    expect(initialLang("en", "ja-JP")).toBe("en");
    expect(initialLang("xx", null)).toBe("ja");
  });

  for (const lang of langs) {
    it(`${lang}: 日本語の辞書と同じキー・同じ形`, () => {
      const other = shape(dictionaries[lang]);
      expect([...other.keys()].sort()).toEqual([...ja.keys()].sort());
      for (const [k, kind] of ja) expect(other.get(k), k).toBe(kind);
    });

    it(`${lang}: 空の文言が無い`, () => {
      for (const [k, v] of flatStrings(dictionaries[lang])) expect(v.trim(), k).not.toBe("");
    });
  }

  it("英語の辞書に日本語が混ざっていない（言語の切り替えの「日本語」だけは例外）", () => {
    const jp = /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/u;
    const offenders = [...flatStrings(dictionaries.en)].filter(([, v]) => jp.test(v));
    expect(offenders).toEqual([]);
  });
});
