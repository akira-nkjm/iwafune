import { Matrix4, Quaternion, Vector3 } from "three";
import type { V3 } from "./types";

/**
 * ビューキューブの 26 の向きの表（単一の置き場）と、押したときのカメラの姿勢の計算。three の数学だけで書き、DOM と描画には触れない。
 *
 * 向き `direction` は「注視点からカメラへ向かう」単位ベクトル（面 = その軸、辺 = 2 つの面の法線の和、角 = 3 つの面の法線の和を正規化）。
 * 上の向き `up` の約束（押したあとは傾きのない、まっすぐな画面になる）:
 * - 側面（PX・MX・PY・MY）とその辺（Z を含まない辺）、および Z を含む辺・PZ 側の角は、+Z を視線に垂直な面へ射影した向きを上にする（水平線が水平になる）
 * - MZ 側の角は −Z を射影した向きを上にする。角から出る 3 本の辺が Y 字（Z の辺が画面の下へ伸び、角の Z の面が上）に見える
 * - 角を押したときは、Y 字に見える 3 つの向き（表の上の向きと、視線まわりに ±120° 回した向き）のうち、今の向きにいちばん近いものへ動く
 *   （`poseForView`。Z が上とは限らない。動きを小さくする）
 * - PZ（真上から見下ろす）と MZ（真下から見上げる）は +Z が視線と平行で射影できないので、どちらも +Y を上にする
 *   （PZ の面から PY 側の辺へ傾けた向きと、MZ の面から MY 側の辺へ傾けた向きに連続する）
 */

/** 面の名前（機体座標の ± の軸。P = プラス側、M = マイナス側） */
export const FACE_LABELS = { px: "PX", mx: "MX", py: "PY", my: "MY", pz: "PZ", mz: "MZ" } as const;

/** 区画の種類 */
export type CubeKind = "face" | "edge" | "corner";

/** ビューキューブの区画 1 つの向き */
export interface CubeView {
  /** 区画の名前（面は `PX`、辺は `PX+PY`、角は `PX+PY+PZ`。軸の順は X・Y・Z） */
  name: string;
  kind: CubeKind;
  /** 立方体の上の位置を表す符号（各 -1・0・1。0 でない軸の数が 1 = 面、2 = 辺、3 = 角） */
  sign: V3;
  /** 注視点からカメラへ向かう単位ベクトル */
  direction: V3;
  /** 画面の上にする向き（単位ベクトル。`direction` と垂直） */
  up: V3;
}

const KINDS: CubeKind[] = ["face", "edge", "corner"];
const AXIS_NAMES = [
  ["MX", "PX"],
  ["MY", "PY"],
  ["MZ", "PZ"],
] as const;

function unit(v: Vector3): V3 {
  v.normalize();
  return [v.x, v.y, v.z];
}

/**
 * Z 軸（`zSign` = 1 なら +Z、−1 なら −Z）を視線に垂直な面へ射影した単位ベクトル。視線が Z と平行（PZ・MZ）なら +Y
 */
function upFor(direction: Vector3, zSign: 1 | -1): V3 {
  const z = new Vector3(0, 0, zSign);
  const up = z.addScaledVector(direction, -direction.z * zSign);
  if (up.lengthSq() < 1e-12) return [0, 1, 0];
  return unit(up);
}

function makeView(sign: V3): CubeView {
  const count = sign.filter((s) => s !== 0).length;
  const direction = new Vector3(...sign).normalize();
  const name = sign
    .flatMap((s, i) => (s === 0 ? [] : [AXIS_NAMES[i]?.[s > 0 ? 1 : 0] ?? ""]))
    .join("+");
  return {
    name,
    kind: KINDS[count - 1] ?? "face",
    sign,
    direction: [direction.x, direction.y, direction.z],
    // 下側（MZ 側）の角だけ −Z を上にする（Y 字に見せる）
    up: upFor(direction, count === 3 && sign[2] < 0 ? -1 : 1),
  };
}

const SIGNS = [-1, 0, 1] as const;

/** 26 の向き（面 6・辺 12・角 8）。符号の組を X・Y・Z の順に並べた表 */
export const CUBE_VIEWS: readonly CubeView[] = SIGNS.flatMap((x) =>
  SIGNS.flatMap((y) => SIGNS.map((z): V3 => [x, y, z])),
)
  .filter((s) => s.some((c) => c !== 0))
  .map(makeView);

/** 符号の組から区画を引く（0 でない成分の符号だけを見る。無ければ undefined） */
export function viewBySign(sign: readonly number[]): CubeView | undefined {
  const s = sign.map((c) => Math.sign(c));
  return CUBE_VIEWS.find((v) => v.sign.every((c, i) => c === s[i]));
}

/** 注視点とカメラの位置関係（カメラは注視点から `distance` だけ、向き `quaternion` の後ろへ引いた所にある） */
export interface CameraPose {
  target: Vector3;
  /** カメラの姿勢（three の既定: 視線は −Z、上は +Y） */
  quaternion: Quaternion;
  distance: number;
}

/** 注視点からカメラへの向きと上の向きから、カメラの姿勢（注視点を見る向き）を作る */
export function lookQuaternion(direction: V3, up: V3): Quaternion {
  const m = new Matrix4().lookAt(new Vector3(...direction), new Vector3(), new Vector3(...up));
  return new Quaternion().setFromRotationMatrix(m);
}

/** 角から見て Y 字のままになる、視線まわりの回転の角 [rad]（辺が 3 本なので 120° ごと） */
const CORNER_TURN = (2 * Math.PI) / 3;

/**
 * 区画を押したときの到達の姿勢: 注視点と距離はそのまま、表の向きから注視点を見る。
 * 面と辺は表の上の向き。角は、Y 字に見える 3 つの向きのうち今の姿勢からの回転がいちばん小さいもの（同じなら表の上の向き）。
 */
export function poseForView(view: CubeView, from: CameraPose): CameraPose {
  const base = lookQuaternion(view.direction, view.up);
  let quaternion = base;
  if (view.kind === "corner") {
    const roll = new Vector3(0, 0, 1);
    for (const k of [1, -1]) {
      const q = base.clone().multiply(new Quaternion().setFromAxisAngle(roll, k * CORNER_TURN));
      // 差が角の計算の分解能（1e-8 程度）より大きいときだけ乗り換える
      if (q.angleTo(from.quaternion) < quaternion.angleTo(from.quaternion) - 1e-6) quaternion = q;
    }
  }
  return { target: from.target.clone(), quaternion, distance: from.distance };
}

/**
 * 今見ている向き（視線）を軸に画面を回した姿勢: 注視点・距離・視線はそのまま、画面の中の絵が時計回りに `angle` [rad] 回る
 * （負なら反時計回り）。カメラ自身は視線のまわりを逆向きに回る。
 */
export function rollPose(from: CameraPose, angle: number): CameraPose {
  const turn = new Quaternion().setFromAxisAngle(new Vector3(0, 0, 1), angle);
  return {
    target: from.target.clone(),
    quaternion: from.quaternion.clone().multiply(turn).normalize(),
    distance: from.distance,
  };
}

/** 視線が立方体の角の向き（3 成分の大きさが等しい）とみなす許容 */
const CORNER_TOLERANCE = 1e-3;

/**
 * 視線まわりに 1 回で回す角 [rad]。角から見ているとき（視線が立方体の対角線）は 120°: 回しても Y 字のまま、次の面が上に来る
 * （60° だと上下が逆の形になる）。それ以外は 90°（面から見ているときは、面の辺が画面にまっすぐなまま）。
 */
export function rollStep(quaternion: Quaternion): number {
  const d = new Vector3(0, 0, 1).applyQuaternion(quaternion);
  const [x, y, z] = [Math.abs(d.x), Math.abs(d.y), Math.abs(d.z)];
  const corner = Math.abs(x - y) < CORNER_TOLERANCE && Math.abs(y - z) < CORNER_TOLERANCE;
  return corner ? CORNER_TURN : Math.PI / 2;
}

/** 姿勢からカメラの位置（注視点 + 後ろ向きの単位ベクトル × 距離） */
export function positionOf(pose: CameraPose): Vector3 {
  return new Vector3(0, 0, 1)
    .applyQuaternion(pose.quaternion)
    .multiplyScalar(pose.distance)
    .add(pose.target);
}

/** 姿勢の補間（注視点と距離は線形、向きは球面補間。カメラは注視点のまわりの弧を通る） */
export function lerpPose(a: CameraPose, b: CameraPose, k: number): CameraPose {
  return {
    target: a.target.clone().lerp(b.target, k),
    quaternion: a.quaternion.clone().slerp(b.quaternion, k),
    distance: a.distance + (b.distance - a.distance) * k,
  };
}

/** 動かす時間 [s]（`Bounds` の全体を収める動きと同じ） */
export const MOVE_SECONDS = 1;

/** 動きの緩急（`Bounds` の既定と同じ。t は 0〜1 の経過、返りは 0〜1 の進み） */
export function easeMove(t: number): number {
  if (t >= 1) return 1;
  return 1 - Math.exp(-5 * t) + 0.007 * t;
}
