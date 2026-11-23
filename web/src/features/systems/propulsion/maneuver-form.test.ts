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

  it("出発・到着の日（depart・arrive）も読まずにそのまま渡す", () => {
    const text = {
      ...emptyManeuverText(),
      id: "MARS",
      body: "mars",
      depart: "2028-12-10T00:00:00Z",
      arrive: "2029-09-25T00:00:00Z",
    };
    expect(maneuverCommand(text, true)).toEqual({ command: "maneuver_add", ...text });
  });

  it("出発の噴射を行うもの（departure_by）も読まずにそのまま渡す", () => {
    const text = {
      ...emptyManeuverText(),
      id: "MARS",
      body: "mars",
      depart: "2028-12-10T00:00:00Z",
      arrive: "2029-09-25T00:00:00Z",
      departure_by: "launcher",
    };
    expect(maneuverCommand(text, false)).toEqual({ command: "maneuver_set", ...text });
  });

  it("スラスタの選択肢は API の maneuver_thruster で選ぶ（役割は見ない）", () => {
    const parts = fixture.parts.map((p, i) => ({ ...p, maneuver_thruster: i === 0 }));
    const options = thrusterOptions({ ...fixture, parts });
    expect(options.map((o) => o.value)).toEqual([parts[0]?.id]);
  });
});
