import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { formatKey } from "@/quantity/format-batch";
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
  power: 2.3,
  shade: 1,
  at: { row: 0, column: 0 },
};

/** 電力の整形（`/units/format`）は届いた値を入れておく */
function render(cell: MatrixCell) {
  const qc = new QueryClient();
  qc.setQueryData(formatKey({ si: 2.3, dimension: "Power", unit: null }), "2.3 W");
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
      shade: 2,
    });
    expect(html).toContain("ring-warn");
    expect(html).toContain("tx · rx 0.5");
    expect(html).not.toContain("rx（tx 10 %）");
  });

  it("未指定のセルは塗らず、未指定と書く", () => {
    const html = render({ ...base, value: null, display: undefined, power: null, shade: -1 });
    const t = getT().modes;
    expect(html).toContain(`aria-label="${t.cellLabel("TRX", "NOMINAL", t.cell.unassigned, "—")}"`);
    expect(html).toContain("text-ink-4");
    for (const shade of ON_SHADES) expect(html).not.toContain(shade);
  });
});
