import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { getT, setLang } from "@/i18n";
import { diagramHiddenSystems, withDiagramHiddenSystems } from "@/lib/nav";
import { findElement } from "@/test/elements";
import { fixture } from "@/test/fixture";
import { DiagramSystems } from "./DiagramSystems";

vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
afterEach(() => setLang("ja"));

describe("系の表示トグル", () => {
  it("API の一覧と順序だけを使い、日英の表示名と押された状態を出す", () => {
    const defs = {
      ...fixture.defs,
      systems: ["thermal", "structure"] as const,
      labels: {
        ...fixture.defs.labels,
        section: {
          ...fixture.defs.labels.section,
          thermal: { label: "熱", label_en: "Thermal" },
          structure: { label: "構造", label_en: "Structure" },
        },
      },
    };
    for (const lang of ["ja", "en"] as const) {
      setLang(lang);
      const html = renderToStaticMarkup(
        <DiagramSystems
          defs={{ ...defs, systems: [...defs.systems] }}
          hidden={new Set(["structure"])}
          onChange={() => {}}
        />,
      );
      const labels = lang === "ja" ? ["熱", "構造"] : ["Thermal", "Structure"];
      expect(html.indexOf(labels[0]!)).toBeLessThan(html.indexOf(labels[1]!));
      expect(html.match(/aria-pressed=/g)).toHaveLength(2);
      expect(html).toContain('aria-pressed="true"');
      expect(html).toContain('aria-pressed="false"');
      expect(html).toContain(getT().diagram.toolbar.hiddenConnections);
    }
  });

  it("構造を再表示し、URL を読み直しても全表示を維持する", () => {
    const params = new URLSearchParams("other=keep");
    const hidden = diagramHiddenSystems(params, fixture.defs);
    const changed = vi.fn();
    const element = DiagramSystems({ defs: fixture.defs, hidden, onChange: changed });
    const structure = findElement(element, (props) => props.children === "structure");
    expect(structure).not.toBeNull();
    const toggle = structure?.props.onChange;
    if (typeof toggle !== "function") throw new Error("missing toggle");
    toggle(true);
    const next = withDiagramHiddenSystems(params, changed.mock.calls[0]![0]);
    expect(diagramHiddenSystems(new URLSearchParams(next.toString()), fixture.defs).size).toBe(0);
    expect(next.get("other")).toBe("keep");
  });
});
