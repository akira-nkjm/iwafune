import { expect, it } from "vitest";
import { isUnspecified, timingCommand, timingFields } from "./timing";

it("any と空の欄は旗を書かないのと同じで、オフセットは開始の基準があるときだけ送る", () => {
  expect(timingCommand("MISSION", { when: "any", start: null, offset: "5 min" })).toEqual({
    command: "mode_timing",
    mode: "MISSION",
    when: null,
    start: null,
    offset: null,
  });
  expect(timingCommand("MISSION", { when: "sunlit", start: null, offset: "" })).toMatchObject({
    when: "sunlit",
    start: null,
  });
  expect(
    timingCommand("DOWNLINK", { when: null, start: "eclipse_exit", offset: " -5 min " }),
  ).toEqual({
    command: "mode_timing",
    mode: "DOWNLINK",
    when: null,
    start: "eclipse_exit",
    offset: "-5 min",
  });
});

it("保存されたモードの指定を欄の値にし、何も無ければ指定なし", () => {
  const empty = timingFields({ when: null, start: null });
  expect(empty).toEqual({ when: null, start: null, offset: "" });
  expect(isUnspecified(empty)).toBe(true);
  expect(isUnspecified({ when: "any", start: null, offset: "" })).toBe(true);
  const set = timingFields({
    when: "eclipse",
    start: { from: "eclipse_entry", offset: { display: "2.0 min" } as never },
  });
  expect(set).toEqual({ when: "eclipse", start: "eclipse_entry", offset: "2.0 min" });
  expect(isUnspecified(set)).toBe(false);
});
