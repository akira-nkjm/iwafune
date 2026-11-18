import { describe, expect, it } from "vitest";
import type { NetView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { layerOf, powerCategories } from "./power";

const power = fixture.nets.find((n) => n.kind === "power") as NetView;

describe("ネットの層と系統", () => {
  it("戻り（GND）は API の is_return だけで決める（ID や表示名は見ない）", () => {
    const named = { ...power, id: "GND", label: "GND リターン", is_return: false };
    expect(layerOf(named)).toBe("power");
    const ret = { ...power, id: "P0", is_return: true, power_class: "return" as const };
    expect(layerOf(ret)).toBe("gnd");
    const categories = powerCategories({ ...fixture, nets: [ret] });
    expect(categories.get("P0")?.key).toBe("gnd");
  });
  it("系統は API の power_class で決め、安定化レールだけ電圧ごとに色を分ける", () => {
    const rail = (si: number, display: string) =>
      ({ ...power.rail, si, display }) as NetView["rail"];
    const nets: NetView[] = [
      { ...power, id: "B", power_class: "battery" },
      { ...power, id: "S", power_class: "solar" },
      { ...power, id: "U", power_class: "unregulated", rail: null },
      { ...power, id: "R5", power_class: "regulated", rail: rail(5, "5 V") },
      { ...power, id: "R12", power_class: "regulated", rail: rail(12, "12 V") },
      { ...power, id: "R5b", power_class: "regulated", rail: rail(5, "5 V") },
    ];
    const c = powerCategories({ ...fixture, nets });
    expect(["B", "S", "U"].map((id) => c.get(id)?.key)).toEqual([
      "battery",
      "solar",
      "unregulated",
    ]);
    expect(c.get("R12")?.order).toBe(40);
    expect(c.get("R5")?.order).toBe(41);
    expect(c.get("R5b")?.key).toBe(c.get("R5")?.key);
  });
  it("データと RF は信号の層", () => {
    for (const n of fixture.nets.filter((x) => x.kind !== "power")) {
      expect(layerOf(n)).toBe("signal");
    }
  });
});
