import { describe, expect, it } from "vitest";
import type { QuantityView, ValueView } from "@/api/types";
import { attrEditMode, cellValue } from "./attr-mode";

const q = (display: string, value: number): QuantityView => ({
  value,
  unit: "bit/s",
  si: value,
  dimension: "DataRate",
  display,
});

const view = (value: ValueView["value"]): ValueView => ({
  attr: "hk_rate",
  value,
  display: "",
  origin: "catalog",
  evidence: "datasheet",
  source: null,
  note: null,
  file: null,
  line: null,
});

describe("属性の値の編集の仕方", () => {
  it("属性定義の value_type と by_state だけで決める", () => {
    expect(attrEditMode({ value_type: "scalar", by_state: false })).toBe("scalar");
    expect(attrEditMode({ value_type: "scalar", by_state: true })).toBe("by_state");
    for (const t of ["vec3", "range", "enum", "bool", "text", "quat", "tensor3", "table"] as const)
      expect(attrEditMode({ value_type: t, by_state: false }), t).toBe("display");
    // 定義の無い属性は service が型を決められないので表示だけ
    expect(attrEditMode(undefined)).toBe("display");
  });

  it("状態のセルは状態ごとの値、どの状態でも同じ値ならその値を出す", () => {
    const byState = view({ type: "by_state", states: { on: q("200 bit/s", 200) } });
    expect(cellValue(byState, "on")?.display).toBe("200 bit/s");
    expect(cellValue(byState, "off")).toBeNull();
    const constant = view({ type: "scalar", nominal: q("1 kbit/s", 1000), min: null, max: null });
    expect(cellValue(constant, "off")?.display).toBe("1 kbit/s");
    expect(cellValue(view({ type: "unset" }), "on")).toBeNull();
  });
});
