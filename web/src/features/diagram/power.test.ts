import { describe, expect, it } from "vitest";
import type { NetView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { layerOf, powerCategories } from "./power";

const power = fixture.nets.find((n) => n.kind === "power") as NetView;

describe("ネットの層と系統", () => {
  it("戻り（GND）は API の is_return だけで決める（ID や表示名は見ない）", () => {
    const named = { ...power, id: "GND", label: "GND リターン", is_return: false };
    expect(layerOf(named)).toBe("power");
    const ret = { ...power, id: "P0", is_return: true };
    expect(layerOf(ret)).toBe("gnd");
    const categories = powerCategories({ ...fixture, nets: [ret] });
    expect(categories.get("P0")?.key).toBe("gnd");
  });
  it("データと RF は信号の層", () => {
    for (const n of fixture.nets.filter((x) => x.kind !== "power")) {
      expect(layerOf(n)).toBe("signal");
    }
  });
});
