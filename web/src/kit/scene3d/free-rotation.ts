import { Quaternion, Vector3 } from "three";

/**
 * 3D の視点を、極のない（上の向きを固定しない）トラックボールのように回す計算。three の数学だけで書き、DOM に触れない。
 * ドラッグした向きに機体が転がる: 画面の右へ dx、下へ dy [px] ドラッグすると、機体は「カメラの上の軸 × dx + カメラの右の軸 × dy」を軸に、
 * ドラッグの長さに比例した角だけ回る。カメラは機体と逆に、注視点のまわりを回る（位置と姿勢を同じ回転で動かすので、注視点を見る向きは保たれる）。
 */

/** 1 px のドラッグで回す角 [rad]。描画領域の高さ 1 つ分のドラッグでちょうど 1 回転する */
export function radiansPerPixel(heightPx: number): number {
  return (2 * Math.PI) / Math.max(1, heightPx);
}

/**
 * カメラを注視点のまわりに回す。`position`・`quaternion` を書き換える（回したあと姿勢は正規化し直し、丸め誤差が溜まらないようにする）。
 * 大きな回転でも軸は毎回カメラの今の軸から決めるので、どの軸まわりにも回り続けられる（極で止まらない）。
 */
export function rotateBall(
  position: Vector3,
  quaternion: Quaternion,
  target: Vector3,
  dx: number,
  dy: number,
  perPixel: number,
): void {
  const length = Math.hypot(dx, dy);
  if (length === 0) return;
  const right = new Vector3(1, 0, 0).applyQuaternion(quaternion);
  const up = new Vector3(0, 1, 0).applyQuaternion(quaternion);
  const axis = up.multiplyScalar(dx).addScaledVector(right, dy).normalize();
  // 機体を回す回転の逆をカメラに掛ける
  const turn = new Quaternion().setFromAxisAngle(axis, -length * perPixel);
  position.sub(target).applyQuaternion(turn).add(target);
  quaternion.premultiply(turn).normalize();
}
