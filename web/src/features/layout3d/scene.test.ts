import { Vector3 } from "three";
import { describe, expect, it } from "vitest";
import type { PartRow } from "@/api/types";
import { systemTone } from "@/lib/systems";
import { toneHex } from "@/lib/tone";
import { fixture } from "@/test/fixture";
import {
  axesOf,
  boundsOf,
  boxesOf,
  boxMaterial,
  DEFAULT_STRUCTURE_OPACITY,
  DIMMED_OPACITY_RATIO,
  pickBox,
} from "./scene";

const base = fixture.parts[0]!;

describe("3D 配置", () => {
  it("取付の無い部品は未配置に回す", () => {
    const { boxes, unplaced } = boxesOf(fixture);
    expect(boxes).toEqual([]);
    expect(unplaced).toEqual(["RW_X", "PCDU", "TRX"]);
  });

  it("取付の根は取付が無くても API の body で原点に置き、動かさない印を付ける", () => {
    const root: PartRow = {
      ...base,
      id: "FRAME",
      body: { position: [0, 0, 0], orientation: [1, 0, 0, 0], root: true },
    };
    const { boxes, unplaced } = boxesOf({ ...fixture, parts: [root] });
    expect(unplaced).toEqual([]);
    expect(boxes[0]).toMatchObject({ id: "FRAME", position: [0, 0, 0], fixed: true });
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
      },
      body: { position: [0.1, 0, 0], orientation: [s, s, 0, 0], root: false },
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

// 外形と表示寸法の比例は CubeSat（0.1 m）と 1 m 級で同じ。
describe("外形に対する表示寸法", () => {
  function designAt(scale: number) {
    const mount = {
      on: "body",
      point: null,
      position: [0, 0, 0] as [number, number, number],
      authored_position: null,
      orientation: [1, 0, 0, 0] as [number, number, number, number],
    };
    const body: PartRow["body"] = { position: [0, 0, 0], orientation: [1, 0, 0, 0], root: false };
    const sized: PartRow = {
      ...base,
      id: "sized",
      mount,
      body,
      values: {
        size: {
          ...base.values.mass!,
          attr: "size",
          value: {
            type: "vector",
            x: scale,
            y: scale / 2,
            z: scale / 3,
            unit: "m",
            si: [scale, scale / 2, scale / 3],
            frame: null,
          },
        },
      },
    };
    const unsized: PartRow = {
      ...base,
      id: "unsized",
      mount,
      body,
      values: {},
      system: "payload",
      parent: "aocs",
    };
    return { ...fixture, parts: [sized, unsized] };
  }

  it("外形が 10 倍なら仮の箱・矢印・重心・軸・グリッド・カメラも 10 倍", () => {
    const small = boxesOf(designAt(0.1));
    const large = boxesOf(designAt(1));
    for (const key of Object.keys(small.dimensions) as (keyof typeof small.dimensions)[]) {
      expect(large.dimensions[key]).toBeCloseTo(small.dimensions[key] * 10);
    }
    expect(large.boxes[1]!.size[0]).toBeCloseTo(small.boxes[1]!.size[0] * 10);
  });

  it("親の名前によらず API の system で色を決める", () => {
    const { boxes } = boxesOf(designAt(0.1));
    expect(boxes[1]!.color).toBe(toneHex[systemTone("payload")]);
    const design = designAt(0.1);
    design.parts[1]!.system = null;
    expect(boxesOf(design).boxes[1]!.color).toBe(toneHex.gray);
  });

  it("回転した箱の外形も含め、未寸法だけでも有限の寸法にする", () => {
    const design = designAt(1);
    design.parts[0]!.body = {
      position: [0, 0, 0],
      orientation: [Math.SQRT1_2, 0, 0, Math.SQRT1_2],
      root: false,
    };
    const b = boundsOf(boxesOf(design).boxes).getSize(new Vector3());
    expect(b.x).toBeCloseTo(0.5);
    expect(b.y).toBeCloseTo(1);
    const only = boxesOf({ ...design, parts: [design.parts[1]!] });
    expect(only.boxes[0]!.size[0]).toBeGreaterThan(0);
    expect(Object.values(only.dimensions).every(Number.isFinite)).toBe(true);
  });
});

describe("構造の透過表示", () => {
  const mount: NonNullable<PartRow["mount"]> = {
    on: "",
    point: null,
    position: [0, 0, 0],
    authored_position: null,
    orientation: [1, 0, 0, 0],
  };
  const body: PartRow["body"] = { position: [0, 0, 0], orientation: [1, 0, 0, 0], root: false };

  it("名前・親によらず API の system が構造の部品だけを透過対象にする", () => {
    const parts: PartRow[] = [
      { ...base, id: "internal", system: "structure", parent: "power", mount, body },
      { ...base, id: "PANEL_FRAME", system: "power", parent: "structure", mount, body },
    ];
    const { boxes } = boxesOf({ ...fixture, parts });
    expect(boxes.map((b) => b.structure)).toEqual([true, false]);
    expect(boxMaterial(boxes[0]!, false, DEFAULT_STRUCTURE_OPACITY).opacity).toBe(0.3);
    expect(boxMaterial(boxes[1]!, false, 0.3).opacity).toBe(0.45);
  });

  it("不透明度を連続して変えても透過モードは一定で、選択中は常に不透明", () => {
    const box = boxesOf({ ...fixture, parts: [{ ...base, system: "structure", mount, body }] })
      .boxes[0]!;
    for (const opacity of [0, 0.15, 0.3, 0.75, 1]) {
      expect(boxMaterial(box, false, opacity)).toMatchObject({
        transparent: true,
        opacity,
        depthWrite: opacity === 1,
      });
      expect(boxMaterial(box, true, opacity)).toMatchObject({ opacity: 1, depthWrite: true });
    }
    expect(boxMaterial(box, false).opacity).toBe(0.45);
    // ほかの部品が選ばれているときは淡く、選んだ部品は不透明のまま
    expect(boxMaterial(box, false, undefined, true).opacity).toBeCloseTo(
      0.45 * DIMMED_OPACITY_RATIO,
    );
    expect(boxMaterial(box, true, 0.3, true).opacity).toBe(1);
    expect(boxMaterial({ ...box, sized: true }, false).opacity).toBe(0.85);
    expect(boxMaterial({ ...box, structure: false }, true, 0).opacity).toBe(1);
  });
});

describe("クリックで選ぶ部品", () => {
  it("いちばん手前を選び、選んでいる部品を押すと奥へ順に移り、最後は手前に戻る", () => {
    const hits = ["PANEL", "PANEL", "FRAME", "RW"];
    expect(pickBox(hits, null)).toBe("PANEL");
    expect(pickBox(hits, "PANEL")).toBe("FRAME");
    expect(pickBox(hits, "FRAME")).toBe("RW");
    expect(pickBox(hits, "RW")).toBe("PANEL");
    expect(pickBox(hits, "OTHER")).toBe("PANEL");
    expect(pickBox([], null)).toBeNull();
  });
});
