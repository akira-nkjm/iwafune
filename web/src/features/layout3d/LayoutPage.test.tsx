import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { q } from "@/api/queries";
import type { AnalysisRunResult } from "@/api/types";
import { fixture } from "@/test/fixture";
import type { Layout3DProps } from "./Layout3D";
import { LayoutPage } from "./LayoutPage";

// ページの取得・重心の受け渡しを WebGL なしで確認する。
vi.mock("./LazyLayout3D", () => ({
  LazyLayout3D: (props: Layout3DProps) => (
    <div
      data-variant={props.variant}
      data-com={JSON.stringify(props.variant === "placement" ? props.com : null)}
    />
  ),
}));

function render(mass?: AnalysisRunResult) {
  const client = new QueryClient();
  client.setQueryData(q.design().queryKey, fixture);
  if (mass) client.setQueryData(q.analysis("structure.mass_properties", null).queryKey, mass);
  return renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <LayoutPage />
    </QueryClientProvider>,
  );
}

const mass: AnalysisRunResult = {
  id: "structure.mass_properties",
  case: null,
  case_source: null,
  status: "ok",
  output: { summary: { center_of_mass: { x: 0.1, y: 0.2, z: 0.3 } } },
  input: null,
  checks: [],
  verdict: null,
  kpis: [],
  dims: {},
  labels: {},
  missing: [],
  error: null,
};

describe("LayoutPage", () => {
  it("設計の配置ページとして表示し、質量特性の重心を配置ビューに渡す", () => {
    const html = render(mass);
    expect(html).toContain("配置</h1>");
    expect(html).toContain('data-variant="placement"');
    expect(html).toContain('data-com="[0.1,0.2,0.3]"');
    expect(html).toContain("min-h-0 flex-1 flex-col");
  });

  it("解析が未取得・入力不足でも配置を表示する", () => {
    for (const result of [
      undefined,
      { ...mass, status: "missing_inputs" as const, output: null },
    ]) {
      const html = render(result);
      expect(html).toContain('data-variant="placement"');
      expect(html).toContain('data-com="null"');
    }
  });
});
