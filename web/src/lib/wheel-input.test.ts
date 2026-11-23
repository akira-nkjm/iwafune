import { expect, it } from "vitest";
import {
  clampZoom,
  classifyWheel,
  createWheelClassifier,
  NOTCH_ZOOM_STEP,
  SCROLL_HOLD_MS,
  type WheelLike,
  wheelPixels,
  zoomFactor,
} from "./wheel-input";

const ev = (o: Partial<WheelLike>): WheelLike => ({
  ctrlKey: false,
  metaKey: false,
  deltaMode: 0,
  deltaX: 0,
  deltaY: 0,
  ...o,
});

it("マウスの 1 目盛り（ピクセル単位 100・行単位 3）は修飾キー無しで notch", () => {
  expect(classifyWheel(ev({ deltaY: 100 }))).toBe("notch");
  expect(classifyWheel(ev({ deltaY: -120 }))).toBe("notch");
  expect(classifyWheel(ev({ deltaMode: 1, deltaY: 3 }))).toBe("notch");
});

it("トラックパッドの 2 本指スクロール（小さな量・斜め）は scroll", () => {
  expect(classifyWheel(ev({ deltaY: 3.5 }))).toBe("scroll");
  expect(classifyWheel(ev({ deltaX: 12, deltaY: -7 }))).toBe("scroll");
  // 斜めなら量が大きくても目盛りではない
  expect(classifyWheel(ev({ deltaX: 5, deltaY: 80 }))).toBe("scroll");
});

it("ピンチ（Ctrl / ⌘ 付きの小さな量）は pinch", () => {
  expect(classifyWheel(ev({ ctrlKey: true, deltaY: -4.2 }))).toBe("pinch");
  expect(classifyWheel(ev({ metaKey: true, deltaY: 2 }))).toBe("pinch");
});

it("Ctrl + マウスホイールは notch（ピンチではない）", () => {
  expect(classifyWheel(ev({ ctrlKey: true, deltaY: -100 }))).toBe("notch");
  expect(classifyWheel(ev({ metaKey: true, deltaMode: 1, deltaY: -3 }))).toBe("notch");
});

it("速く払ったトラックパッド（直前にスクロールがあった大きな縦の量）は scroll のまま", () => {
  const classify = createWheelClassifier();
  expect(classify(ev({ deltaY: 8 }), 1000)).toBe("scroll");
  expect(classify(ev({ deltaY: 90 }), 1000 + SCROLL_HOLD_MS - 1)).toBe("scroll");
  // 間が空けばマウスの目盛り。行単位・修飾キー付きは直前に関わらず目盛り
  expect(classify(ev({ deltaY: 100 }), 5000)).toBe("notch");
  expect(classify(ev({ deltaY: 4 }), 5010)).toBe("scroll");
  expect(classify(ev({ deltaMode: 1, deltaY: 3 }), 5020)).toBe("notch");
  expect(classify(ev({ ctrlKey: true, deltaY: 100 }), 5030)).toBe("notch");
});

it("拡大率: 目盛りは一定の段（拡大と縮小で逆数）、ピンチは量に比例、範囲に収める", () => {
  expect(zoomFactor("notch", -100)).toBe(NOTCH_ZOOM_STEP);
  expect(zoomFactor("notch", 100)).toBeCloseTo(1 / NOTCH_ZOOM_STEP, 12);
  expect(zoomFactor("pinch", -10)).toBeCloseTo(Math.exp(0.1), 12);
  expect(zoomFactor("pinch", 0)).toBe(1);
  expect(zoomFactor("pinch", 5) * zoomFactor("pinch", -5)).toBeCloseTo(1, 12);
  expect(clampZoom(20, 0.1, 10)).toBe(10);
  expect(clampZoom(0.01, 0.1, 10)).toBe(0.1);
  expect(clampZoom(2, 0.1, 10)).toBe(2);
});

it("量をピクセルにそろえる（行は 16 px、ページは渡した高さ）", () => {
  expect(wheelPixels({ deltaMode: 0, deltaX: 2, deltaY: 3 }, 800)).toEqual({ x: 2, y: 3 });
  expect(wheelPixels({ deltaMode: 1, deltaX: 0, deltaY: 3 }, 800)).toEqual({ x: 0, y: 48 });
  expect(wheelPixels({ deltaMode: 2, deltaX: 0, deltaY: 1 }, 800)).toEqual({ x: 0, y: 800 });
});
