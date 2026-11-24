import { Quaternion, Vector3 } from "three";
import { expect, it } from "vitest";
import {
  type CameraPose,
  CUBE_VIEWS,
  easeMove,
  lerpPose,
  lookQuaternion,
  poseForView,
  positionOf,
  viewBySign,
} from "./view-cube";

const vec = (v: readonly number[]) => new Vector3(v[0], v[1], v[2]);
const byName = (name: string) => {
  const v = CUBE_VIEWS.find((c) => c.name === name);
  if (!v) throw new Error(`no view ${name}`);
  return v;
};

it("26 の区画: 面 6・辺 12・角 8 で、名前も向きも重ならない", () => {
  expect(CUBE_VIEWS).toHaveLength(26);
  const count = (k: string) => CUBE_VIEWS.filter((v) => v.kind === k).length;
  expect([count("face"), count("edge"), count("corner")]).toEqual([6, 12, 8]);
  expect(new Set(CUBE_VIEWS.map((v) => v.name)).size).toBe(26);
  for (const a of CUBE_VIEWS)
    for (const b of CUBE_VIEWS)
      if (a !== b) expect(vec(a.direction).distanceTo(vec(b.direction))).toBeGreaterThan(0.3);
});

it("面は 6 つの軸、辺・角は面の法線の和を正規化した向き", () => {
  const axes: Record<string, number[]> = {
    PX: [1, 0, 0],
    MX: [-1, 0, 0],
    PY: [0, 1, 0],
    MY: [0, -1, 0],
    PZ: [0, 0, 1],
    MZ: [0, 0, -1],
  };
  for (const [name, axis] of Object.entries(axes)) {
    expect(byName(name).kind).toBe("face");
    expect(byName(name).direction).toEqual(axis);
  }
  for (const v of CUBE_VIEWS) {
    expect(vec(v.direction).length()).toBeCloseTo(1, 12);
    // 名前の面の法線を足して正規化したものと一致する
    const sum = new Vector3();
    for (const face of v.name.split("+")) sum.add(vec(axes[face] ?? [0, 0, 0]));
    expect(vec(v.direction).distanceTo(sum.normalize())).toBeLessThan(1e-12);
  }
  // 辺 PX+PY は (1,1,0)/√2、角 PX+PY+PZ は (1,1,1)/√3
  expect(byName("PX+PY").direction[0]).toBeCloseTo(Math.SQRT1_2, 12);
  expect(byName("PX+PY+PZ").direction[2]).toBeCloseTo(1 / Math.sqrt(3), 12);
});

it("上の向き: 単位ベクトルで視線と垂直。PZ・MZ は +Y、それ以外は +Z を射影した向き（水平線が水平）", () => {
  for (const v of CUBE_VIEWS) {
    const d = vec(v.direction);
    const up = vec(v.up);
    expect(up.length()).toBeCloseTo(1, 12);
    expect(Math.abs(up.dot(d))).toBeLessThan(1e-12);
    const q = lookQuaternion(v.direction, v.up);
    const right = new Vector3(1, 0, 0).applyQuaternion(q);
    const screenUp = new Vector3(0, 1, 0).applyQuaternion(q);
    const back = new Vector3(0, 0, 1).applyQuaternion(q);
    // 姿勢は表の向きと上を、そのとおりに画面の奥行き・上へ写す
    expect(back.distanceTo(d)).toBeLessThan(1e-12);
    expect(screenUp.distanceTo(up)).toBeLessThan(1e-12);
    if (v.name === "PZ" || v.name === "MZ") {
      expect(v.up).toEqual([0, 1, 0]);
    } else {
      // 傾きがない: 画面の右が水平（Z 成分が 0）で、上が +Z 側を向く
      expect(Math.abs(right.z)).toBeLessThan(1e-12);
      expect(up.z).toBeGreaterThan(0);
    }
  }
  // 側面の上は +Z そのもの
  expect(byName("PX").up).toEqual([0, 0, 1]);
  expect(byName("PX+MY").up).toEqual([0, 0, 1]);
});

it("符号の組から区画を引く（0 でない成分の符号だけを見る）", () => {
  expect(viewBySign([0.38, 0.38, 0])?.name).toBe("PX+PY");
  expect(viewBySign([-1, 0, 0])?.name).toBe("MX");
  expect(viewBySign([0, 0, 0])).toBeUndefined();
});

it("押したときの姿勢: 注視点と距離はそのまま、位置 = 注視点 + 向き × 距離、注視点を見て、上は表のとおり", () => {
  const from: CameraPose = {
    target: new Vector3(1, -2, 0.5),
    quaternion: new Quaternion().setFromAxisAngle(new Vector3(1, 1, 0).normalize(), 2.5),
    distance: 3.7,
  };
  for (const v of CUBE_VIEWS) {
    const pose = poseForView(v, from);
    expect(pose.target.equals(from.target)).toBe(true);
    expect(pose.distance).toBe(3.7);
    const pos = positionOf(pose);
    const expected = from.target.clone().addScaledVector(vec(v.direction), 3.7);
    expect(pos.distanceTo(expected)).toBeLessThan(1e-12);
    // 視線（−Z）が注視点を向く
    const look = new Vector3(0, 0, -1).applyQuaternion(pose.quaternion);
    expect(look.distanceTo(from.target.clone().sub(pos).normalize())).toBeLessThan(1e-12);
    const up = new Vector3(0, 1, 0).applyQuaternion(pose.quaternion);
    expect(up.distanceTo(vec(v.up))).toBeLessThan(1e-12);
  }
});

it("補間: 端はそのまま、途中は注視点のまわりの弧（距離一定）を通る。反対側へ回っても注視点を通り抜けない", () => {
  const target = new Vector3(0.2, 0.1, -0.3);
  const a = poseForView(byName("PX"), { target, quaternion: new Quaternion(), distance: 2 });
  const b = poseForView(byName("MX"), a);
  expect(positionOf(lerpPose(a, b, 0)).distanceTo(positionOf(a))).toBeLessThan(1e-12);
  expect(positionOf(lerpPose(a, b, 1)).distanceTo(positionOf(b))).toBeLessThan(1e-12);
  for (let k = 0; k <= 10; k++) {
    const mid = lerpPose(a, b, k / 10);
    expect(positionOf(mid).distanceTo(target)).toBeCloseTo(2, 9);
    expect(mid.quaternion.length()).toBeCloseTo(1, 12);
  }
});

it("緩急: 0 で 0、1 で 1、単調に増える", () => {
  expect(easeMove(0)).toBe(0);
  expect(easeMove(1)).toBe(1);
  expect(easeMove(2)).toBe(1);
  let prev = 0;
  for (let i = 1; i < 100; i++) {
    const v = easeMove(i / 100);
    expect(v).toBeGreaterThan(prev);
    prev = v;
  }
});
