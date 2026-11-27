import type { Root } from "mdast";
import remarkMath from "remark-math";
import remarkParse from "remark-parse";
import { unified } from "unified";
import { visit } from "unist-util-visit";
import { VFile } from "vfile";
import { describe, expect, it } from "vitest";
import remarkDocsImages from "./remark-docs-images.ts";
import remarkDocsLinks from "./remark-docs-links.ts";
import remarkDocsTitle from "./remark-docs-title.ts";
import remarkFences from "./remark-fences.ts";
import remarkGithubAlerts from "./remark-github-alerts.ts";
import remarkScreenshots from "./remark-screenshots.ts";

const DOCS = "/repo/docs";

/** Markdown を読んで、プラグインを当てた木を返す。 */
function run(md: string, file: string, plugin: (...a: never[]) => unknown, opts?: unknown): Root {
  const processor = unified().use(remarkParse).use(remarkMath);
  processor.use(plugin as never, opts as never);
  const vfile = new VFile({ path: `${DOCS}/${file}`, value: md });
  return processor.runSync(processor.parse(vfile), vfile) as Root;
}

describe("remark-docs-title", () => {
  it("最初の H1 だけを取り除く", () => {
    const tree = run("# T\n\ntext\n\n## S\n", "a.md", remarkDocsTitle);
    expect(tree.children.map((n) => n.type)).toEqual(["paragraph", "heading"]);
  });
});

describe("remark-docs-links", () => {
  it("リンクと定義を書き換える", () => {
    const md = "[a](../model.md#x) [b](../../crates/x.rs)\n\n[ref]: power.md\n";
    const tree = run(md, "guide/a.md", remarkDocsLinks, { docsDir: DOCS, base: "/iwafune" });
    const para = tree.children[0] as unknown as { children: { url?: string }[] };
    expect(para.children.map((c) => c.url).filter(Boolean)).toEqual([
      "/iwafune/model/#x",
      "https://github.com/akira-nkjm/iwafune/blob/main/crates/x.rs",
    ]);
    expect((tree.children[1] as unknown as { url: string }).url).toBe("/iwafune/guide/power/");
  });
  it("HTML の a の href も書き換える", () => {
    const tree = run('<a href="../model.md">m</a>\n', "guide/a.md", remarkDocsLinks, {
      docsDir: DOCS,
      base: "/iwafune",
    });
    const html: string[] = [];
    visit(tree, "html", (n) => {
      html.push(n.value);
    });
    expect(html.join("")).toContain('href="/iwafune/model/"');
  });
});

describe("remark-docs-images", () => {
  it("画像を配布先の URL にする", () => {
    const tree = run("![x](../assets/f.svg)\n", "guide/a.md", remarkDocsImages, {
      docsDir: DOCS,
      base: "/iwafune",
    });
    const img = (tree.children[0] as unknown as { children: { url: string }[] }).children[0];
    expect(img?.url).toBe("/iwafune/docs-files/assets/f.svg");
  });
});

describe("remark-fences", () => {
  it("math と mermaid を変換し、ほかの言語は残す", () => {
    const md = "```math\nx^2\n```\n\n```mermaid\ngraph TD\nA-->B\n```\n\n```rust\nfn a() {}\n```\n";
    const tree = run(md, "a.md", remarkFences);
    const [m, d, r] = tree.children as unknown as { type: string; data?: { hName: string } }[];
    expect(m?.type).toBe("math");
    expect(d?.data?.hName).toBe("pre");
    expect(r?.type).toBe("code");
  });
});

describe("remark-github-alerts", () => {
  it("[!NOTE] を aside にする", () => {
    const tree = run("> [!NOTE]\n> Be careful.\n", "a.md", remarkGithubAlerts);
    const q = tree.children[0] as unknown as {
      data: { hName: string; hProperties: { className: string[] } };
      children: unknown[];
    };
    expect(q.data.hName).toBe("aside");
    expect(q.data.hProperties.className).toContain("starlight-aside--note");
    expect(q.children).toHaveLength(2);
  });
  it("ふつうの引用はそのまま", () => {
    const tree = run("> plain\n", "a.md", remarkGithubAlerts);
    expect((tree.children[0] as unknown as { data?: unknown }).data).toBeUndefined();
  });
});

describe("remark-screenshots", () => {
  const md = "<!-- screenshot: /design/components — The components page -->\n";
  const opts = (exists: (f: string) => boolean) => ({
    docsDir: DOCS,
    shotsDir: "/site/shots",
    exists,
  });
  it("写真が無ければ説明の枠にする", () => {
    const tree = run(
      md,
      "guide/a.md",
      remarkScreenshots,
      opts(() => false),
    );
    const fig = tree.children[0] as unknown as {
      data: { hName: string; hProperties: { className: string[] } };
    };
    expect(fig.data.hName).toBe("figure");
    expect(fig.data.hProperties.className).toContain("shot--placeholder");
  });
  it("写真があれば画像にする（日本語のページは .ja を先に探す）", () => {
    const seen: string[] = [];
    const tree = run(
      md,
      "ja/guide/a.md",
      remarkScreenshots,
      opts((f) => {
        seen.push(f);
        return f.endsWith("design-components.ja.png");
      }),
    );
    const fig = tree.children[0] as unknown as { children: { url?: string; type: string }[] };
    expect(fig.children[0]?.type).toBe("image");
    expect(fig.children[0]?.url).toBe("../../../../site/shots/design-components.ja.png");
    expect(seen[0]).toBe("/site/shots/design-components.ja.png");
  });
  it("目印でないコメントは触らない", () => {
    const tree = run(
      "<!-- note -->\n",
      "a.md",
      remarkScreenshots,
      opts(() => true),
    );
    expect(tree.children[0]?.type).toBe("html");
  });
});

describe("rehypeTableWrap", () => {
  it("表を横に動かせる枠で包む", async () => {
    const { wrapTables } = await import("./rehype-table-wrap.ts");
    const table = { type: "element", tagName: "table", children: [] };
    const tree = {
      type: "root",
      children: [{ type: "element", tagName: "p", children: [] }, table],
    };
    wrapTables(tree);
    expect(tree.children[1]).toMatchObject({
      tagName: "div",
      properties: { className: ["table-wrap"] },
    });
    expect((tree.children[1] as { children: unknown[] }).children[0]).toBe(table);
  });
});
