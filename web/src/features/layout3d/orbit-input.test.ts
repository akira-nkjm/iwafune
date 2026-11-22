import { PerspectiveCamera, Vector3 } from "three";
import { expect, it } from "vitest";
import {
  clampDollyFactor,
  cursorAnchor,
  dollyAbout,
  panOffset,
  worldPerPixel,
} from "./orbit-input";

const close = (a: Vector3, b: Vector3) => expect(a.distanceTo(b)).toBeLessThan(1e-9);

it("1 px あたりの世界の長さ: 画角 90°・距離 1 m・高さ 100 px なら、画面の高さが 2 m で 1 px = 0.02 m", () => {
  expect(worldPerPixel(1, 90, 100)).toBeCloseTo(0.02, 12);
});

it("スクロールの移動: 内容が指に付いてくる向き（dy > 0 でカメラは下、dx > 0 でカメラは右）", () => {
  const right = new Vector3(1, 0, 0);
  const up = new Vector3(0, 0, 1);
  close(panOffset(10, 0, 0.5, right, up), new Vector3(5, 0, 0));
  close(panOffset(0, 10, 0.5, right, up), new Vector3(0, 0, -5));
  close(panOffset(-4, -2, 1, right, up), new Vector3(-4, 0, 2));
});

it("カーソルの下の点: 中心は注視点、右上の隅は半分の大きさだけ右・上", () => {
  const t = new Vector3(1, 2, 3);
  const right = new Vector3(1, 0, 0);
  const up = new Vector3(0, 0, 1);
  const half = { w: 4, h: 2 };
  close(cursorAnchor(t, right, up, { x: 0, y: 0 }, half), t);
  close(cursorAnchor(t, right, up, { x: 1, y: 1 }, half), new Vector3(5, 2, 5));
});

it("距離の範囲に収める: 近づきすぎ・離れすぎを絞る", () => {
  const range = { min: 1, max: 100 };
  expect(clampDollyFactor(10, 2, range)).toBeCloseTo(2, 12);
  expect(clampDollyFactor(10, 100, range)).toBeCloseTo(10, 12);
  expect(clampDollyFactor(10, 0.001, range)).toBeCloseTo(0.1, 12);
});

it("カーソルを中心にした拡大: アンカーの画面上の位置が変わらず、距離が 1/factor になる", () => {
  const camera = new PerspectiveCamera(40, 2, 0.1, 100);
  const target = new Vector3(0, 0, 0);
  camera.position.set(6, -8, 5);
  camera.up.set(0, 0, 1);
  camera.lookAt(target);
  camera.updateMatrixWorld();
  const right = new Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
  const up = new Vector3().setFromMatrixColumn(camera.matrixWorld, 1);
  const d = camera.position.distanceTo(target);
  const half = { h: d * Math.tan((camera.fov * Math.PI) / 360), w: 0 };
  half.w = half.h * camera.aspect;
  const anchor = cursorAnchor(target, right, up, { x: 0.6, y: -0.3 }, half);
  const before = anchor.clone().project(camera);

  dollyAbout(camera.position, target, anchor, 2);
  camera.lookAt(target);
  camera.updateMatrixWorld();
  const after = anchor.clone().project(camera);

  expect(after.x).toBeCloseTo(before.x, 9);
  expect(after.y).toBeCloseTo(before.y, 9);
  expect(camera.position.distanceTo(target)).toBeCloseTo(d / 2, 9);
});
