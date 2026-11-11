import { describe, expect, it } from "vitest";
import {
  isToggleShortcut,
  type KeyValueStorage,
  readClosedSections,
  readOpen,
  writeClosedSections,
  writeOpen,
} from "./sidebar-state";

function memory(): KeyValueStorage & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return { data, getItem: (k) => data.get(k) ?? null, setItem: (k, v) => void data.set(k, v) };
}

const broken: KeyValueStorage = {
  getItem: () => {
    throw new Error("SecurityError");
  },
  setItem: () => {
    throw new Error("QuotaExceededError");
  },
};

describe("サイドバーの開閉", () => {
  it("⌘B / Ctrl+B で切り替える（B だけ・Alt 付きでは切り替えない）", () => {
    expect(isToggleShortcut({ key: "b", metaKey: true, ctrlKey: false })).toBe(true);
    expect(isToggleShortcut({ key: "B", metaKey: false, ctrlKey: true })).toBe(true);
    expect(isToggleShortcut({ key: "b", metaKey: false, ctrlKey: false })).toBe(false);
    expect(isToggleShortcut({ key: "b", metaKey: true, ctrlKey: false, altKey: true })).toBe(false);
  });

  it("開閉は保存され、次に開いたときに戻る（初めては開いている）", () => {
    const s = memory();
    expect(readOpen(s)).toBe(true);
    writeOpen(false, s);
    expect(readOpen(s)).toBe(false);
    writeOpen(true, s);
    expect(readOpen(s)).toBe(true);
  });

  it("区分の開閉も保存される", () => {
    const s = memory();
    expect(readClosedSections(s)).toEqual([]);
    writeClosedSections(["mission"], s);
    expect(readClosedSections(s)).toEqual(["mission"]);
  });

  it("保存先が使えなくても例外を出さず、既定（開いている）で動く", () => {
    expect(readOpen(broken)).toBe(true);
    expect(() => writeOpen(false, broken)).not.toThrow();
    expect(readClosedSections(broken)).toEqual([]);
    expect(readOpen(null)).toBe(true);
  });
});
