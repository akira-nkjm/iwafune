import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { BudgetUncounted } from "@/api/types";
import { dictionaries } from "@/i18n";
import { BudgetUncountedNote } from "./BudgetUncounted";

const o = dictionaries.ja.overview;

describe("バジェットで数えなかった部品", () => {
  it("空なら何も出さない", () => {
    expect(renderToStaticMarkup(<BudgetUncountedNote uncounted={[]} />)).toBe("");
  });

  it("件数と部品の名前を 1 行で出し、電力は数えなかったモードを添える", () => {
    const uncounted: BudgetUncounted[] = [
      { part: "CAM", label: "カメラ", modes: [] },
      { part: "HTR", label: "ヒーター", modes: ["NOMINAL", "DOWNLINK"] },
    ];
    const html = renderToStaticMarkup(<BudgetUncountedNote uncounted={uncounted} />);
    expect(html).toContain(o.budgetUncounted(2, "カメラ, ヒーター [NOMINAL, DOWNLINK]"));
  });
});
