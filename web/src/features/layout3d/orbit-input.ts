import type { Vector3 } from "three";

/**
 * 3D の視点操作の計算（OrbitControls の外側で動かす、ホイールの移動と拡大）。three の Vector3 だけで書き、DOM に触れない。
 * ホイールの読み方は lib/wheel-input.ts。
 */

/** 画面の 1 px が、注視点の奥行きで世界の何 m にあたるか（透視投影。fov は縦の画角 [度]、height は描画領域の高さ [px]） */
export function worldPerPixel(distance: number, fovDeg: number, heightPx: number): number {
  return (2 * distance * Math.tan((fovDeg * Math.PI) / 360)) / heightPx;
}

/**
 * 2 本指のスクロール（ピクセル量 dx, dy）でカメラと注視点を一緒に動かす量。内容が指に付いてくる向き
 * （OS が送る向きのまま。dy > 0 で内容は上へ = カメラは下へ、dx > 0 で内容は左へ = カメラは右へ）。
 * right / up はカメラの右・上の単位ベクトル。
 */
export function panOffset(
  dx: number,
  dy: number,
  perPixel: number,
  right: Vector3,
  up: Vector3,
): Vector3 {
  return right
    .clone()
    .multiplyScalar(dx * perPixel)
    .addScaledVector(up, -dy * perPixel);
}

/** 描画領域の中の位置（-1〜1。右・上が正）に対応する、注視点を通る画面に平行な面の上の点 */
export function cursorAnchor(
  target: Vector3,
  right: Vector3,
  up: Vector3,
  ndc: { x: number; y: number },
  halfSize: { w: number; h: number },
): Vector3 {
  return target
    .clone()
    .addScaledVector(right, ndc.x * halfSize.w)
    .addScaledVector(up, ndc.y * halfSize.h);
}

/** 視点との距離が [min, max] に収まるよう、拡大率（>1 で近づく）を絞る */
export function clampDollyFactor(
  distance: number,
  factor: number,
  range: { min: number; max: number },
): number {
  const next = Math.min(range.max, Math.max(range.min, distance / factor));
  return distance / next;
}

/**
 * anchor を画面上の同じ位置に保ったまま factor 倍に拡大する: カメラと注視点を anchor に向けて 1/factor に縮める
 * （向きは変わらず、距離だけ 1/factor になる）。結果は引数の位置・注視点を書き換えて返す。
 */
export function dollyAbout(
  position: Vector3,
  target: Vector3,
  anchor: Vector3,
  factor: number,
): void {
  position
    .sub(anchor)
    .multiplyScalar(1 / factor)
    .add(anchor);
  target
    .sub(anchor)
    .multiplyScalar(1 / factor)
    .add(anchor);
}
