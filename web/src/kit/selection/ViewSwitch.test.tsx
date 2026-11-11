import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { getT } from "@/i18n";
import { ViewSwitch } from "./ViewSwitch";

/** 選ばれている選択肢（`aria-checked="true"` のボタン）の文字 */
function checked(html: string): string | undefined {
  return html.match(/aria-checked="true"[^>]*>([^<]*)</)?.[1];
}

describe("ViewSwitch", () => {
  it("渡した値の表示方法を選んだ形で出す（URL を読まないので、ルーターの外でも描ける）", () => {
    const t = getT().common;
    const tree = renderToStaticMarkup(<ViewSwitch value="tree" onChange={() => {}} />);
    expect(tree).toContain(`aria-label="${t.view}"`);
    expect(checked(tree)).toBe(t.treeView);
    const table = renderToStaticMarkup(<ViewSwitch value="table" onChange={() => {}} />);
    expect(checked(table)).toBe(t.tableView);
  });

  it("ツリー以外の値は表として出す", () => {
    const html = renderToStaticMarkup(<ViewSwitch value="overview" onChange={() => {}} />);
    expect(checked(html)).toBe(getT().common.tableView);
  });
});
