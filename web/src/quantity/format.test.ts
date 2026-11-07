import { describe, expect, it } from "vitest";
import type { QuantityView, ValueView } from "@/api/types";
import { quantityInputText, stateValue, valueText } from "./format";

const q = (display: string, value: number, unit: string): QuantityView => ({
  value,
  unit,
  si: value,
  dimension: "Power",
  display,
});

const view = (value: ValueView["value"], display = ""): ValueView => ({
  attr: "power",
  value,
  display,
  origin: "catalog",
  evidence: "datasheet",
  source: null,
  note: null,
  file: null,
  line: null,
});

describe("値の表示", () => {
  it("service の display をそのまま使う（換算しない）", () => {
    expect(
      valueText(
        view({ type: "scalar", nominal: q("620 g", 620, "g"), min: null, max: null }, "620 g"),
      ),
    ).toBe("620 g");
  });

  it("状態ごとの値も service の display を使い、状態を引ける", () => {
    const v = view(
      { type: "by_state", states: { off: q("0 W", 0, "W"), on: q("1 W", 1, "W") } },
      "off 0 W / on 1 W",
    );
    expect(valueText(v)).toBe("off 0 W / on 1 W");
    expect(stateValue(v, "on")?.display).toBe("1 W");
    expect(stateValue(v, "peak")).toBeNull();
  });

  it("入力欄には書かれたままの数値と単位を入れる", () => {
    expect(quantityInputText(q("18 mN·m·s", 18, "mN*m*s"))).toBe("18 mN*m*s");
    expect(quantityInputText({ ...q("0.3", 0.3, ""), dimension: "Dimensionless" })).toBe("0.3");
    expect(valueText(view({ type: "unset" }, "x"))).toBe("—");
  });
});
