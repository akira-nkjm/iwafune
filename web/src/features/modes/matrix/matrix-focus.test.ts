import { describe, expect, it, vi } from "vitest";
import { arrowTarget, cellFocus } from "./matrix-focus";

/** 押したキーと、既定の動きを止めたかを見る偽のイベント */
const key = (name: string) => ({ key: name, preventDefault: vi.fn() });

describe("arrowTarget", () => {
  it("矢印キーで上下左右の隣の位置、ほかのキーは null", () => {
    const at = { row: 2, column: 3 };
    expect(arrowTarget("ArrowUp", at)).toEqual({ row: 1, column: 3 });
    expect(arrowTarget("ArrowDown", at)).toEqual({ row: 3, column: 3 });
    expect(arrowTarget("ArrowLeft", at)).toEqual({ row: 2, column: 2 });
    expect(arrowTarget("ArrowRight", at)).toEqual({ row: 2, column: 4 });
    expect(arrowTarget("d", at)).toBeNull();
  });
});

describe("cellFocus", () => {
  it("矢印キーで隣のセルにフォーカスを動かし、スクロールを止める", () => {
    const focus = cellFocus();
    const right = { focus: vi.fn() };
    const below = { focus: vi.fn() };
    focus.register({ row: 0, column: 1 }, right);
    focus.register({ row: 1, column: 0 }, below);

    const event = key("ArrowRight");
    expect(focus.move(event, { row: 0, column: 0 })).toBe(true);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(right.focus).toHaveBeenCalledTimes(1);

    focus.move(key("ArrowDown"), { row: 0, column: 0 });
    expect(below.focus).toHaveBeenCalledTimes(1);
  });

  it("端では動かないが矢印キーは使ったことにする。ほかのキーはセルに任せる", () => {
    const focus = cellFocus();
    const edge = key("ArrowUp");
    expect(focus.move(edge, { row: 0, column: 0 })).toBe(true);
    expect(edge.preventDefault).toHaveBeenCalled();

    const other = key("D");
    expect(focus.move(other, { row: 0, column: 0 })).toBe(false);
    expect(other.preventDefault).not.toHaveBeenCalled();
  });

  it("忘れたセルには動かない。同じ位置に入った別のセルは、前のセルを忘れても残す", () => {
    const focus = cellFocus();
    const old = { focus: vi.fn() };
    const next = { focus: vi.fn() };
    const forgetOld = focus.register({ row: 0, column: 1 }, old);
    focus.register({ row: 0, column: 1 }, next);
    forgetOld();
    focus.move(key("ArrowRight"), { row: 0, column: 0 });
    expect(next.focus).toHaveBeenCalledTimes(1);
    expect(old.focus).not.toHaveBeenCalled();

    const gone = { focus: vi.fn() };
    focus.register({ row: 1, column: 1 }, gone)();
    focus.move(key("ArrowDown"), { row: 0, column: 1 });
    expect(gone.focus).not.toHaveBeenCalled();
  });
});
