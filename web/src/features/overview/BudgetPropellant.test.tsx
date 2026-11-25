import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import type { BudgetPropellant, QuantityView } from "@/api/types";
import { dictionaries } from "@/i18n";
import { MassTotals, PropellantRow } from "./BudgetPropellant";

const o = dictionaries.ja.overview;

const qty = (display: string): QuantityView => ({
  value: 0,
  unit: "",
  si: 0,
  dimension: "Mass",
  display,
});

const counted: BudgetPropellant = {
  status: "counted",
  analysis: "propulsion.propellant",
  loaded: qty("67.2 g"),
  wet: {
    total: qty("6.30 kg"),
    margin: qty("4.50 kg"),
    verdict: "ok",
    fill: { ratio: 0.53, basis: "used_over_limit" },
  },
};

const render = (node: React.ReactNode) => renderToStaticMarkup(<MemoryRouter>{node}</MemoryRouter>);

describe("質量のバジェットの推薬", () => {
  it("数えたときは乾燥と推薬込みの両方の合計と余裕、推薬の行を出す（値は API の表示文字列）", () => {
    const totals = render(
      <MassTotals
        dry={qty("6.23 kg")}
        dryVerdict="ok"
        dryMargin={qty("4.57 kg")}
        propellant={counted}
      />,
    );
    expect(totals).toContain(o.budgetDry);
    expect(totals).toContain("6.23 kg");
    expect(totals).toContain(o.budgetMargin("4.57 kg"));
    expect(totals).toContain(o.budgetWet);
    expect(totals).toContain("6.30 kg");
    expect(totals).toContain(o.budgetMargin("4.50 kg"));
    const row = render(<PropellantRow propellant={counted} design={undefined} />);
    expect(row).toContain(o.budgetPropellant);
    expect(row).toContain("67.2 g");
  });

  it("タンクが 2 つ以上あれば、タンクごとの積む推薬と容積の余裕（API の表示文字列）を並べる", () => {
    const main = {
      id: "MAIN",
      propellant: "hydrazine",
      loaded: qty("40.0 g"),
      capacity: null,
      margin: qty("12 %"),
    };
    const rcs = {
      id: "RCS_TANK",
      propellant: "hydrazine",
      loaded: qty("27.2 g"),
      capacity: null,
      margin: null,
    };
    const two: BudgetPropellant = { ...counted, tanks: [main, rcs] };
    const row = render(<PropellantRow propellant={two} design={undefined} />);
    expect(row).toContain(o.budgetTank("MAIN"));
    expect(row).toContain("40.0 g");
    expect(row).toContain(o.budgetTankMargin("12 %"));
    expect(row).toContain(o.budgetTank("RCS_TANK"));
    // 1 つだけなら総量と同じなので並べない
    const one = render(
      <PropellantRow propellant={{ ...counted, tanks: [main] }} design={undefined} />,
    );
    expect(one).not.toContain(o.budgetTank("MAIN"));
  });

  it("数えなかったときは推薬込みを出さず、理由（API の構造）を i18n の文言で出す", () => {
    const none: BudgetPropellant = { status: "no_propulsion" };
    const totals = render(
      <MassTotals dry={qty("6.23 kg")} dryVerdict="ok" dryMargin={null} propellant={none} />,
    );
    expect(totals).not.toContain(o.budgetWet);
    expect(render(<PropellantRow propellant={none} design={undefined} />)).toContain(
      o.propellantNone,
    );
    const failed = render(
      <PropellantRow propellant={{ status: "failed", message: "比推力が 0" }} design={undefined} />,
    );
    expect(failed).toContain(o.propellantFailed("比推力が 0"));
  });
});
