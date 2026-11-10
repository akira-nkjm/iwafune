import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { fire } from "@/test/elements";
import { SelectableRow } from "./SelectableRow";

function row(selected: boolean) {
  return renderToStaticMarkup(
    <table>
      <tbody>
        <SelectableRow selected={selected} onSelect={() => {}}>
          <td>RW_X</td>
        </SelectableRow>
      </tbody>
    </table>,
  );
}

describe("SelectableRow", () => {
  it("行にフォーカスでき、選ばれているかを aria-selected と地の色で示す", () => {
    expect(row(true)).toContain('aria-selected="true"');
    expect(row(true)).toContain("bg-accent-tint");
    expect(row(true)).toContain('tabindex="0"');
    expect(row(false)).toContain('aria-selected="false"');
    expect(row(false)).not.toContain("bg-accent-tint hover:bg-accent-tint");
  });

  it("クリックで行にフォーカスを置いて onSelect", () => {
    const onSelect = vi.fn();
    const focus = vi.fn();
    const element = SelectableRow({ selected: false, onSelect, children: null });
    fire(element, "onClick", { currentTarget: { focus } });
    expect(focus).toHaveBeenCalled();
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("Enter と Space で選び、ほかのキーでは選ばない", () => {
    const onSelect = vi.fn();
    const element = SelectableRow({ selected: false, onSelect, children: null });
    const event = (key: string) => ({
      key,
      currentTarget: { focus: vi.fn() },
      preventDefault: vi.fn(),
    });
    const enter = event("Enter");
    fire(element, "onKeyDown", enter);
    fire(element, "onKeyDown", event(" "));
    fire(element, "onKeyDown", event("ArrowDown"));
    expect(onSelect).toHaveBeenCalledTimes(2);
    expect(enter.preventDefault).toHaveBeenCalled();
  });
});
