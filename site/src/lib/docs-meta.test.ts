import { describe, expect, it } from "vitest";
import { deriveMeta, stripFrontmatter } from "./docs-meta.ts";

describe("deriveMeta", () => {
  it("最初の H1 を題名、次の段落を説明にする", () => {
    const md = "# Power `analyses`\n\nFirst *paragraph* with [a link](x.md).\n\n## Next\n\nSecond.";
    expect(deriveMeta(md)).toEqual({
      title: "Power analyses",
      description: "First paragraph with a link.",
    });
  });
  it("H1 の前にコメントがあってもよい。画像だけの段落は説明にしない", () => {
    const md = "<!-- x -->\n# T\n\n![a](b.svg)\n\nReal text.";
    expect(deriveMeta(md)).toEqual({ title: "T", description: "Real text." });
  });
  it("次の見出しまでに段落がなければ説明なし", () => {
    expect(deriveMeta("# T\n\n## S\n\ntext")).toEqual({ title: "T" });
  });
  it("コードブロックの中の # は題名にしない", () => {
    expect(deriveMeta("```\n# no\n```\n")).toEqual({});
  });
  it("長い説明は切る", () => {
    const d = deriveMeta(`# T\n\n${"あ".repeat(400)}`).description ?? "";
    expect(d.length).toBe(160);
    expect(d.endsWith("…")).toBe(true);
  });
  it("front matter を読み飛ばす", () => {
    expect(stripFrontmatter("---\ntitle: x\n---\n# T")).toBe("# T");
  });
});
