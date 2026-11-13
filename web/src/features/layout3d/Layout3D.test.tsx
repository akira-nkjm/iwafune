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
    expect(html).toContain(L.title);
    expect(html).toContain("flex min-h-0 flex-1 flex-col");
    expect(html).not.toContain("h-[460px]");
    expect(html).toContain(`aria-label="${L.operation}"`);
    expect(html).toContain(`aria-label="${L.snap}"`);
    expect(html).toContain(L.com);
    expect(render({ variant: "placement", design: fixture, com: null })).not.toContain(L.com);
  });

  it("熱の温度の色は、熱の見出しと凡例を出し、動かす操作は出さない", () => {
    const t = getT().systems;
    const html = render({ variant: "heat", design: fixture, colorOf: () => "#000000" });
    expect(html).toContain(t.thermal3d);
    expect(html).toContain("h-[460px]");
    expect(html).toContain(t.thermal3dLegend);
    expect(html).not.toContain(`aria-label="${L.operation}"`);
    expect(html).not.toContain(`aria-label="${L.snap}"`);
    expect(html).not.toContain(L.com);
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
