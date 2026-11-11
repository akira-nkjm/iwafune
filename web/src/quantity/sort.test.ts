import { describe, expect, it } from "vitest";
import type { ValueData } from "@/api/types";
import { qv } from "@/test/fixture";
import { valueSortSi } from "./sort";

const q = (si: number) => qv(si * 1000, "g", "Mass", si);
const cases: [string, ValueData | null | undefined, number | null][] = [
  [
    "scalar nominal SI, independent of display units and bounds",
    {
      type: "scalar",
      nominal: q(2),
      min: q(1),
      max: q(3),
    },
    2,
  ],
  ["range minimum", { type: "range", min: q(1), max: q(9) }, 1],
  [
    "maximum over all states",
    { type: "by_state", states: { peak: q(9), off: q(0), idle: q(1) } },
    9,
  ],
  ["negative state values", { type: "by_state", states: { a: q(-3), b: q(-1) } }, -1],
  ["zero is a value", { type: "scalar", nominal: q(0), min: null, max: null }, 0],
  ["empty states", { type: "by_state", states: {} }, null],
  ["unset", { type: "unset" }, null],
  ["unsupported value type", { type: "text", value: "2 kg" }, null],
  ["null", null, null],
  ["absent", undefined, null],
];

describe("valueSortSi", () => {
  it.each(cases)("%s", (_, value, expected) => {
    expect(valueSortSi(value)).toBe(expected);
  });
});
