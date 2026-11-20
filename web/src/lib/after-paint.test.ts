import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { afterPaint } from "./after-paint";

describe("afterPaint（描画のあとで呼ぶ）", () => {
  let frames: (() => void)[] = [];
  beforeEach(() => {
    vi.useFakeTimers();
    frames = [];
    vi.stubGlobal("requestAnimationFrame", (f: () => void) => frames.push(f));
    vi.stubGlobal("cancelAnimationFrame", (id: number) => {
      frames[id - 1] = () => undefined;
    });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });
  const frame = () => {
    for (const f of frames.splice(0)) f();
  };

  it("次のフレームのあと（タイマーを 1 つはさんで）に 1 回だけ呼ぶ", () => {
    const run = vi.fn();
    afterPaint(run);
    expect(run).not.toHaveBeenCalled();
    frame();
    expect(run).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("取り消したら呼ばない（フレームの前でも、フレームのあとでも）", () => {
    const before = vi.fn();
    afterPaint(before)();
    frame();
    vi.runAllTimers();
    expect(before).not.toHaveBeenCalled();
    const after = vi.fn();
    const cancel = afterPaint(after);
    frame();
    cancel();
    vi.runAllTimers();
    expect(after).not.toHaveBeenCalled();
  });
});
