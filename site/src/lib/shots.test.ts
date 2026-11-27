import { describe, expect, it } from "vitest";
import { findMarkers, parseMarker, shotFileName, shotId, shotSlug } from "./shots.ts";

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
    expect(parseMarker("screenshot: <route> — placeholder text")).toBeNull();
  });
  it("名前付きの目印を読む", () => {
    expect(parseMarker("screenshot: /systems/power as missing-inputs — The list")).toEqual({
      route: "/systems/power",
      name: "missing-inputs",
      description: "The list",
    });
    // 説明の中の " as " は名前にしない。
    expect(parseMarker("screenshot: /a — shown as a table")).toEqual({
      route: "/a",
      description: "shown as a table",
    });
  });
  it("名前があると別の画像になる", () => {
    expect(shotId("/systems/power", "missing-inputs")).toBe("systems-power--missing-inputs");
    expect(shotFileName("/", "ja", "mass-over")).toBe("home--mass-over.ja.png");
    expect(shotFileName("/", "en")).toBe("home.png");
  });
  it("コードブロックの中は読まない", () => {
    const md = "<!-- screenshot: /a — A -->\n```md\n<!-- screenshot: /b — B -->\n```\n";
    expect(findMarkers(md).map((m) => m.route)).toEqual(["/a"]);
    expect(findMarkers("例: `<!-- screenshot: /x — X -->` と書く")).toEqual([]);
  });
});
