import { describe, expect, it } from "vitest";
import { consoleCommands, tokenize } from "./commands.ts";

describe("tokenize", () => {
  it("引用符で 1 語にまとめる", () => {
    expect(tokenize('fune set CAM.mass "1.2 kg" --note "Heavier lens barrel"')).toEqual([
      "fune",
      "set",
      "CAM.mass",
      "1.2 kg",
      "--note",
      "Heavier lens barrel",
    ]);
    expect(tokenize("a '' b")).toEqual(["a", "", "b"]);
  });
  it("閉じていない引用符は誤り", () => {
    expect(() => tokenize('a "b')).toThrow();
  });
});

describe("consoleCommands", () => {
  const md = [
    "```console",
    "$ fune init x",
    "✔ Created",
    "$ fune requirement add R \\",
    "    --kind at_most",
    "ok",
    "```",
    "text",
    "```console",
    "$ cd x",
    "```",
    "<!-- stop -->",
    "```console",
    "$ never",
    "```",
  ].join("\n");
  it("出力の行を除き、行末の \\ でつなぐ", () => {
    expect(consoleCommands(md, "<!-- stop -->")).toEqual([
      "fune init x",
      "fune requirement add R --kind at_most",
      "cd x",
    ]);
  });
  it("止める目印が無ければ誤り", () => {
    expect(() => consoleCommands(md, "<!-- none -->")).toThrow();
  });
});
