import { describe, expect, it } from "vitest";
import { extractCssRefs, extractRefs, failedFormulas } from "./check-dist.ts";

describe("extractRefs", () => {
  it("リンクと資源を分けて集める", () => {
    const html = `<link rel="stylesheet" href="/iwafune/a.css"><link rel="canonical" href="https://x/y/">
      <script src="/iwafune/a.js"></script><script>var u="https://evil/";</script>
      <a href="/iwafune/guide/">g</a><img src="/iwafune/i.png" srcset="/iwafune/i1.webp 1x, /iwafune/i2.webp 2x">`;
    const refs = extractRefs(html);
    expect(refs).toContainEqual({ kind: "resource", url: "/iwafune/a.css" });
    expect(refs).toContainEqual({ kind: "meta", url: "https://x/y/" });
    expect(refs).toContainEqual({ kind: "resource", url: "/iwafune/a.js" });
    expect(refs).toContainEqual({ kind: "a", url: "/iwafune/guide/" });
    expect(refs).toContainEqual({ kind: "resource", url: "/iwafune/i2.webp" });
    expect(refs.some((r) => r.url.includes("evil"))).toBe(false);
  });
});

describe("extractCssRefs", () => {
  it("url() と @import を集め、data: は除く", () => {
    const css =
      '@import "a.css"; .x{background:url(data:image/png;base64,AA)} @font-face{src:url("f.woff2")}';
    expect(extractCssRefs(css)).toEqual(["f.woff2", "a.css"]);
  });
});

describe("failedFormulas", () => {
  it("描けなかった数式を数える", () => {
    expect(failedFormulas('<p><span class="katex">x</span></p>')).toBe(0);
    expect(
      failedFormulas(
        '<span class="katex-error" title="e">a</span><span class="katex-error">b</span>',
      ),
    ).toBe(2);
  });
});
