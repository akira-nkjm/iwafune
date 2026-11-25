import { describe, expect, it } from "vitest";
import type { AttrInput, QuantityView, ValueView } from "@/api/types";
import { attrInput, cellValue, choiceText, isQuantityInput } from "./attr-mode";

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

describe("属性の値の入力の形", () => {
  it("API が返した input をそのまま使い、値の型や属性の名前では分けない", () => {
    const input: AttrInput = { kind: "choice", choices: [] };
    expect(attrInput({ input })).toBe(input);
    // 定義の無い属性は service が入力の形を決められないので表示だけ
    expect(attrInput(undefined)).toBeNull();
  });

  it("パラメーター追加の欄が持つのは量だけ", () => {
    expect(
      isQuantityInput({ input: { kind: "quantity", dimension: "Mass", by_state: false } }),
    ).toBe(true);
    expect(isQuantityInput({ input: { kind: "text" } })).toBe(false);
    expect(isQuantityInput(undefined)).toBe(false);
  });

  it("選ぶ・打つ欄の今の値は、列挙・文字列・真偽の綴り", () => {
    expect(choiceText(view({ type: "enum", value: "xenon" }))).toBe("xenon");
    expect(choiceText(view({ type: "text", value: "XE_TANK" }))).toBe("XE_TANK");
    expect(choiceText(view({ type: "bool", value: false }))).toBe("false");
    expect(choiceText(view({ type: "unset" }))).toBe("");
    expect(choiceText(undefined)).toBe("");
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
