import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { relevantParts } from "./relevant";

describe("系のページに出す部品と属性", () => {
  it("電源: 電力を持つ部品と、電源の role の部品。列はその系の属性だけ", () => {
    const r = relevantParts(fixture, "power");
    expect(r.parts.map((p) => p.id)).toEqual(["RW_X", "PCDU"]);
    expect(r.attrs).toEqual(["power"]);
  });
  it("AOCS: max_momentum を持つ RW だけ", () => {
    const r = relevantParts(fixture, "aocs");
    expect(r.parts.map((p) => p.id)).toEqual(["RW_X"]);
    expect(r.attrs).toEqual(["max_momentum"]);
  });
  it("通信: 送信機（role）と tx_power", () => {
    const r = relevantParts(fixture, "comm");
    expect(r.parts.map((p) => p.id)).toEqual(["TRX"]);
    expect(r.attrs).toEqual(["tx_power"]);
  });
});
