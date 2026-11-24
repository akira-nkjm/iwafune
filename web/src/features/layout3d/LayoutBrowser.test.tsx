import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { nodeLabel } from "@/lib/design";
import { fixture, mountAt } from "@/test/fixture";
import { LayoutBrowser } from "./LayoutBrowser";

const L = getT().systems.layout3d;

/** RW_X と PCDU は 3D に箱があり、TRX は未配置 */
function render(
  hidden: string[] = [],
  selected: string | null = null,
  design = fixture,
  configuration: string | null = null,
) {
  return renderToStaticMarkup(
    <LayoutBrowser
      design={design}
      configuration={configuration}
      placed={new Set(["RW_X", "PCDU"])}
      selected={selected}
      onSelect={() => {}}
      hidden={new Set(hidden)}
      onHidden={() => {}}
    />,
  );
}

describe("LayoutBrowser", () => {
  it("製品ツリーのフォルダーと部品に、3D に箱があるものだけ目のアイコンを出す", () => {
    const html = render();
    const aocs = nodeLabel(fixture, "aocs");
    expect(html).toContain('role="tree"');
    expect(html).toContain(`aria-label="${L.hide(aocs)}"`);
    expect(html).toContain(`aria-label="${L.hide("RW_X")}"`);
    expect(html).toContain(`aria-label="${L.hide("PCDU")}"`);
    expect(html).not.toContain(`aria-label="${L.hide("TRX")}"`);
    expect(html).not.toContain(`aria-label="${L.hide(nodeLabel(fixture, "comm"))}"`);
    expect(html).not.toContain(L.showAll);
  });

  it("隠した部品は「表示」に変わり、すべて表示のボタンを出す。狭い欄では質量の列を省く", () => {
    const html = render(["RW_X"]);
    expect(html).toContain(`aria-label="${L.show("RW_X")}"`);
    expect(html).toContain(`aria-label="${L.show(nodeLabel(fixture, "aocs"))}"`);
    expect(html).toContain('aria-pressed="false"');
    expect(html).toContain(L.showAll);
    expect(html).not.toContain(`title="${getT().components.table.mass}"`);
  });

  it("3D で選んだ部品をツリーでも選ぶ", () => {
    const html = render([], "PCDU");
    expect(html).toMatch(/aria-label="PCDU"[^>]*aria-selected="true"/);
  });
});

it("上書きの印は、形態を表示しているときに上書きされた部品にだけ出す（根は形態の一覧から読む）", () => {
  const label = getT().components.placement.overridden;
  const design = {
    ...fixture,
    parts: fixture.parts.map((part) =>
      part.id === "RW_X" ? { ...part, mount: { ...mountAt("ROOT"), overridden: true } } : part,
    ),
  };
  expect(render()).not.toContain(label);
  expect(render([], null, design).split(label)).toHaveLength(1);
  expect(render([], null, design, "STOWED").split(label)).toHaveLength(2);
  // 取付の無い部品（根）は API の mount が無いので、形態の `overridden` に載っているかで決める
  const root = { ...fixture.parts[0]!, id: "RW_X", mount: null };
  const rooted = {
    ...fixture,
    parts: [root],
    configurations: [
      {
        id: "STOWED",
        label: "Stowed",
        description: null,
        overridden: ["RW_X"],
        text: { id: "STOWED" },
        file: "configurations/STOWED.toml",
      },
    ],
  };
  expect(render([], null, rooted, "STOWED").split(label)).toHaveLength(2);
  expect(render([], null, rooted).split(label)).toHaveLength(1);
});
