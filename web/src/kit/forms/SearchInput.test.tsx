import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { byLabel, fire } from "@/test/elements";
import { SearchInput } from "./SearchInput";
import { matchesSearch } from "./search";

describe("SearchInput", () => {
  it("欄の名前（aria-label）を付け、placeholder を省くと名前を出す", () => {
    const html = renderToStaticMarkup(
      <SearchInput label="部品を検索" value="rw" onChange={() => {}} />,
    );
    expect(html).toContain('aria-label="部品を検索"');
    expect(html).toContain('placeholder="部品を検索"');
    expect(html).toContain('value="rw"');
    // アイコンは読み上げない
    expect(html).toContain('aria-hidden="true"');
    const custom = renderToStaticMarkup(
      <SearchInput label="部品を検索" placeholder="ID・種別" value="" onChange={() => {}} />,
    );
    expect(custom).toContain('placeholder="ID・種別"');
  });

  it("打つたびに文字列で onChange を呼ぶ", () => {
    const onChange = vi.fn();
    const input = byLabel(SearchInput({ label: "検索", value: "", onChange }), "検索");
    fire(input, "onChange", { target: { value: "rw_" } });
    expect(onChange).toHaveBeenCalledWith("rw_");
  });
});

describe("matchesSearch", () => {
  it("前後の空白を除き、大文字と小文字を区別しない部分一致", () => {
    expect(matchesSearch("  rw ", ["RW_X", "reaction_wheel"])).toBe(true);
    expect(matchesSearch("obc", ["RW_X", null])).toBe(false);
    expect(matchesSearch("wheel", [null, "reaction_wheel"])).toBe(true);
  });

  it("空の検索は何にでも一致する", () => {
    expect(matchesSearch("", [])).toBe(true);
    expect(matchesSearch("   ", [null])).toBe(true);
  });
});
