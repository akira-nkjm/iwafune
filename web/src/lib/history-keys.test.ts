import { describe, expect, it } from "vitest";
import { historyKey, isTextEditing } from "./history-keys";

const base = { metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, target: null };
const key = (init: Partial<Parameters<typeof historyKey>[0]> & { key: string }) =>
  historyKey({ defaultPrevented: false, ...base, ...init });

const field = (tagName: string, extra: object = {}) =>
  ({ tagName, ...extra }) as unknown as EventTarget;

describe("取り消し・やり直しのキー", () => {
  it("⌘Z・Ctrl+Z は取り消し、⇧⌘Z・Ctrl+Shift+Z・Ctrl+Y はやり直し", () => {
    expect(key({ key: "z", metaKey: true })).toBe("undo");
    expect(key({ key: "z", ctrlKey: true })).toBe("undo");
    expect(key({ key: "Z", metaKey: true, shiftKey: true })).toBe("redo");
    expect(key({ key: "Z", ctrlKey: true, shiftKey: true })).toBe("redo");
    expect(key({ key: "y", ctrlKey: true })).toBe("redo");
  });

  it("修飾キーなし・Alt 付き・ほかのキーは何もしない", () => {
    expect(key({ key: "z" })).toBeNull();
    expect(key({ key: "z", ctrlKey: true, altKey: true })).toBeNull();
    expect(key({ key: "x", ctrlKey: true })).toBeNull();
    expect(key({ key: "y", ctrlKey: true, shiftKey: true })).toBeNull();
  });

  it("文字を打つ欄の中では欄自身の取り消しを使う（何もしない）", () => {
    for (const target of [
      field("INPUT", { type: "text" }),
      field("INPUT", { type: "number" }),
      field("INPUT"),
      field("TEXTAREA"),
      field("DIV", { isContentEditable: true }),
    ]) {
      expect(key({ key: "z", ctrlKey: true, target }), JSON.stringify(target)).toBeNull();
    }
  });

  it("文字を打たない部品（ボタン・チェック・スライダー・本文）の上では全体の取り消し", () => {
    for (const target of [
      field("BUTTON"),
      field("INPUT", { type: "checkbox" }),
      field("INPUT", { type: "range" }),
      field("DIV", { isContentEditable: false }),
      field("BODY"),
      null,
    ]) {
      expect(key({ key: "z", ctrlKey: true, target }), JSON.stringify(target)).toBe("undo");
    }
    expect(isTextEditing(null)).toBe(false);
  });

  it("ほかが処理済みのキー・変換中のキーは何もしない", () => {
    expect(key({ key: "z", ctrlKey: true, defaultPrevented: true })).toBeNull();
    expect(key({ key: "z", ctrlKey: true, isComposing: true })).toBeNull();
  });
});
