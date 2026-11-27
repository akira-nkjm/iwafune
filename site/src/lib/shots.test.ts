import { describe, expect, it } from "vitest";
import { findMarkers, parseMarker, shotFileName, shotSlug } from "./shots.ts";

describe("shotSlug", () => {
  it("ルートをファイル名の元にする", () => {
    expect(shotSlug("/")).toBe("home");
    expect(shotSlug("/design/components")).toBe("design-components");
    expect(shotSlug("/Design/Power_Budget/?tab=x#y")).toBe("design-power-budget");
  });
  it("言語で拡張子の前が変わる", () => {
    expect(shotFileName("/overview", "en")).toBe("overview.png");
    expect(shotFileName("/overview", "ja")).toBe("overview.ja.png");
  });
});

describe("parseMarker / findMarkers", () => {
  it("ルートと説明を読む", () => {
    expect(parseMarker(" screenshot: /overview — The overview page ")).toEqual({
      route: "/overview",
      description: "The overview page",
    });
    expect(parseMarker("screenshot: /a - plain hyphen")?.description).toBe("plain hyphen");
    expect(parseMarker("TODO later")).toBeNull();
  });
  it("コードブロックの中は読まない", () => {
    const md = "<!-- screenshot: /a — A -->\n```md\n<!-- screenshot: /b — B -->\n```\n";
    expect(findMarkers(md).map((m) => m.route)).toEqual(["/a"]);
  });
});
