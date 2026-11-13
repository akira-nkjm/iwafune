import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { formatKey } from "@/quantity/format-batch";
import { qv } from "@/test/fixture";
import { ModeCell } from "./ModeCell";
import { ON_SHADES } from "./matrix";
import { MatrixFocus } from "./matrix-focus";
import type { MatrixCell } from "./matrix-rows";

const base: MatrixCell = {
  mode: "NOMINAL",
  part: "TRX",
  states: ["off", "rx", "tx"],
  value: { state: "rx", duty: { tx: "0.1" } },
  display: "rx（tx 10 %）",
  changed: false,
  quantity: qv(2.3, "W", "Power"),
  shade: 1,
  at: { row: 0, column: 0 },
};

/** 電力の整形（`/units/format`）は届いた値を入れておく */
function render(cell: MatrixCell) {
  const qc = new QueryClient();
  qc.setQueryData(formatKey({ si: 2.3, dimension: "Power", unit: null }), "2.3 W");
  qc.setQueryData(formatKey({ si: 0.25, dimension: "Fraction", unit: null }), "25 %");
  return renderToStaticMarkup(
    <QueryClientProvider client={qc}>
      <table>
        <tbody>
          <MatrixFocus>
            <tr>
              <td>
                <ModeCell cell={cell} onCell={() => {}} />
              </td>
            </tr>
          </MatrixFocus>
        </tbody>
      </table>
    </QueryClientProvider>,
  );
}

describe("ModeCell", () => {
  it("セル 1 つ（MatrixCell）から、記号・電力・保存済みの文言・塗りの段階を出す", () => {
    const html = render(base);
    const t = getT().modes;
    expect(html).toContain(`aria-label="${t.cellLabel("TRX", "NOMINAL", "rx", "2.3 W")}"`);
    expect(html).toContain("◐");
    expect(html).toContain("2.3 W");
    expect(html).toContain("rx（tx 10 %）");
    expect(html).toContain(ON_SHADES[1]);
    expect(html).not.toContain("ring-warn");
  });

  it("下書きで変えたセルは印を付け、打った値をそのまま出す", () => {
    const html = render({
      ...base,
      value: { state: "tx", duty: { rx: "0.5" } },
      changed: true,
      display: undefined,
      shade: 2,
    });
    expect(html).toContain("ring-warn");
    expect(html).toContain("tx · rx 0.5");
    expect(html).not.toContain("rx（tx 10 %）");
  });

  it("未指定のセルは塗らず、未指定と書く", () => {
    const html = render({ ...base, value: null, display: undefined, quantity: null, shade: -1 });
    const t = getT().modes;
    expect(html).toContain(`aria-label="${t.cellLabel("TRX", "NOMINAL", t.cell.unassigned, "—")}"`);
    expect(html).toContain("text-ink-4");
    for (const shade of ON_SHADES) expect(html).not.toContain(shade);
  });
});

it("蝕の上書きだけを黄色い枠で示し、日照と同じセルは薄い文字と読み上げ説明を出す", () => {
  const inherited = render({ ...base, eclipse: true, override: false, quantity: null });
  expect(inherited).toContain(getT().modes.sameAsSunlit);
  expect(inherited).toContain("text-ink-4");
  expect(inherited).not.toContain("ring-warn");
  for (const shade of ON_SHADES) expect(inherited).not.toContain(shade);
  const overridden = render({ ...base, eclipse: true, override: true, quantity: null });
  expect(overridden).toContain(getT().modes.eclipseOverride);
  expect(overridden).toContain("ring-warn");
  expect(overridden).not.toContain(getT().modes.sameAsSunlit);
});

it("保存済みの蝕の割合は整形 API の表示を使う", () => {
  const html = render({
    ...base,
    eclipse: true,
    override: true,
    value: { state: "rx", duty: { tx: "0.25" } },
    savedDuty: { tx: 0.25 },
    display: undefined,
    quantity: null,
  });
  expect(html).toContain("tx 25 %");
  expect(html).not.toContain("tx 0.25");
});

it("上書きを解除した直後も、日照の保存済みの割合は API の整形を引き継ぐ", () => {
  const html = render({ ...base, eclipse: true, override: false, changed: true, quantity: null });
  expect(html).toContain("rx（tx 10 %）");
  expect(html).not.toContain("tx 0.1");
});
