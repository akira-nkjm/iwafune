import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { fire } from "@/test/elements";
import { PRESSED } from "./pressed";
import { Toggle } from "./Toggle";

describe("Toggle", () => {
  it("value で押されている見た目と aria-pressed", () => {
    const on = renderToStaticMarkup(
      <Toggle value onChange={() => {}}>
        電源
      </Toggle>,
    );
    expect(on).toContain('aria-pressed="true"');
    for (const c of PRESSED.split(" ")) expect(on).toContain(c);
    const off = renderToStaticMarkup(
      <Toggle value={false} onChange={() => {}}>
        電源
      </Toggle>,
    );
    expect(off).toContain('aria-pressed="false"');
  });

  it("押すと反対の値で onChange", () => {
    const onChange = vi.fn();
    fire(Toggle({ value: true, onChange, children: "電源" }), "onClick");
    expect(onChange).toHaveBeenCalledWith(false);
  });
});
