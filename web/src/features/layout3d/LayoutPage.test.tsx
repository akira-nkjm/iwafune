import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { EffectCallback } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { q } from "@/api/queries";
import type { AnalysisRunResult, ConfigurationView, PartRow } from "@/api/types";
import { paths } from "@/lib/nav";
import { fixture, mountAt } from "@/test/fixture";
import type { Layout3DProps } from "./Layout3D";
import { LayoutPage } from "./LayoutPage";

const route = vi.hoisted(() => ({ params: new URLSearchParams(), set: vi.fn(), effect: vi.fn() }));

vi.mock("react", async (original) => ({
  ...(await original<typeof import("react")>()),
  useEffect: (effect: EffectCallback, deps?: readonly unknown[]) => {
    if (deps?.includes(route.params)) route.effect(effect);
  },
}));
vi.mock("react-router", async (original) => ({
  ...(await original<typeof import("react-router")>()),
  useSearchParams: () => [route.params, route.set],
}));

// ページの取得・重心の受け渡しを WebGL なしで確認する。
vi.mock("./LazyLayout3D", () => ({
  LazyLayout3D: (props: Layout3DProps) => (
    <div
      data-variant={props.variant}
      data-configuration={props.variant === "placement" ? (props.configuration ?? "") : ""}
      data-overridden={JSON.stringify(
        props.design.parts.filter((p) => p.mount?.overridden).map((p) => p.id),
      )}
      data-com={JSON.stringify(props.variant === "placement" ? props.com : null)}
    />
  ),
}));

const stowed: ConfigurationView = {
  id: "STOWED",
  label: "Stowed",
  label_en: null,
  description: null,
  overridden: [fixture.parts[0]!.id],
  text: { id: "STOWED" },
  file: "configurations/STOWED.toml",
};
const stowedParts: PartRow[] = fixture.parts.map((part, index) =>
  index === 0 ? { ...part, mount: { ...mountAt("ROOT"), overridden: true } } : part,
);

function render(
  mass?: AnalysisRunResult,
  url = paths.layout(),
  configurations: ConfigurationView[] = [],
  stowedMass?: AnalysisRunResult,
) {
  route.params = new URL(url, "https://example.test").searchParams;
  route.set.mockClear();
  route.effect.mockClear();
  const client = new QueryClient();
  client.setQueryData(q.design().queryKey, { ...fixture, configurations });
  client.setQueryData(q.parts().queryKey, fixture.parts);
  client.setQueryData(q.parts(stowed.id).queryKey, stowedParts);
  if (mass) client.setQueryData(q.analysis("structure.mass_properties", null).queryKey, mass);
  if (stowedMass)
    client.setQueryData(
      q.analysis("structure.mass_properties", null, [], stowed.id).queryKey,
      stowedMass,
    );
  const html = renderToStaticMarkup(
    <QueryClientProvider client={client}>
      <LayoutPage />
    </QueryClientProvider>,
  );
  client.clear();
  return html;
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
  overview: { main: [], checks: [], chart: null, hidden_tables: [] },
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

it("URL の形態に切り替えると対応する部品と、その形態の配置で求めた重心を表示する", () => {
  const stowedMass: AnalysisRunResult = {
    ...mass,
    output: { summary: { center_of_mass: { x: 0.4, y: 0.5, z: 0.6 } } },
  };
  const html = render(mass, paths.layout(stowed.id), [stowed], stowedMass);
  expect(html).toContain('data-configuration="STOWED"');
  expect(html).toContain(`data-overridden="[&quot;${stowedParts[0]!.id}&quot;]"`);
  expect(html).toContain('data-com="[0.4,0.5,0.6]"');
  expect(html).toContain('value="STOWED" selected=""');
  // その形態の重心が未取得の間は、基準の重心を出さずに印を隠す
  expect(render(mass, paths.layout(stowed.id), [stowed])).toContain('data-com="null"');
});

it.each(["UNKNOWN", stowed.id])("未知または削除済みの形態 %s は基準に戻る", (id) => {
  const url = `${paths.layout(id)}&other=keep`;
  if (id === stowed.id)
    expect(render(mass, url, [stowed])).toContain('data-configuration="STOWED"');
  const html = render(mass, url);
  expect(html).toContain('data-configuration=""');
  expect(html).toContain('data-overridden="[]"');
  expect(html).toContain('data-com="[0.1,0.2,0.3]"');
  route.effect.mock.lastCall?.[0]();
  expect(route.set).toHaveBeenCalledExactlyOnceWith(new URLSearchParams("other=keep"), {
    replace: true,
  });
  render(mass, `${paths.layout()}?other=keep`);
  route.effect.mock.lastCall?.[0]();
  expect(route.set).not.toHaveBeenCalled();
});
