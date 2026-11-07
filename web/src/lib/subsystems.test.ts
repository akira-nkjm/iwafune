import { describe, expect, it } from "vitest";
import { subsystemKey, subsystemTone } from "./subsystems";

describe("サブシステムの色", () => {
  it("対応（AOCS=blue, COM=gold, POWER=green, C&DH=navy, PROP=red, PL=rose）", () => {
    expect(subsystemTone("AOCS")).toBe("blue");
    expect(subsystemTone("COM")).toBe("gold");
    expect(subsystemTone("POWER")).toBe("green");
    expect(subsystemTone("C&DH")).toBe("navy");
    expect(subsystemTone("PROP")).toBe("red");
    expect(subsystemTone("PAYLOAD")).toBe("rose");
  });

  it("service が解決したサブシステムを使い、不明なら OTHER", () => {
    expect(subsystemKey(null, "reaction_wheel")).toBe("OTHER");
    expect(subsystemKey("misc", "unknown")).toBe("OTHER");
    expect(subsystemKey("structure")).toBe("STR");
  });
});
