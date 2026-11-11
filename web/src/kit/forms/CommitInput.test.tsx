import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { byLabel, fire } from "@/test/elements";
import { CommitInput } from "./CommitInput";

/** 欄（TextInput の要素）と、確定の呼ばれ方を見るための偽物 */
function setup(props: { value?: string; allowEmpty?: boolean } = {}) {
  const onCommit = vi.fn();
  const element = byLabel(
    CommitInput({
      value: props.value ?? "a",
      label: "名前",
      onCommit,
      allowEmpty: props.allowEmpty,
    }),
    "名前",
  );
  return { element, onCommit };
}

/** 打った文字列を持つ欄（blur・focus は呼ばれたかを見る） */
function target(value: string) {
  return { value, blur: vi.fn() };
}

describe("CommitInput", () => {
  it("いまの値を出し、aria-label を付ける", () => {
    const html = renderToStaticMarkup(<CommitInput value="RW_X" label="ID" onCommit={() => {}} />);
    expect(html).toContain('aria-label="ID"');
    expect(html).toContain('value="RW_X"');
  });

  it("欄を離れると、前後の空白を除いた文字列で確定する", () => {
    const { element, onCommit } = setup();
    fire(element, "onBlur", { currentTarget: target("  b  ") });
    expect(onCommit).toHaveBeenCalledWith("b");
  });

  it("値が変わっていなければ確定しない", () => {
    const { element, onCommit } = setup();
    fire(element, "onBlur", { currentTarget: target("a ") });
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("空にして離れると空で確定する（既定）。allowEmpty=false なら元の値に戻す", () => {
    const empty = setup();
    fire(empty.element, "onBlur", { currentTarget: target("") });
    expect(empty.onCommit).toHaveBeenCalledWith("");

    const kept = setup({ value: "2", allowEmpty: false });
    const input = target("  ");
    fire(kept.element, "onBlur", { currentTarget: input });
    expect(kept.onCommit).not.toHaveBeenCalled();
    expect(input.value).toBe("2");
  });

  it("Enter は欄を離れて確定させ、Escape は打った文字を捨てて離れる", () => {
    const { element } = setup();
    const enter = target("b");
    const preventDefault = vi.fn();
    fire(element, "onKeyDown", { key: "Enter", currentTarget: enter, preventDefault });
    expect(enter.blur).toHaveBeenCalled();
    expect(preventDefault).toHaveBeenCalled();

    const cancel = target("b");
    fire(element, "onKeyDown", { key: "Escape", currentTarget: cancel, preventDefault });
    expect(cancel.value).toBe("a");
    expect(cancel.blur).toHaveBeenCalled();

    const other = target("b");
    fire(element, "onKeyDown", { key: "x", currentTarget: other, preventDefault });
    expect(other.blur).not.toHaveBeenCalled();
  });
});
