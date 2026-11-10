import { Pencil } from "lucide-react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { getT } from "@/i18n";
import { toneTint } from "@/lib/tone";
import { fire } from "@/test/elements";
import { DetailAction, DetailHeader, DetailPane } from "./DetailPane";
import { DetailSection } from "./DetailSection";
import { TablePanel } from "./TablePanel";

describe("DetailHeader", () => {
  it("名前と補足を出し、閉じるボタンは onClose があるときだけ（aria-label 付き）", () => {
    const close = getT().common.close;
    const with_ = renderToStaticMarkup(
      <DetailHeader title="RW_X" subtitle="姿勢 · reaction_wheel" onClose={() => {}} />,
    );
    expect(with_).toContain("RW_X");
    expect(with_).toContain("姿勢 · reaction_wheel");
    expect(with_).toContain(`aria-label="${close}"`);
    const without = renderToStaticMarkup(<DetailHeader title="RW_X" />);
    expect(without).not.toContain(`aria-label="${close}"`);
  });

  it("tone を渡すと、サブシステム色の淡い地と左の色の線", () => {
    const html = renderToStaticMarkup(<DetailHeader title="RW_X" tone="blue" />);
    expect(html).toContain(`style="background:${toneTint("blue", "header")}"`);
    expect(html).toContain("border-cat-blue");
    expect(renderToStaticMarkup(<DetailHeader title="RW_X" />)).not.toContain("style=");
  });

  it("操作（actions）を閉じるボタンの前に並べる", () => {
    const html = renderToStaticMarkup(
      <DetailHeader
        title="RW_X"
        actions={<DetailAction icon={Pencil} label="名前を変える" onClick={() => {}} />}
        onClose={() => {}}
      />,
    );
    expect(html.indexOf("名前を変える")).toBeLessThan(html.indexOf(getT().common.close));
  });
});

describe("DetailAction", () => {
  it("アイコンだけのボタンに、title と aria-label の両方で名前を付ける", () => {
    const onClick = vi.fn();
    const html = renderToStaticMarkup(
      <DetailAction icon={Pencil} label="名前を変える" onClick={onClick} />,
    );
    expect(html).toContain('aria-label="名前を変える"');
    expect(html).toContain('title="名前を変える"');
    fire(DetailAction({ icon: Pencil, label: "名前を変える", onClick }), "onClick");
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe("DetailPane と DetailSection", () => {
  it("見出しの下に、区分（名前付きの section）を並べる", () => {
    const html = renderToStaticMarkup(
      <DetailPane header={<DetailHeader title="RW_X" />}>
        <DetailSection title="基本">中身</DetailSection>
      </DetailPane>,
    );
    expect(html.indexOf("RW_X")).toBeLessThan(html.indexOf("基本"));
    expect(html).toContain('<section class="space-y-1.5" aria-label="基本">');
    expect(html).toContain("中身");
  });
});

describe("TablePanel", () => {
  it("見出し（題・説明・右の操作）の下に中身を置く", () => {
    const html = renderToStaticMarkup(
      <TablePanel title="属性" caption="12 件" right={<button type="button">追加</button>}>
        <table />
      </TablePanel>,
    );
    expect(html).toContain("<h2");
    expect(html).toContain("属性");
    expect(html).toContain("12 件");
    expect(html).toContain("追加");
    expect(html.indexOf("属性")).toBeLessThan(html.indexOf("<table"));
  });
});
