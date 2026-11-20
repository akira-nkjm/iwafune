import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { getT } from "@/i18n";
import { fixture } from "@/test/fixture";
import { Layout3D, type Layout3DProps } from "./Layout3D";

// three.js の場面（WebGL）は node では描かない。用途ごとの帯とインスペクタだけを見る
vi.mock("./Scene3D", () => ({ Scene3D: () => null }));

/** テストの設計は取付が無いので、3D の場面は描かずに見出しの帯とインスペクタだけを出す */
function render(props: Layout3DProps) {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <Layout3D {...props} />
    </QueryClientProvider>,
  );
}

describe("Layout3D", () => {
  const L = getT().systems.layout3d;

  it("構造の配置の編集は、移動・回転とスナップの操作を出し、重心があれば印の説明を添える", () => {
    const html = render({ variant: "placement", design: fixture, com: [0, 0, 0.1] });
    expect(html).not.toContain("<h2");
    expect(html).toContain(L.parts(0));
    expect(html).toContain(L.unplacedCount(fixture.parts.length));
    expect(html).toContain(`aria-label="${L.structureOpacity}"`);
    expect(html).toContain('type="range"');
    expect(html).toContain('value="0.3"');
    expect(html).toContain("flex min-h-0 flex-1 flex-col");
    expect(html).not.toContain("h-[460px]");
    expect(html).toContain(`aria-label="${L.operation}"`);
    expect(html).toContain(`aria-label="${L.snap}"`);
    expect(html).toContain(L.com);
    expect(render({ variant: "placement", design: fixture, com: null })).not.toContain(L.com);
  });

  it("熱の温度の色は、凡例の注記と渡した操作・一覧を出し、動かす操作と配置の欄は出さない", () => {
    const t = getT().systems;
    const html = render({
      variant: "heat",
      design: fixture,
      colorOf: () => "#000000",
      tools: <span>出どころの選択</span>,
      overlay: <span>色の凡例</span>,
      aside: <span>節点の一覧</span>,
    });
    // 3D を大きく見せるため、高さは画面に合わせる
    expect(html).toContain("h-[calc(100dvh-15rem)]");
    expect(html).toContain(t.thermal3dLegend);
    for (const text of ["出どころの選択", "色の凡例", "節点の一覧"]) expect(html).toContain(text);
    expect(html).not.toContain(L.hint);
    expect(html).not.toContain(`aria-label="${L.operation}"`);
    expect(html).not.toContain(`aria-label="${L.snap}"`);
    expect(html).not.toContain(L.com);
    expect(html).not.toContain(`aria-label="${L.structureOpacity}"`);
  });

  it("どちらも未配置の部品をインスペクタに並べる", () => {
    for (const html of [
      render({ variant: "placement", design: fixture, com: null }),
      render({ variant: "heat", design: fixture, colorOf: () => "#000000" }),
    ]) {
      expect(html).toContain(L.empty);
      for (const p of fixture.parts) expect(html).toContain(`>${p.id}</button>`);
    }
  });
});
