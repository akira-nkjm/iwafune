import { Vector3 } from "three";
import { describe, expect, it } from "vitest";
import type { AttributeDefView, PartRow } from "@/api/types";
import { systemTone } from "@/lib/systems";
import { toneHex } from "@/lib/tone";
import { bodyAt, fixture, mountAt } from "@/test/fixture";
import {
  axesOf,
  boundsOf,
  boxesOf,
  boxMaterial,
  DEFAULT_STRUCTURE_OPACITY,
  DIMMED_OPACITY_RATIO,
  directionAttrs,
  FOV_DRAW_MAX,
  fovsOf,
  hasSize,
  pickBox,
  WIREFRAME_OPACITY,
  WIREFRAME_SELECTED_OPACITY,
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
      body: bodyAt([0, 0, 0], [1, 0, 0, 0], true),
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
      mount: mountAt("BUS_FRAME", [0, 0, 0], [s, s, 0, 0]),
      body: bodyAt([0.1, 0, 0], [s, s, 0, 0]),
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

describe("方向と視野の属性は API の定義で決まる", () => {
  const vec = (attr: string, si: [number, number, number]) => ({
    ...(base.values.mass as PartRow["values"][string]),
    attr,
    value: { type: "vector" as const, x: si[0], y: si[1], z: si[2], unit: "", si, frame: null },
  });
  const scalarAngle = (attr: string, rad: number) => ({
    ...(base.values.mass as PartRow["values"][string]),
    attr,
    value: {
      type: "scalar" as const,
      nominal: { value: rad, unit: "rad", si: rad, dimension: "Angle", display: "" },
      min: null,
      max: null,
    },
  });
  const def = (id: string, extra: Partial<AttributeDefView>): AttributeDefView => ({
    ...(fixture.defs.attributes[0] as AttributeDefView),
    id,
    builtin: false,
    unit_vector: false,
    fov_axis: null,
    ...extra,
  });
  const onBody = (values: PartRow["values"]): PartRow => ({
    ...base,
    mount: mountAt("body"),
    body: bodyAt([0, 0, 0], [1, 0, 0, 0]),
    values,
  });

  it("独自の方向の属性も、組み込みのあとに並べて矢印にする（ID は固定で持たない）", () => {
    const defs = {
      ...fixture.defs,
      attributes: [def("zz_custom", { unit_vector: true }), ...fixture.defs.attributes],
    };
    expect(directionAttrs(defs)).toEqual([
      "axis",
      "boresight",
      "direction",
      "normal",
      "spin_axis",
      "zz_custom",
    ]);
    const p = onBody({ zz_custom: vec("zz_custom", [0, 3, 0]), mass: base.values.mass as never });
    const [a] = axesOf({ ...fixture, defs, parts: [p] });
    expect(a).toMatchObject({ attr: "zz_custom", order: 5, part: p.id, dir: [0, 1, 0] });
  });

  it("方向の印が無い属性は、ベクトルでも描かない", () => {
    const p = onBody({ foo: vec("foo", [1, 0, 0]) });
    expect(axesOf({ ...fixture, parts: [p] })).toEqual([]);
  });

  it("視野の対応は定義の fov_axis で決まる（独自の対応も描く）", () => {
    const defs = {
      ...fixture.defs,
      attributes: [def("my_axis", { unit_vector: true }), def("my_fov", { fov_axis: "my_axis" })],
    };
    const p = onBody({ my_axis: vec("my_axis", [1, 0, 0]), my_fov: scalarAngle("my_fov", 0.2) });
    expect(fovsOf({ ...fixture, defs, parts: [p] })).toEqual([
      { part: p.id, dir: [1, 0, 0], halfAngle: 0.2 },
    ]);
    expect(fovsOf({ ...fixture, parts: [p] })).toEqual([]);
  });
});

// 外形と表示寸法の比例は CubeSat（0.1 m）と 1 m 級で同じ。
describe("外形に対する表示寸法", () => {
  function designAt(scale: number) {
    const mount = mountAt("body");
    const body: PartRow["body"] = bodyAt([0, 0, 0], [1, 0, 0, 0]);
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

  it("size の無い部品は取付点の印（sized = false、外形に対する一定の大きさ）で、size のある部品は箱", () => {
    const design = designAt(0.1);
    const { boxes, dimensions } = boxesOf(design);
    expect(boxes.map((b) => b.sized)).toEqual([true, false]);
    expect(boxes[1]!.size).toEqual([dimensions.marker, dimensions.marker, dimensions.marker]);
    expect(hasSize(design.parts[0]!)).toBe(true);
    expect(hasSize(design.parts[1]!)).toBe(false);
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
    design.parts[0]!.body = bodyAt([0, 0, 0], [Math.SQRT1_2, 0, 0, Math.SQRT1_2]);
    const b = boundsOf(boxesOf(design).boxes).getSize(new Vector3());
    expect(b.x).toBeCloseTo(0.5);
    expect(b.y).toBeCloseTo(1);
    const only = boxesOf({ ...design, parts: [design.parts[1]!] });
    expect(only.boxes[0]!.size[0]).toBeGreaterThan(0);
    expect(Object.values(only.dimensions).every(Number.isFinite)).toBe(true);
  });
});

describe("構造の透過表示", () => {
  const mount: NonNullable<PartRow["mount"]> = mountAt("");
  const body: PartRow["body"] = bodyAt([0, 0, 0], [1, 0, 0, 0]);

  it("名前・親・系によらず API の `structural` が構造の部品だけを透過対象にする", () => {
    const parts: PartRow[] = [
      { ...base, id: "internal", structural: true, parent: "power", mount, body },
      { ...base, id: "PANEL_FRAME", structural: false, parent: "structure", mount, body },
    ];
    const { boxes } = boxesOf({ ...fixture, parts });
    expect(boxes.map((b) => b.structure)).toEqual([true, false]);
    expect(boxMaterial(boxes[0]!, false, DEFAULT_STRUCTURE_OPACITY).opacity).toBe(0.3);
    expect(boxMaterial(boxes[1]!, false, 0.3).opacity).toBe(0.85);
  });

  it("不透明度を連続して変えても透過モードは一定で、選択中は常に不透明", () => {
    const box = boxesOf({ ...fixture, parts: [{ ...base, structural: true, mount, body }] })
      .boxes[0]!;
    for (const opacity of [0, 0.15, 0.3, 0.75, 1]) {
      expect(boxMaterial(box, false, opacity)).toMatchObject({
        transparent: true,
        opacity,
        depthWrite: opacity === 1,
      });
      expect(boxMaterial(box, true, opacity)).toMatchObject({ opacity: 1, depthWrite: true });
    }
    expect(boxMaterial(box, false).opacity).toBe(0.85);
    // ほかの部品が選ばれているときは淡く、選んだ部品は不透明のまま
    expect(boxMaterial(box, false, undefined, true).opacity).toBeCloseTo(
      0.85 * DIMMED_OPACITY_RATIO,
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

describe("視野の円錐", () => {
  const body: PartRow["body"] = bodyAt([0, 0, 0], [1, 0, 0, 0]);
  const withFov = (deg: number): PartRow => ({
    ...base,
    body,
    values: {
      ...base.values,
      boresight: {
        ...(base.values.mass as PartRow["values"][string]),
        attr: "boresight",
        value: { type: "vector", x: 0, y: 0, z: 2, unit: "", si: [0, 0, 2], frame: null },
      },
      fov_half_angle: {
        ...(base.values.mass as PartRow["values"][string]),
        attr: "fov_half_angle",
        value: {
          type: "scalar",
          nominal: {
            value: deg,
            unit: "deg",
            si: (deg * Math.PI) / 180,
            dimension: "Angle",
            display: "",
          },
          min: null,
          max: null,
        },
      },
    },
  });

  it("視線と半角を持つ部品だけを、長さ 1 の向きと半角 [rad] にする。広すぎる視野は上限で描く", () => {
    const [f] = fovsOf({ ...fixture, parts: [withFov(10)] });
    expect(f?.dir).toEqual([0, 0, 1]);
    expect(f?.halfAngle).toBeCloseTo((10 * Math.PI) / 180);
    expect(fovsOf({ ...fixture, parts: [withFov(89)] })[0]?.halfAngle).toBe(FOV_DRAW_MAX);
    expect(fovsOf({ ...fixture, parts: [{ ...withFov(10), body: null }] })).toEqual([]);
    expect(fovsOf(fixture)).toEqual([]);
  });
});

describe("枠線だけの表示", () => {
  const box = { color: "#000000", structure: true } as Parameters<typeof boxMaterial>[0];
  it("面をほぼ透明にし、選んだ部品だけ少し濃くする（構造の不透明度は使わない）", () => {
    expect(boxMaterial(box, false, 1, false, true)).toMatchObject({
      opacity: WIREFRAME_OPACITY,
      depthWrite: false,
    });
    expect(boxMaterial(box, false, 1, true, true).opacity).toBe(WIREFRAME_OPACITY);
    expect(boxMaterial(box, true, 1, false, true).opacity).toBe(WIREFRAME_SELECTED_OPACITY);
  });
});
