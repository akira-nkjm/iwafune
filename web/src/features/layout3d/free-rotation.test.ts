import { type Quaternion, Vector3 } from "three";
import { expect, it } from "vitest";
import { radiansPerPixel, rotateBall } from "./free-rotation";
import { lookQuaternion } from "./view-cube";

const TARGET = new Vector3(0.5, -0.25, 1);
const DISTANCE = 4;

/** 注視点から `direction`（単位ベクトル）の向きに距離 4 m 離れ、注視点を見るカメラ（上は +Z） */
function cameraAt(direction: [number, number, number]) {
  return {
    position: TARGET.clone().addScaledVector(new Vector3(...direction), DISTANCE),
    quaternion: lookQuaternion(direction, [0, 0, 1]),
  };
}

const up = (q: Quaternion) => new Vector3(0, 1, 0).applyQuaternion(q);
const back = (q: Quaternion) => new Vector3(0, 0, 1).applyQuaternion(q);
const PER_PIXEL = radiansPerPixel(600);

/** 注視点との距離が保たれ、姿勢が正規化され、注視点を見ている */
function expectHealthy(c: ReturnType<typeof cameraAt>) {
  expect(c.position.distanceTo(TARGET)).toBeCloseTo(DISTANCE, 9);
  expect(c.quaternion.length()).toBeCloseTo(1, 12);
  const look = new Vector3(0, 0, -1).applyQuaternion(c.quaternion);
  expect(look.distanceTo(TARGET.clone().sub(c.position).normalize())).toBeLessThan(1e-9);
}

it("描画領域の高さ 1 つ分のドラッグでちょうど 1 回転する", () => {
  expect(radiansPerPixel(600) * 600).toBeCloseTo(2 * Math.PI, 12);
});

it("画面の縦（X のまわり）へ 1 回転ぶんドラッグすると、元の位置と向きへ戻る。途中で極を越えて逆さまになる", () => {
  const c = cameraAt([Math.SQRT1_2, -Math.SQRT1_2, 0]);
  const q0 = c.quaternion.clone();
  const p0 = c.position.clone();
  let upsideDown = false;
  let belowTarget = false;
  // 600 px を 1 px ずつ
  for (let i = 0; i < 600; i++) {
    rotateBall(c.position, c.quaternion, TARGET, 0, 1, PER_PIXEL);
    if (up(c.quaternion).z < -0.5) upsideDown = true;
    if (c.position.z < TARGET.z - 3) belowTarget = true;
  }
  expect(upsideDown).toBe(true);
  expect(belowTarget).toBe(true);
  // 600 回の合成で、ずれは 1e-9 未満（丸め誤差が溜まらない）
  expect(c.position.distanceTo(p0)).toBeLessThan(1e-9);
  expect(Math.abs(c.quaternion.dot(q0))).toBeGreaterThan(1 - 1e-12);
  expectHealthy(c);
});

it("横（画面の上の軸のまわり）にも、斜めにも 1 回転でき、戻る", () => {
  for (const [dx, dy] of [
    [1, 0],
    [0.6, 0.8],
    [-0.8, 0.6],
  ] as const) {
    const c = cameraAt(
      [0.3, 0.9, 0.2].map((v) => v / Math.hypot(0.3, 0.9, 0.2)) as [number, number, number],
    );
    const p0 = c.position.clone();
    const q0 = c.quaternion.clone();
    for (let i = 0; i < 600; i++) rotateBall(c.position, c.quaternion, TARGET, dx, dy, PER_PIXEL);
    expect(c.position.distanceTo(p0)).toBeLessThan(1e-9);
    expect(Math.abs(c.quaternion.dot(q0))).toBeGreaterThan(1 - 1e-12);
  }
});

it("ドラッグの向き: 右へ引くと機体の手前が右へ（カメラは左へ回る）、下へ引くと手前が下へ（上面が見えてくる）", () => {
  const a = cameraAt([0, -1, 0]);
  const right0 = new Vector3(1, 0, 0).applyQuaternion(a.quaternion);
  rotateBall(a.position, a.quaternion, TARGET, 50, 0, PER_PIXEL);
  // カメラの位置は、画面の右の逆（左）へ動く
  expect(a.position.clone().sub(TARGET).dot(right0)).toBeLessThan(0);
  const b = cameraAt([0, -1, 0]);
  rotateBall(b.position, b.quaternion, TARGET, 0, 50, PER_PIXEL);
  // カメラは上へ動く
  expect(b.position.z).toBeGreaterThan(TARGET.z);
});

it("極を越えても続けて回せる: 真上（PZ）から先へ回した向きが、極で止まらない", () => {
  const c = cameraAt([0, 0, 1]);
  c.quaternion.copy(lookQuaternion([0, 0, 1], [0, 1, 0]));
  for (let i = 0; i < 100; i++) rotateBall(c.position, c.quaternion, TARGET, 0, 1, PER_PIXEL);
  // 100 px = 60° 回った（極に張り付かない）
  const angle = Math.acos(back(c.quaternion).z);
  expect(angle).toBeCloseTo(100 * PER_PIXEL, 9);
  expectHealthy(c);
});

it("動かさなければ何も変わらない", () => {
  const c = cameraAt([1, 0, 0]);
  const p = c.position.clone();
  rotateBall(c.position, c.quaternion, TARGET, 0, 0, PER_PIXEL);
  expect(c.position.equals(p)).toBe(true);
});
