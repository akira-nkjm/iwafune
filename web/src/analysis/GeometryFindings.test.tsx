import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AnalysisRunResult } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { GeometryFindings } from "./GeometryFindings";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));

const result = {
  id: "structure.interference",
  status: "ok",
  configuration: "STOWED",
  output: {
    overlaps: [
      { first: "SS_PY", second: "SP_PY", acknowledged: false },
      {
        first: "SAP_PY",
        second: "SP_PY",
        acknowledged: true,
        reason: "Hinge edge",
        acknowledged_configuration: "STOWED",
      },
    ],
  },
} as unknown as AnalysisRunResult;

const render = (props: Partial<Parameters<typeof GeometryFindings>[0]> = {}) =>
  renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <GeometryFindings result={result} {...props} />
    </QueryClientProvider>,
  );

afterEach(() => setLang("ja"));

describe.each(["ja", "en"] as const)("GeometryFindings (%s)", (lang) => {
  const text = () => getT().systems.findings;

  it("承知でない所見と承知済みの所見を状態つきで並べ、承知済みは理由と形態を添える", () => {
    setLang(lang);
    const html = render();
    expect(html).toContain("SS_PY – SP_PY");
    expect(html).toContain("SAP_PY – SP_PY");
    expect(html).toContain(text().open);
    expect(html).toContain(text().acknowledged);
    expect(html).toContain("Hinge edge");
    expect(html).toContain(text().scope("STOWED"));
    // 所見の種類の名前は API の種類から引く
    expect(html).toContain(text().kind.interference);
    // 操作は頼まれたときだけ
    expect(html).not.toContain(`>${text().acknowledge}</button>`);
    expect(html).not.toContain(`>${text().withdraw}</button>`);
  });

  it("操作を出すと、承知でない所見には「承知する」、承知済みには「取り消す」が付く", () => {
    setLang(lang);
    const html = render({ actions: true });
    expect(html.split(`>${text().acknowledge}</button>`).length - 1).toBe(1);
    expect(html.split(`>${text().withdraw}</button>`).length - 1).toBe(1);
  });

  it("部品を選ぶ操作があれば、2 つの部品の名前がそれぞれ部品を選ぶボタンになる", () => {
    setLang(lang);
    const html = render({ onSelectPart: () => {} });
    for (const id of ["SS_PY", "SAP_PY", "SP_PY"])
      expect(html).toContain(`aria-label="${getT().systems.layout3d.findings.select(id)}"`);
  });

  it("所見が無い解析（と、ほかの解析）は何も出さない", () => {
    setLang(lang);
    expect(render({ result: { ...result, output: { overlaps: [] } } as AnalysisRunResult })).toBe(
      "",
    );
    expect(render({ result: { ...result, id: "power.energy_balance" } as AnalysisRunResult })).toBe(
      "",
    );
  });
});
