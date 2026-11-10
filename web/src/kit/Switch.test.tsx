import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Switch } from "./Switch";

describe("Switch", () => {
  it("value でオン・オフ（role=switch の aria-checked）", () => {
    const on = renderToStaticMarkup(<Switch value onChange={() => {}} aria-label="軸" />);
    expect(on).toContain('role="switch"');
    expect(on).toContain('aria-checked="true"');
    expect(on).toContain("bg-accent");
    const off = renderToStaticMarkup(<Switch value={false} onChange={() => {}} aria-label="軸" />);
    expect(off).toContain('aria-checked="false"');
    expect(off).toContain("bg-line-strong");
  });

  it("文字を添えると label で包み、文字を押しても切り替わる", () => {
    const html = renderToStaticMarkup(
      <Switch value onChange={() => {}}>
        軸
      </Switch>,
    );
    expect(html).toMatch(/^<label/);
    expect(html).toContain("軸");
  });
});
