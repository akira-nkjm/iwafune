import { describe, expect, it } from "vitest";
import type { PartRow } from "@/api/types";
import { fixture } from "@/test/fixture";
import { axesOf, boxesOf } from "./scene";

const base = fixture.parts[0]!;

describe("3D 配置", () => {
  it("取付の無い部品は未配置に回す", () => {
    const { boxes, unplaced } = boxesOf(fixture);
    expect(boxes).toEqual([]);
    expect(unplaced).toEqual(["RW_X", "PCDU", "TRX"]);
  });
});

describe("方向ベクトルの矢印", () => {
  it("回転した部品でも矢印を部品座標のまま親 mesh に渡す", () => {
    const s = Math.SQRT1_2;
    const p: PartRow = {
      ...base,
      mount: {
        on: "BUS_FRAME",
        point: null,
        position: [0, 0, 0],
        authored_position: null,
        orientation: [s, s, 0, 0],
        body_position: [0.1, 0, 0],
        body_orientation: [s, s, 0, 0],
      },
      values: {
        ...base.values,
        spin_axis: {
          ...(base.values.mass as PartRow["values"][string]),
          attr: "spin_axis",
          value: { type: "vector", x: 0, y: 0, z: 1, unit: "", si: [0, 0, 1], frame: null },
        },
      },
    };
    const [a] = axesOf({ ...fixture, parts: [p] });
    expect(a?.part).toBe(p.id);
    expect(a?.dir[0]).toBeCloseTo(0);
    expect(a?.dir[1]).toBeCloseTo(0);
    expect(a?.dir[2]).toBeCloseTo(1);
  });
});
