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

  it("スラスタの選択肢は役割で選ぶ（API の role）", () => {
    const parts = fixture.parts.map((p, i) =>
      i === 0 ? { ...p, role: "propulsion_thruster" as const } : { ...p, role: null },
    );
    const options = thrusterOptions({ ...fixture, parts });
    expect(options.map((o) => o.value)).toEqual([parts[0]?.id]);
  });
});
