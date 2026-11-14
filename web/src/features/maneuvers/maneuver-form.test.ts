import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { emptyManeuverText, maneuverCommand, thrusterOptions } from "./maneuver-form";

describe("マヌーバの欄 → fune maneuver add|set の引数", () => {
  it("ID が空なら送らず、打った文字列は読まずにそのまま渡す", () => {
    expect(maneuverCommand(emptyManeuverText(), true)).toBeNull();
    const text = { ...emptyManeuverText(), id: "RAISE", altitude: "600 km", sunlit_only: "true" };
    expect(maneuverCommand(text, true)).toEqual({ command: "maneuver_add", ...text });
    expect(maneuverCommand(text, false)?.command).toBe("maneuver_set");
  });

  it("スラスタの選択肢は API の maneuver_thruster で選ぶ（役割は見ない）", () => {
    const parts = fixture.parts.map((p, i) => ({ ...p, maneuver_thruster: i === 0 }));
    const options = thrusterOptions({ ...fixture, parts });
    expect(options.map((o) => o.value)).toEqual([parts[0]?.id]);
  });
});
