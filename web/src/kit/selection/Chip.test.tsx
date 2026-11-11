import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { fire } from "@/test/elements";
import { Chip, ChipLink } from "./Chip";
import { PRESSED } from "./pressed";

describe("Chip", () => {
  it("選ばれていれば aria-pressed=true で、Toggle と同じ見た目", () => {
    const html = renderToStaticMarkup(
      <Chip selected onSelect={() => {}}>
        RW_X
      </Chip>,
    );
    expect(html).toContain('aria-pressed="true"');
    for (const c of PRESSED.split(" ")) expect(html).toContain(c);
    expect(html).toContain("font-mono");
  });

  it("選ばれていなければ aria-pressed=false、selected を渡さなければ aria-pressed を付けない", () => {
    const off = renderToStaticMarkup(
      <Chip selected={false} onSelect={() => {}}>
        A
      </Chip>,
    );
    expect(off).toContain('aria-pressed="false"');
    expect(off).not.toContain("bg-accent-tint");
    const plain = renderToStaticMarkup(<Chip onSelect={() => {}}>A</Chip>);
    expect(plain).not.toContain("aria-pressed");
  });

  it("押すと onSelect を呼ぶ（フォームの中でも送信しない button）", () => {
    const onSelect = vi.fn();
    const element = Chip({ onSelect, children: "A" });
    expect(element.props.type).toBe("button");
    fire(element, "onClick");
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("名前を出すときは等幅にしない", () => {
    const html = renderToStaticMarkup(
      <Chip mono={false} onSelect={() => {}}>
        電源
      </Chip>,
    );
    expect(html).not.toContain("font-mono");
  });
});

describe("ChipLink", () => {
  it("開き先へのリンクで、aria-pressed は付けない", () => {
    const html = renderToStaticMarkup(
      <MemoryRouter>
        <ChipLink to="/design/components/RW_X">RW_X</ChipLink>
      </MemoryRouter>,
    );
    expect(html).toContain('href="/design/components/RW_X"');
    expect(html).not.toContain("aria-pressed");
  });
});
