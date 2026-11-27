import { describe, expect, it } from "vitest";
import { rewriteImage, rewriteLink, routeOfDoc } from "./routes.ts";

const ctx = (fileRel: string) => ({ fileRel, base: "/iwafune" });
const GH = "https://github.com/akira-nkjm/iwafune/blob/main";

describe("routeOfDoc", () => {
  it("ファイルをルートにする", () => {
    expect(routeOfDoc("guide/install.md")).toBe("/guide/install/");
    expect(routeOfDoc("guide/index.md")).toBe("/guide/");
    expect(routeOfDoc("ja/analyses/power.md")).toBe("/ja/analyses/power/");
    expect(routeOfDoc("architecture/registry.md")).toBe("/architecture/registry/");
    expect(routeOfDoc("index.md")).toBe("/");
  });
});

describe("rewriteLink", () => {
  it("docs の .md をルートにして、見出しを保つ", () => {
    expect(rewriteLink("../model.md", ctx("docs/guide/concepts.md"))).toBe("/iwafune/model/");
    expect(rewriteLink("power.md#equations", ctx("docs/analyses/index.md"))).toBe(
      "/iwafune/analyses/power/#equations",
    );
    expect(rewriteLink("./index.md", ctx("docs/ja/guide/install.md"))).toBe("/iwafune/ja/guide/");
    expect(rewriteLink("../../model.md", ctx("docs/ja/guide/install.md"))).toBe("/iwafune/model/");
  });
  it("docs の外のファイルは GitHub の main へ", () => {
    expect(
      rewriteLink("../../crates/lib/sat-astro/src/rocket.rs", ctx("docs/analyses/power.md")),
    ).toBe(`${GH}/crates/lib/sat-astro/src/rocket.rs`);
    expect(rewriteLink("../CONTRIBUTING.md#rules", ctx("docs/development.md"))).toBe(
      `${GH}/CONTRIBUTING.md#rules`,
    );
    expect(rewriteLink("../fixtures/demo-6u/", ctx("docs/development.md"))).toBe(
      "https://github.com/akira-nkjm/iwafune/tree/main/fixtures/demo-6u",
    );
  });
  it("docs の中の .md 以外は GitHub へ", () => {
    expect(rewriteLink("diagrams/x.drawio", ctx("docs/architecture.md"))).toBe(
      `${GH}/docs/diagrams/x.drawio`,
    );
  });
  it("/ で始まる相対はリポジトリのルートから", () => {
    expect(rewriteLink("/docs/model.md", ctx("docs/guide/x.md"))).toBe("/iwafune/model/");
    expect(rewriteLink("/crates/a.rs", ctx("docs/guide/x.md"))).toBe(`${GH}/crates/a.rs`);
  });
  it("外部 URL・見出し・メール・リポジトリの外はそのまま", () => {
    for (const u of ["https://example.org/a", "#top", "mailto:a@b.c", "//cdn/x", "../../../x.md"]) {
      expect(rewriteLink(u, ctx("docs/guide/x.md"))).toBe(u);
    }
  });
});

describe("rewriteImage", () => {
  it("docs の画像を配布先にする", () => {
    expect(rewriteImage("../assets/figures/a.svg", ctx("docs/analyses/power.md"))).toBe(
      "/iwafune/docs-files/assets/figures/a.svg",
    );
    expect(rewriteImage("../../assets/figures/a.svg", ctx("docs/ja/analyses/power.md"))).toBe(
      "/iwafune/docs-files/assets/figures/a.svg",
    );
  });
  it("外部と docs の外はそのまま", () => {
    expect(rewriteImage("https://x/y.png", ctx("docs/a.md"))).toBe("https://x/y.png");
    expect(rewriteImage("../web/public/favicon.svg", ctx("docs/a.md"))).toBe(
      "../web/public/favicon.svg",
    );
  });
});
