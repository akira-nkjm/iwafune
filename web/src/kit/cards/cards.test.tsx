import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { findElement, fire } from "@/test/elements";
import { CARD_WIDTH, CardGrid, columnsFor } from "./CardGrid";
import { GroupBox } from "./GroupBox";
import { ItemCard } from "./ItemCard";

const card = (props: { count?: number; selected?: boolean; onSelect?: () => void }) =>
  ItemCard({
    role: "battery",
    tone: "gray",
    caption: "バッテリー",
    name: "BAT_1",
    title: "説明",
    selected: false,
    onSelect: () => {},
    width: CARD_WIDTH,
    ...props,
  });

describe("ItemCard", () => {
  it("補足・名前・説明を描き、選ばれていれば輪を付ける", () => {
    const html = renderToStaticMarkup(card({ selected: true }));
    expect(html).toContain("バッテリー");
    expect(html).toContain("BAT_1");
    expect(html).toContain('title="説明"');
    expect(html).toContain("ring-accent");
    expect(renderToStaticMarkup(card({}))).not.toContain("ring-accent");
  });

  it("押すと onSelect を呼び、空いた所の選択解除へ伝えない", () => {
    const onSelect = vi.fn();
    const stop = vi.fn();
    const button = findElement(card({ onSelect }), (p) => p.type === "button");
    if (!button) throw new Error("button がありません");
    fire(button, "onClick", { stopPropagation: stop });
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(stop).toHaveBeenCalledTimes(1);
  });

  it("員数の丸と重ねた影は 2 以上のときだけ", () => {
    expect(renderToStaticMarkup(card({ count: 3 }))).toContain("4px 4px 0");
    expect(renderToStaticMarkup(card({ count: 3 }))).toContain(">3<");
    for (const count of [1, undefined]) {
      const html = renderToStaticMarkup(card({ count }));
      expect(html).not.toContain("4px 4px 0");
      expect(html).not.toContain("-right-2.5");
    }
  });
});

describe("GroupBox", () => {
  it("onSelectHeading が無ければ見出しは h3、あればボタンで押すと呼ぶ", () => {
    const plain = renderToStaticMarkup(
      <GroupBox tone="gray" prefix="サブシステム" title="電源">
        中身
      </GroupBox>,
    );
    expect(plain).toContain("<h3");
    expect(plain).not.toContain("<button");
    expect(plain).toContain("サブシステム");
    expect(plain).toContain("中身");

    const onSelectHeading = vi.fn();
    const stop = vi.fn();
    const element = GroupBox({ tone: "gray", title: "PCU", onSelectHeading });
    const button = findElement(element, (p) => p.type === "button");
    if (!button) throw new Error("button がありません");
    fire(button, "onClick", { stopPropagation: stop });
    expect(onSelectHeading).toHaveBeenCalledTimes(1);
    expect(stop).toHaveBeenCalledTimes(1);
  });

  it("selected で輪、nested で影の無い濃い地", () => {
    const sel = renderToStaticMarkup(<GroupBox tone="gray" title="A" selected nested />);
    expect(sel).toContain("ring-accent");
    expect(sel).toContain("bg-panel-2/60");
    expect(sel).not.toContain("shadow-panel");
    const top = renderToStaticMarkup(<GroupBox tone="gray" title="A" />);
    expect(top).not.toContain("ring-accent");
    expect(top).toContain("shadow-panel");
  });
});

describe("CardGrid", () => {
  it("枚数から列の数を決める", () => {
    expect([1, 2, 3, 6, 7].map(columnsFor)).toEqual([1, 2, 2, 2, 3]);
    expect(columnsFor(0)).toBe(1);
    const html = renderToStaticMarkup(<CardGrid count={4}>x</CardGrid>);
    expect(html).toContain(`repeat(2, ${CARD_WIDTH}px)`);
  });
});
