import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("ui/dialog", () => {
  it("閉じるボタンの読み上げ名は辞書から引く（日本語を直に書かない）", () => {
    const src = readFileSync(new URL("./dialog.tsx", import.meta.url), "utf8");
    expect(src).not.toMatch(/[぀-ヿ一-鿿]/);
    expect(src).toContain("common.close");
  });
});
